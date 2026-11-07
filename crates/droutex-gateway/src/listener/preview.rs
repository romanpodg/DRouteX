//! Opt-in preview and its same-generation experimental DNS readiness evidence.
use super::{Binding, CLEANUP, ListenerError, ListenerSocket, MAX_LISTENERS, Result};
use crate::{origin::resolver::OriginResolver, tls_metadata::read_hello};
use droutex_common::{ListenerId, OutboundAvailability, ProxyOutboundId, ProxyOutboundKind};
use droutex_config::OutboundObservation;
use droutex_core::deployment::{Deployment, SmartDnsBinding};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::{Semaphore, oneshot, watch},
    task::JoinSet,
    time::{Instant, timeout},
};

const MAX_SESSIONS: usize = 64;
const MAX_OBSERVATIONS: usize = 256;
const HELLO_TIME: Duration = Duration::from_secs(5);
const ORIGIN_TIME: Duration = Duration::from_secs(2);
const SESSION_TIME: Duration = Duration::from_secs(120);

#[cfg(all(test, target_os = "linux"))]
#[path = "preview/tls_acceptance.rs"]
mod tls_acceptance;

struct ReadyState {
    active: AtomicBool,
    deployment: Weak<Deployment>,
    metadata: droutex_config::snapshot::SnapshotMetadata,
    listeners: BTreeMap<ListenerId, std::net::SocketAddr>,
    outbounds: BTreeSet<ProxyOutboundId>,
    resolver: Weak<OriginResolver>,
    shutdown: watch::Receiver<bool>,
}

/// Evidence owned by running experimental listener tasks. It is not a grant.
#[derive(Clone)]
pub struct GatewayReadiness(Arc<ReadyState>);
impl GatewayReadiness {
    /// Check the exact owner, generation and listener, in addition to the
    /// binding's separate live lease/current-generation check.
    pub fn matches(&self, binding: &SmartDnsBinding<'_, '_>) -> bool {
        let state = &self.0;
        state.active.load(Ordering::Acquire)
            && !*state.shutdown.borrow()
            && state.shutdown.has_changed().is_ok()
            && state
                .deployment
                .upgrade()
                .is_some_and(|owner| binding.belongs_to(&owner))
            && state.metadata == binding.metadata()
            && state
                .listeners
                .get(&binding.listener())
                .is_some_and(|local| local.port() == binding.port())
            && state.outbounds.contains(&binding.outbound())
            && state
                .resolver
                .upgrade()
                .is_some_and(|resolver| resolver.inventory_available())
            && state.active.load(Ordering::Acquire)
    }
    /// The advertised address must match the bound socket or an explicitly
    /// host-declared alias, in addition to matching the socket address family.
    /// A declared alias is operator evidence, not a reachability probe.
    pub fn supports_address(
        &self,
        binding: &SmartDnsBinding<'_, '_>,
        address: std::net::IpAddr,
    ) -> bool {
        self.matches(binding)
            && self
                .0
                .listeners
                .get(&binding.listener())
                .is_some_and(|local| {
                    local.is_ipv4() == address.is_ipv4()
                        && (local.ip() == address
                            || self
                                .0
                                .resolver
                                .upgrade()
                                .is_some_and(|resolver| resolver.has_trusted_alias(address)))
                })
    }
}
struct ReadyGuard(Arc<ReadyState>);
impl Drop for ReadyGuard {
    fn drop(&mut self) {
        self.0.active.store(false, Ordering::Release);
    }
}

/// An explicit experimental listener. The host must maintain the deployment's
/// authorization clock, retain and poll this future, and signal shutdown.
/// Missing checkpoints, unsupported outbounds and all parsing/admission errors
/// close the connection. This does not grant SmartDNS DNS readiness.
pub async fn serve_preview(
    deployment: Arc<Deployment>,
    sockets: Vec<ListenerSocket>,
    shutdown: watch::Receiver<bool>,
    resolver: Arc<OriginResolver>,
    observations: Vec<OutboundObservation>,
) -> Result<()> {
    serve(deployment, sockets, shutdown, resolver, observations, None).await
}

/// Run the experimental listener and publish evidence only after its bound
/// sockets have been checked. Dropping this future invalidates that evidence.
pub async fn serve_preview_ready(
    deployment: Arc<Deployment>,
    sockets: Vec<ListenerSocket>,
    shutdown: watch::Receiver<bool>,
    resolver: Arc<OriginResolver>,
    observations: Vec<OutboundObservation>,
    ready: oneshot::Sender<GatewayReadiness>,
) -> Result<()> {
    serve(
        deployment,
        sockets,
        shutdown,
        resolver,
        observations,
        Some(ready),
    )
    .await
}

async fn serve(
    deployment: Arc<Deployment>,
    sockets: Vec<ListenerSocket>,
    shutdown: watch::Receiver<bool>,
    resolver: Arc<OriginResolver>,
    observations: Vec<OutboundObservation>,
    ready: Option<oneshot::Sender<GatewayReadiness>>,
) -> Result<()> {
    if sockets.is_empty() || sockets.len() > MAX_LISTENERS || observations.len() > MAX_OBSERVATIONS
    {
        return Err(ListenerError::Configuration);
    }
    let current = deployment.pin().map_err(|_| ListenerError::Unavailable)?;
    let bindings = checked_bindings(&current, sockets)?;
    let outbounds = usable_outbounds(&current, &observations);
    let state = Arc::new(ReadyState {
        active: AtomicBool::new(true),
        deployment: Arc::downgrade(&deployment),
        metadata: current.metadata(),
        listeners: bindings
            .iter()
            .map(|(binding, _)| (binding.id, binding.local))
            .collect(),
        outbounds,
        resolver: Arc::downgrade(&resolver),
        shutdown: shutdown.clone(),
    });
    let _guard = ReadyGuard(Arc::clone(&state));
    drop(current);
    let permits = Arc::new(Semaphore::new(MAX_SESSIONS));
    let facts = Arc::new(observations);
    let (cancel, stop) = watch::channel(false);
    let mut listeners = JoinSet::new();
    for (binding, socket) in bindings {
        let task = ListenerTask {
            owner: Arc::clone(&deployment),
            resolver: Arc::clone(&resolver),
            facts: Arc::clone(&facts),
            permits: Arc::clone(&permits),
            stop: stop.clone(),
            _guard: ReadyGuard(Arc::clone(&state)),
        };
        listeners.spawn(task.run(binding, socket));
    }
    if let Some(ready) = ready {
        let _ = ready.send(GatewayReadiness(state));
    }
    wait_and_cleanup(shutdown, listeners, cancel, &resolver).await
}

async fn wait_and_cleanup(
    mut shutdown: watch::Receiver<bool>,
    mut listeners: JoinSet<Result<()>>,
    cancel: watch::Sender<bool>,
    resolver: &OriginResolver,
) -> Result<()> {
    let outcome = loop {
        if *shutdown.borrow() {
            break Ok(());
        }
        tokio::select! {
            changed = shutdown.changed() => if changed.is_err() || *shutdown.borrow_and_update() { break Ok(()); },
            result = listeners.join_next() => break match result {
                Some(Ok(Err(error))) => Err(error),
                Some(Ok(Ok(()))) if *shutdown.borrow() || shutdown.has_changed().is_err() => Ok(()),
                _ => Err(ListenerError::Unavailable),
            },
        }
    };
    cancel.send_replace(true);
    let drained = drain_listeners(&mut listeners).await;
    let cleanup = timeout(CLEANUP, resolver.shutdown_inventory()).await;
    if !drained || !matches!(cleanup, Ok(Ok(()))) {
        return Err(ListenerError::Cleanup);
    }
    outcome
}

async fn drain_listeners(listeners: &mut JoinSet<Result<()>>) -> bool {
    if timeout(CLEANUP, async {
        while listeners.join_next().await.is_some() {}
    })
    .await
    .is_ok()
    {
        return true;
    }
    listeners.abort_all();
    while listeners.join_next().await.is_some() {}
    false
}

fn checked_bindings(
    current: &droutex_core::deployment::Decision<'_>,
    sockets: Vec<ListenerSocket>,
) -> Result<Vec<(Binding, tokio::net::TcpListener)>> {
    let mut ids = BTreeSet::new();
    let mut addresses = BTreeSet::new();
    let mut bindings = Vec::with_capacity(sockets.len());
    for listener in sockets {
        let binding = Binding {
            id: listener.id,
            local: listener
                .socket
                .local_addr()
                .map_err(|_| ListenerError::Io)?,
        };
        if !ids.insert(binding.id) || !addresses.insert(binding.local) {
            return Err(ListenerError::Configuration);
        }
        binding.check(current)?;
        bindings.push((binding, listener.socket));
    }
    Ok(bindings)
}

struct ListenerTask {
    owner: Arc<Deployment>,
    resolver: Arc<OriginResolver>,
    facts: Arc<Vec<OutboundObservation>>,
    permits: Arc<Semaphore>,
    stop: watch::Receiver<bool>,
    _guard: ReadyGuard,
}
impl ListenerTask {
    async fn run(mut self, binding: Binding, socket: tokio::net::TcpListener) -> Result<()> {
        let mut sessions = JoinSet::new();
        let outcome = loop {
            while sessions.try_join_next().is_some() {}
            if *self.stop.borrow() {
                break Ok(());
            }
            tokio::select! {
                changed = self.stop.changed() => if changed.is_err() || *self.stop.borrow_and_update() { break Ok(()); },
                result = sessions.join_next(), if !sessions.is_empty() => { let _ = result; },
                accepted = socket.accept() => {
                    let (stream, peer) = match accepted { Ok(result) => result, Err(_) => break Err(ListenerError::Io) };
                    if let Err(error) = self.spawn_session(binding, stream, peer.ip(), &mut sessions) {
                        break Err(error);
                    }
                }
            }
        };
        sessions.abort_all();
        while sessions.join_next().await.is_some() {}
        outcome
    }

    fn spawn_session(
        &self,
        binding: Binding,
        stream: TcpStream,
        peer: std::net::IpAddr,
        sessions: &mut JoinSet<()>,
    ) -> Result<()> {
        let current = self.owner.pin().map_err(|_| ListenerError::Unavailable)?;
        binding.check(&current)?;
        drop(current);
        let Ok(permit) = Arc::clone(&self.permits).try_acquire_owned() else {
            return Ok(());
        };
        let owner = Arc::clone(&self.owner);
        let resolver = Arc::clone(&self.resolver);
        let facts = Arc::clone(&self.facts);
        let mut session_stop = self.stop.clone();
        sessions.spawn(async move {
            let _permit = permit;
            if *session_stop.borrow() {
                return;
            }
            let work = session(
                owner,
                resolver,
                facts,
                binding.id,
                peer,
                stream,
                session_stop.clone(),
            );
            tokio::select! {
                biased;
                _ = session_stop.wait_for(|stopped| *stopped) => (),
                _ = work => (),
            }
        });
        Ok(())
    }
}

/// Readiness covers only outbounds that the preview can actually connect.
fn usable_outbounds(
    current: &droutex_core::deployment::Decision<'_>,
    observations: &[OutboundObservation],
) -> BTreeSet<ProxyOutboundId> {
    if current.origin_upstream().is_none() {
        return BTreeSet::new();
    }
    let configured = &current.gateway_configuration().document().outbounds;
    observations
        .iter()
        .filter(|observation| {
            observation.kind == ProxyOutboundKind::GatewayDirect
                && observation.availability == OutboundAvailability::Available
                && observation.tcp == droutex_policy::TcpCapability::Supported
                && observations
                    .iter()
                    .filter(|item| item.id == observation.id)
                    .count()
                    == 1
                && configured.iter().any(|outbound| {
                    outbound.id == observation.id
                        && outbound.enabled
                        && matches!(
                            outbound.transport,
                            droutex_config::OutboundKind::VpsDirect {}
                        )
                })
        })
        .map(|observation| observation.id)
        .collect()
}

async fn session(
    owner: Arc<Deployment>,
    resolver: Arc<OriginResolver>,
    facts: Arc<Vec<OutboundObservation>>,
    listener: ListenerId,
    peer: std::net::IpAddr,
    stream: TcpStream,
    stop: watch::Receiver<bool>,
) -> Option<()> {
    let ingress = owner
        .reserve_smartdns_gateway_ingress(listener, peer)
        .ok()?;
    let hello = read_hello(stream, Instant::now() + HELLO_TIME).await.ok()?;
    let attempt = ingress
        .prepare_for_name(hello.name().as_str(), (*facts).clone())
        .ok()?;
    let auth = attempt.consume().ok()?;
    let deadline = Instant::now() + ORIGIN_TIME;
    let result = resolver.resolve(&auth, deadline).await.ok()?;
    let connection = result.connect_direct(0, deadline).await.ok()?;
    connection
        .relay(hello, stop, Instant::now() + SESSION_TIME)
        .await
        .ok()?;
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        origin::resolver::tests::{
            admitted, apply, fixture, fixture_with_dial, id, keys, observations, serve_dns,
            smart_document,
        },
        tls_metadata::tests::{sni, synthetic},
    };
    use serde_json::json;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, UdpSocket},
    };

    #[tokio::test]
    async fn listener_cleanup_joins_completed_and_aborted_tasks() {
        let mut completed = JoinSet::new();
        completed.spawn(async { Ok(()) });
        assert!(drain_listeners(&mut completed).await);
        assert!(completed.is_empty());

        let mut blocked = JoinSet::new();
        let (dropped, receipt) = oneshot::channel::<()>();
        blocked.spawn(async move {
            let _dropped = dropped;
            std::future::pending::<Result<()>>().await
        });
        assert!(!drain_listeners(&mut blocked).await);
        assert!(blocked.is_empty());
        assert!(receipt.await.is_err());
    }

    async fn refuses_before_dns(source: &str, hello: Vec<u8>) {
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let dns = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let resolver = Arc::new(fixture(dns.local_addr().unwrap()));
        let owner = Arc::new(Deployment::in_memory(id(1), keys().1));
        let mut doc = smart_document(address);
        doc["gatewayGrants"][0]["source"] = json!(source);
        apply(&owner, &doc, 1);
        let (cancel, stop) = watch::channel(false);
        let server = tokio::spawn(serve_preview(
            owner,
            vec![ListenerSocket { id: id(22), socket }],
            stop,
            resolver,
            observations(),
        ));
        let mut client = TcpStream::connect(address).await.unwrap();
        client.write_all(&hello).await.unwrap();
        let mut byte = [0u8; 1];
        let read = timeout(Duration::from_secs(3), client.read(&mut byte))
            .await
            .unwrap();
        assert!(
            matches!(read, Ok(0) | Err(_)),
            "denied client must not receive data"
        );
        let mut query = [0u8; 512];
        assert!(
            timeout(Duration::from_millis(100), dns.recv_from(&mut query))
                .await
                .is_err(),
            "denied input must not reach origin DNS"
        );
        cancel.send(true).unwrap();
        timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn encrypted_name_is_rejected_before_resolution() {
        let mut extensions = sni(b"service.test");
        extensions.extend_from_slice(&[0xfe, 0x0d, 0, 1, 0]);
        refuses_before_dns("127.0.0.1", synthetic(&extensions)).await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unrecognized_source_is_rejected_before_resolution() {
        refuses_before_dns("127.0.0.2", synthetic(&sni(b"service.test"))).await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shutdown_closes_partial_clienthello() {
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let owner = Arc::new(Deployment::in_memory(id(1), keys().1));
        apply(&owner, &smart_document(address), 1);
        let (cancel, stop) = watch::channel(false);
        let server = tokio::spawn(serve_preview(
            owner,
            vec![ListenerSocket { id: id(22), socket }],
            stop,
            Arc::new(OriginResolver::new(vec![]).unwrap()),
            observations(),
        ));
        let mut client = TcpStream::connect(address).await.unwrap();
        client.write_all(&[0x16]).await.unwrap();
        cancel.send(true).unwrap();
        timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let mut byte = [0u8; 1];
        let read = timeout(Duration::from_secs(2), client.read(&mut byte))
            .await
            .unwrap();
        assert!(matches!(read, Ok(0) | Err(_)));
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn preview_composes_ingress_policy_dns_and_byte_relay() {
        let ingress = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = ingress.local_addr().unwrap();
        let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin_address = origin.local_addr().unwrap();
        let dns = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let dns_address = dns.local_addr().unwrap();
        let dns_server = tokio::spawn(serve_dns(dns, "8.8.8.8".parse().unwrap(), 3));
        let owner = Arc::new(Deployment::in_memory(id(1), keys().1));
        apply(&owner, &smart_document(address), 1);
        drop(admitted(&owner).await); // Maintain the clock; this grants no preview session.
        tokio::time::sleep(Duration::from_millis(100)).await; // Replenish the spent rate credit.
        let resolver = Arc::new(fixture_with_dial(dns_address, origin_address));
        let (cancel, stop) = watch::channel(false);
        let server = tokio::spawn(serve_preview(
            owner,
            vec![ListenerSocket {
                id: id(22),
                socket: ingress,
            }],
            stop,
            resolver,
            observations(),
        ));
        let origin_task = tokio::spawn(async move {
            let (mut stream, _) = origin.accept().await.unwrap();
            let mut received = Vec::new();
            stream.read_to_end(&mut received).await.unwrap();
            stream.write_all(b"origin-reply").await.unwrap();
            stream.shutdown().await.unwrap();
            received
        });
        let mut client = TcpStream::connect(address).await.unwrap();
        let mut expected = synthetic(&sni(b"service.test"));
        expected.extend_from_slice(b"application-data");
        client.write_all(&expected).await.unwrap();
        client.shutdown().await.unwrap();
        let mut response = Vec::new();
        timeout(Duration::from_secs(5), client.read_to_end(&mut response))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response, b"origin-reply");
        assert_eq!(
            timeout(Duration::from_secs(3), origin_task)
                .await
                .unwrap()
                .unwrap(),
            expected
        );
        timeout(Duration::from_secs(3), dns_server)
            .await
            .unwrap()
            .unwrap();
        cancel.send(true).unwrap();
        timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
