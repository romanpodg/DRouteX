mod client;
mod inventory;
mod io;
mod live;
mod registry;
use crate::{
    DnsError, ErrorCode, Result,
    budget::Budget,
    cache::{Cache, Key, Template, UpstreamAnswer},
    clock::Refresh,
    wire::{Processor, Reply, response},
};
use droutex_config::{
    DnsAction,
    upstream::{Fallback, Member, Transport},
};
use droutex_core::deployment::AuthorizedRequest;
use droutex_policy::PolicySelection;
use hickory_proto::op::{DnsRequest, DnsRequestOptions, DnsResponse, Message, ResponseCode};
#[cfg(test)]
pub(super) use io::exact_io_limits_fixture;
#[cfg(test)]
pub(super) use io::inventory_checkpoint_fixture;
#[cfg(test)]
pub(super) use io::partial_write_checkpoint_fixture;
#[cfg(test)]
pub(super) use io::terminal_denial_fixture;
#[cfg(test)]
pub(super) use live::cache_transition_fixture;
#[cfg(test)]
pub(super) use live::shared_waiter_fixture;
use registry::Registry;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::time::Instant;

pub(super) use registry::Registry as CleanupRegistry;

pub(super) struct Processing<'request, 'store> {
    pub auth: &'request AuthorizedRequest<'store>,
    pub budget: &'request Arc<Budget>,
    pub refresh: &'request Arc<Refresh>,
    pub registry: &'request Arc<Registry>,
    pub cutoff: Instant,
    pub lost: Arc<AtomicBool>,
}

pub(crate) enum Outcome {
    Upstream(UpstreamAnswer),
    Failure(Message),
}

pub(super) struct Forwarder {
    aliases: Vec<std::net::IpAddr>,
    cache: Cache,
    #[cfg(test)]
    fixture: Option<Fixture>,
}
impl Forwarder {
    #[cfg(test)]
    pub fn fixture_cache_hit_barrier(&mut self, barrier: Arc<CacheHitBarrier>) {
        if let Some(fixture) = &mut self.fixture {
            fixture.cache_hit = Some(barrier);
        }
    }
    #[cfg(test)]
    pub fn fixture_cleanup_delay(&mut self, delay: std::time::Duration) {
        if let Some(fixture) = &mut self.fixture {
            for target in fixture.targets.values_mut().flatten() {
                target.cleanup_delay = delay;
            }
        }
    }
    #[cfg(test)]
    pub fn fixture_tc_delay(&mut self, delay: std::time::Duration) {
        if let Some(fixture) = &mut self.fixture {
            for target in fixture
                .targets
                .values_mut()
                .filter_map(|targets| targets.first_mut())
            {
                target.tc_delay = delay;
            }
        }
    }
    pub fn new(aliases: Vec<std::net::IpAddr>) -> Result<Self> {
        if aliases.len() > 256 {
            return Err(DnsError(ErrorCode::Configuration));
        }
        Ok(Self {
            aliases,
            cache: Cache::new(),
            #[cfg(test)]
            fixture: None,
        })
    }
}
impl Processor for Forwarder {
    fn forwarding_available(&self) -> bool {
        true
    }
    async fn answer(
        &self,
        request: &Message,
        selection: PolicySelection<'_, DnsAction>,
        context: &Processing<'_, '_>,
    ) -> Result<Reply> {
        match selection.action() {
            DnsAction::Block {} => {
                return Ok(response(request, ResponseCode::NXDomain).into());
            }
            DnsAction::Resolve { .. } => {
                if let Some(reply) = crate::policy::local_answer(
                    request,
                    selection.profile(),
                    context
                        .auth
                        .decision()
                        .compiled_configuration()
                        .private_records(),
                    context.budget,
                )? {
                    return Ok(reply.into());
                }
            }
            DnsAction::Gateway { .. } => {}
        }
        let DnsAction::Resolve { upstream } = selection.action() else {
            return Ok(response(request, ResponseCode::ServFail).into());
        };
        let live = Arc::new(live::Live::new(
            context
                .auth
                .dns_io_check()
                .map_err(|_| DnsError(ErrorCode::Admission))?,
            Arc::clone(context.refresh),
            context.cutoff,
            Arc::clone(&context.lost),
            None,
        ));
        live.current(context.cutoff).await?;
        let key = Key::admitted(context.auth, *upstream, request)?;
        if let Ok(_copy_credit) = context.budget.wire(4096)
            && let Some(template) = self.cache.get(&key, Instant::now())
            && let Some(reply) = template.reply(request, Instant::now())
        {
            #[cfg(test)]
            if let Some(barrier) = self
                .fixture
                .as_ref()
                .and_then(|fixture| fixture.cache_hit.as_ref())
            {
                // A private delay only: the real current check still follows it.
                barrier.entered.notify_one();
                barrier.release.notified().await;
            }
            live.current(context.cutoff).await?;
            if reply.expires.is_some_and(|expiry| Instant::now() < expiry) {
                return Ok(reply);
            }
        }
        let group = context
            .auth
            .decision()
            .compiled_configuration()
            .upstream(*upstream)
            .ok_or(DnsError(ErrorCode::Scope))?;
        // At most two immutable validated members must outlive the processor
        // borrow. The live checker still requires its original generation/owner.
        let members: Vec<_> = group.members().iter().copied().map(Target::from).collect();
        let fallback = group.fallback();
        let inventory = Arc::new(inventory::Inventory::new(
            &self.aliases,
            context
                .auth
                .decision()
                .dns_configuration()
                .document()
                .listeners
                .iter()
                .map(|listener| listener.bind.ip()),
        )?);
        let template = request.clone();
        #[cfg(test)]
        let (members, inventory) = if let Some(fixture) = &self.fixture {
            let targets = fixture
                .targets
                .get(upstream)
                .ok_or(DnsError(ErrorCode::Scope))?;
            if targets.len() != members.len()
                || targets
                    .iter()
                    .zip(&members)
                    .any(|(target, actual)| target.transport != actual.transport)
            {
                return Err(DnsError(ErrorCode::Scope));
            }
            (targets.clone(), Arc::clone(&fixture.inventory))
        } else {
            (members, inventory)
        };
        let wire = context.budget.wire(128 * 1024)?;
        let worker = Arc::clone(&live);
        let lost = Arc::clone(&context.lost);
        let cutoff = context.cutoff;
        let mut ticket =
            context
                .registry
                .start(context.auth.network_group(), move |stop| async move {
                    let _wire = wire;
                    worker.attach_cancel(stop)?;
                    run(
                        &template, &members, fallback, worker, inventory, cutoff, lost,
                    )
                    .await
                })?;
        match ticket.answer().await? {
            Outcome::Failure(message) => Ok(message.into()),
            Outcome::Upstream(answer) => {
                live.current(context.cutoff).await?;
                if let Ok(_copy_credit) = context.budget.wire(4096)
                    && let Some(template) = Template::build(&answer)
                {
                    live.current(context.cutoff).await?;
                    self.cache.insert(key, template, Instant::now());
                }
                Ok(answer.message.into())
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Target {
    transport: Transport,
    address: std::net::SocketAddr,
    #[cfg(test)]
    cleanup_delay: std::time::Duration,
    #[cfg(test)]
    tc_delay: std::time::Duration,
}
impl From<Member> for Target {
    fn from(member: Member) -> Self {
        Self {
            transport: member.transport(),
            address: member.address(),
            #[cfg(test)]
            cleanup_delay: std::time::Duration::ZERO,
            #[cfg(test)]
            tc_delay: std::time::Duration::ZERO,
        }
    }
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct CacheHitBarrier {
    pub entered: tokio::sync::Notify,
    pub release: tokio::sync::Notify,
}
#[cfg(test)]
struct Fixture {
    targets: std::collections::BTreeMap<droutex_common::DnsUpstreamId, Vec<Target>>,
    inventory: Arc<inventory::Inventory>,
    cache_hit: Option<Arc<CacheHitBarrier>>,
}

/// Private test-only exact loopback injection. Production resources still pass
/// the real CP/DP compiler; no public constructor or runtime feature enables it.
#[cfg(test)]
pub(super) fn fixture(
    targets: Vec<(
        droutex_common::DnsUpstreamId,
        Vec<(Transport, std::net::SocketAddr)>,
    )>,
    inventory: Arc<std::sync::Mutex<Result<std::collections::BTreeSet<std::net::IpAddr>>>>,
) -> Result<Forwarder> {
    let mut mapped = std::collections::BTreeMap::new();
    if targets.len() > 64 {
        return Err(DnsError(ErrorCode::Configuration));
    }
    for (id, entries) in targets {
        if entries.is_empty()
            || entries.len() > 2
            || entries
                .iter()
                .any(|(_, address)| !address.ip().is_loopback() || address.port() == 0)
        {
            return Err(DnsError(ErrorCode::Configuration));
        }
        let members = entries
            .into_iter()
            .map(|(transport, address)| Target {
                transport,
                address,
                cleanup_delay: std::time::Duration::ZERO,
                tc_delay: std::time::Duration::ZERO,
            })
            .collect();
        if mapped.insert(id, members).is_some() {
            return Err(DnsError(ErrorCode::Configuration));
        }
    }
    Ok(Forwarder {
        aliases: Vec::new(),
        cache: Cache::new(),
        fixture: Some(Fixture {
            targets: mapped,
            inventory: Arc::new(inventory::Inventory::fixture(inventory)),
            cache_hit: None,
        }),
    })
}

fn query(request: &Message) -> Result<DnsRequest> {
    let question = request
        .queries
        .first()
        .ok_or(DnsError(ErrorCode::Malformed))?;
    let mut options = DnsRequestOptions::default();
    options.edns_payload_len = crate::MAX_UDP_BYTES as u16;
    // Qualified library creates an independent transaction ID and a fresh
    // RD/EDNS0 message. No client EDNS option or DNSSEC flag is copied.
    Ok(DnsRequest::from_query(question.clone(), options))
}

async fn transport(
    request: &Message,
    member: Target,
    tcp: bool,
    deadline: Instant,
    live: Arc<live::Live>,
    inventory: Arc<inventory::Inventory>,
) -> Result<(DnsResponse, Instant)> {
    let outbound = query(request)?;
    let budget = io::Budget::new(member.address, deadline, live, inventory);
    let result = if tcp {
        match client::TcpAttempt::connect(Arc::clone(&budget)).await {
            Ok(mut owner) => {
                let result = owner.query(outbound).await;
                #[cfg(test)]
                tokio::time::sleep(member.cleanup_delay).await;
                // This await is outside every cancellable query future. A3s
                // timeout retains this worker/slot until the same handle joins.
                match owner.finish().await {
                    Ok(()) => result,
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    } else {
        client::udp(Arc::clone(&budget), outbound).await
    };
    budget.finish_inventory().await?;
    if budget.destination_refused() {
        return Err(DnsError(ErrorCode::Scope));
    }
    let received = budget.receipt_floor();
    result
        .map_err(|error| match error {
            client::Failure::Deadline | client::Failure::Cleanup => DnsError(ErrorCode::Timeout),
            _ => DnsError(ErrorCode::Unavailable),
        })
        .and_then(|response| Ok((response, received?)))
}

async fn run(
    request: &Message,
    members: &[Target],
    fallback: Fallback,
    live: Arc<live::Live>,
    inventory: Arc<inventory::Inventory>,
    cutoff: Instant,
    lost: Arc<AtomicBool>,
) -> Result<Outcome> {
    for (member_index, member) in members.iter().enumerate() {
        let deadline = (Instant::now() + io::ATTEMPT).min(cutoff);
        let mut result = async {
            live.current(deadline).await?;
            transport(
                request,
                *member,
                member.transport == Transport::Tcp,
                deadline,
                Arc::clone(&live),
                Arc::clone(&inventory),
            )
            .await
        }
        .await;
        let response_was_tcp = member.transport == Transport::UdpTcp
            && result
                .as_ref()
                .is_ok_and(|(reply, _)| reply.metadata.truncation);
        if member.transport == Transport::UdpTcp
            && result
                .as_ref()
                .is_ok_and(|(reply, _)| reply.metadata.truncation)
        {
            #[cfg(test)]
            tokio::time::sleep(member.tc_delay).await;
            result = async {
                live.current(deadline).await?;
                transport(
                    request,
                    *member,
                    true,
                    deadline,
                    Arc::clone(&live),
                    Arc::clone(&inventory),
                )
                .await
            }
            .await;
        }
        if lost.load(Ordering::Acquire) {
            return Err(DnsError(ErrorCode::Admission));
        }
        live.current(cutoff).await?;
        if result
            .as_ref()
            .is_err_and(|error| error.code() == ErrorCode::Scope)
        {
            break;
        }
        if let Ok((response, received)) = result
            && !response.metadata.truncation
            && (fallback == Fallback::None
                || !matches!(
                    response.metadata.response_code,
                    ResponseCode::ServFail | ResponseCode::Refused
                ))
        {
            let reply = crate::wire::sanitize(request, response.into_message(), true);
            if crate::wire::fits_complete(&reply, io::MESSAGE_BYTES) && Instant::now() < deadline {
                return Ok(Outcome::Upstream(UpstreamAnswer {
                    message: reply,
                    received,
                    member: member_index,
                    transport: if member.transport == Transport::Tcp
                        || member.transport == Transport::UdpTcp && response_was_tcp
                    {
                        Transport::Tcp
                    } else {
                        Transport::UdpTcp
                    },
                }));
            }
        }
        if fallback == Fallback::None {
            break;
        }
    }
    live.current(cutoff).await?;
    Ok(Outcome::Failure(crate::wire::sanitize(
        request,
        response(request, ResponseCode::ServFail),
        true,
    )))
}

#[cfg(test)]
pub(super) async fn candidate_limit_fixture(
    auth: &AuthorizedRequest<'_>,
    owner: Arc<droutex_core::deployment::Deployment>,
) {
    use std::{collections::BTreeSet, sync::Mutex};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::watch,
        time::{Duration, timeout},
    };
    for foreign in [client::CANDIDATES - 1, client::CANDIDATES] {
        let refresh = Arc::new(Refresh::new());
        let upkeep = Arc::clone(&owner);
        assert!(
            refresh
                .perform(move || {
                    upkeep.refresh_authorization_clock(
                        &droutex_core::deployment::Cancellation::default(),
                    )
                })
                .await
                .unwrap()
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let peer = listener.local_addr().unwrap();
        let (_stop, stopped) = watch::channel(false);
        let live = Arc::new(live::Live::new(
            auth.dns_io_check().unwrap(),
            refresh,
            Instant::now() + Duration::from_millis(1500),
            Arc::new(AtomicBool::new(false)),
            Some(stopped),
        ));
        let inventory = Arc::new(inventory::Inventory::fixture(Arc::new(Mutex::new(Ok(
            BTreeSet::new(),
        )))));
        let budget = io::Budget::new(peer, Instant::now() + io::ATTEMPT, live, inventory);
        let witness = Arc::clone(&budget);
        let remote = async {
            let (mut stream, _) = listener.accept().await.unwrap();
            stream.set_nodelay(true).unwrap();
            let length = stream.read_u16().await.unwrap();
            let mut request = vec![0; usize::from(length)];
            stream.read_exact(&mut request).await.unwrap();
            let mut good = b"\x00\x00\x81\x80\x00\x01\x00\x00\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01".to_vec();
            good[..2].copy_from_slice(&request[..2]);
            let mut bad = good.clone();
            bad[13] = b'f';
            for index in 0..foreign {
                stream.write_u16(bad.len() as u16).await.unwrap();
                stream.write_all(&bad).await.unwrap();
                // Prove delivery to validation rather than overflowing the library's
                // bounded response queue and mistaking a dropped valid packet for a limit.
                loop {
                    let observed = witness.counters.rejected_wake.notified();
                    if witness.counters.rejected.load(Ordering::Relaxed) > index {
                        break;
                    }
                    observed.await;
                }
            }
            if foreign < client::CANDIDATES {
                stream.write_u16(good.len() as u16).await.unwrap();
                stream.write_all(&good).await.unwrap();
            }
            let mut byte = [0];
            assert!(matches!(stream.read(&mut byte).await, Ok(0) | Err(_)));
        };
        let local = async {
            let template = Message::from_vec(b"\x04\xd2\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01").unwrap();
            let mut attempt = client::TcpAttempt::connect(Arc::clone(&budget))
                .await
                .unwrap();
            let response = attempt.query(query(&template).unwrap()).await;
            attempt.finish().await.unwrap();
            if foreign < client::CANDIDATES {
                assert!(response.is_ok());
            } else {
                assert!(
                    matches!(response, Err(client::Failure::Candidates)),
                    "{response:?}"
                );
            }
        };
        timeout(Duration::from_secs(3), async {
            tokio::join!(local, remote);
        })
        .await
        .unwrap();
        budget.finish_inventory().await.unwrap();
        assert_eq!(budget.counters.rejected.load(Ordering::Relaxed), foreign);
        assert_eq!(budget.counters.sockets.load(Ordering::Relaxed), 0);
    }
}
