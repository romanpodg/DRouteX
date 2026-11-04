//! Bounded independent DNS origin resolution and opaque direct connection.
use super::{
    inventory::{InventoryError, OriginInventory},
    registry::Registry,
    response::{QueryKind, ResponseChain, ResponseError, ResponseStep},
    screen_resolution,
};
use droutex_config::upstream::{Fallback, Transport};
use droutex_core::deployment::{AdmissionScope, AuthorizedRequest};
use droutex_policy::TcpIntent;
use hickory_proto::{
    op::{Message, Query},
    rr::Name,
};
use std::{
    error::Error,
    fmt,
    future::{Future, poll_fn},
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    num::NonZeroU16,
    sync::{Arc, OnceLock},
    task::Poll,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
    sync::{OwnedSemaphorePermit, Semaphore},
    time::{Instant, timeout_at},
};

const TOTAL: Duration = Duration::from_secs(2);
const ATTEMPT: Duration = Duration::from_millis(500);
const CLEANUP: Duration = Duration::from_secs(3);
const MAX_PACKETS: usize = 16;
const MAX_WIRE: usize = 4096;
const MAX_IO_OPS: usize = 256;
const MAX_RESOLVERS: usize = 8;
const MAX_CONNECTIONS: usize = 8;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
static CONNECTION_SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
static SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolveError {
    Admission,
    Unavailable,
    Busy,
    Timeout,
    Io,
    InvalidResponse,
    Truncated,
    NoAddress,
    UnsafeAddress,
    Cleanup,
    UnsupportedOutbound,
    InvalidCandidate,
}
impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "origin resolver {self:?}")
    }
}
impl Error for ResolveError {}
type Result<T> = std::result::Result<T, ResolveError>;

/// Checked numeric DNS results only. A later connector must recheck live
/// admission and fresh self inventory immediately before dialing a numeric address.
pub struct ResolvedOrigin<'request, 'store> {
    owner: &'request AuthorizedRequest<'store>,
    domain: String,
    addresses: Vec<IpAddr>,
    port: NonZeroU16,
    aliases: Vec<IpAddr>,
    deadline: Instant,
    registry: Arc<Registry>,
}
impl fmt::Debug for ResolvedOrigin<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedOrigin").finish_non_exhaustive()
    }
}
impl ResolvedOrigin<'_, '_> {
    pub fn addresses(&self) -> &[IpAddr] {
        &self.addresses
    }
    pub fn port(&self) -> NonZeroU16 {
        self.port
    }
    /// Rechecks the original gateway admission and resolution deadline. This
    /// alone does not replace fresh self-inventory before a numeric dial.
    pub fn verify_current(&self) -> Result<()> {
        current(self.owner, &self.domain, self.port, self.deadline)
    }
}

/// A connected socket is intentionally opaque: data transfer is not yet
/// implemented and must preserve current-admission checks on every I/O poll.
/// The original admission and global connection slot stay held for its lifetime.
pub struct OriginConnection<'request, 'store> {
    _origin: ResolvedOrigin<'request, 'store>,
    _stream: TcpStream,
    _slot: OwnedSemaphorePermit,
}
impl fmt::Debug for OriginConnection<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginConnection").finish_non_exhaustive()
    }
}
impl<'request, 'store> ResolvedOrigin<'request, 'store> {
    /// Establish one numeric direct connection from this exact resolution.
    /// This does not implement a gateway listener or permit data transfer.
    /// Consumes the resolution; a retry requires a new DNS resolution.
    ///
    /// ```compile_fail
    /// use droutex_gateway::origin::resolver::ResolvedOrigin;
    /// use tokio::time::Instant;
    /// async fn reuse(origin: ResolvedOrigin<'_, '_>, deadline: Instant) {
    ///     let _ = origin.connect_direct(0, deadline).await;
    ///     let _ = origin.connect_direct(0, deadline).await;
    /// }
    /// ```
    pub async fn connect_direct(
        self,
        index: usize,
        outer_deadline: Instant,
    ) -> Result<OriginConnection<'request, 'store>> {
        self.connect_direct_with(index, outer_deadline, TcpStream::connect)
            .await
    }

    // Private injection keeps production callers on numeric TcpStream::connect;
    // tests can exercise the whole gate with a controlled local transport.
    async fn connect_direct_with<F, D>(
        self,
        index: usize,
        outer_deadline: Instant,
        dial: D,
    ) -> Result<OriginConnection<'request, 'store>>
    where
        D: FnOnce(SocketAddr) -> F,
        F: Future<Output = io::Result<TcpStream>>,
    {
        let deadline = outer_deadline
            .min(self.deadline)
            .min(Instant::now() + CONNECT_TIMEOUT);
        current(self.owner, &self.domain, self.port, deadline)?;
        match self.owner.scope() {
            AdmissionScope::Gateway(candidate) if matches!(candidate.action().intent(), TcpIntent::GatewayDirect(id) if id == candidate.outbound()) =>
                {}
            _ => return Err(ResolveError::UnsupportedOutbound),
        }
        let address = *self
            .addresses
            .get(index)
            .ok_or(ResolveError::InvalidCandidate)?;
        let document = self.owner.decision().gateway_configuration().document();
        let mut configured = self.aliases.clone();
        configured.extend(document.listeners.iter().map(|listener| listener.bind.ip()));
        screen_resolution(&[address], &configured).map_err(|_| ResolveError::UnsafeAddress)?;
        let inventory = OriginInventory::with_registry(
            &self.aliases,
            document.listeners.iter().map(|listener| listener.bind.ip()),
            Arc::clone(&self.registry),
        )
        .map_err(|_| ResolveError::Unavailable)?;
        let slot =
            Arc::clone(CONNECTION_SLOTS.get_or_init(|| Arc::new(Semaphore::new(MAX_CONNECTIONS))))
                .try_acquire_owned()
                .map_err(|_| ResolveError::Busy)?;
        probe(
            &inventory,
            self.owner,
            &self.domain,
            self.port,
            address,
            deadline,
        )
        .await?;
        let target = SocketAddr::new(address, self.port.get());
        let stream = guarded(self.owner, &self.domain, self.port, deadline, dial(target)).await?;
        Ok(OriginConnection {
            _origin: self,
            _stream: stream,
            _slot: slot,
        })
    }
}

/// Caller is the trusted host and supplies all public/NAT aliases not visible to
/// the OS. Listener addresses are derived from the held signed generation.
pub struct OriginResolver {
    aliases: Vec<IpAddr>,
    slots: Arc<Semaphore>,
    registry: Arc<Registry>,
    #[cfg(test)]
    fixture: Option<SocketAddr>,
}
impl OriginResolver {
    pub fn new(aliases: Vec<IpAddr>) -> Result<Self> {
        if aliases.len() > super::MAX_SELF_ADDRESSES {
            return Err(ResolveError::Unavailable);
        }
        Ok(Self {
            aliases,
            slots: Arc::clone(SLOTS.get_or_init(|| Arc::new(Semaphore::new(MAX_RESOLVERS)))),
            registry: Arc::new(Registry::new()),
            #[cfg(test)]
            fixture: None,
        })
    }

    /// Refuse new inventory work, cancel probes and join their blocking workers.
    /// This does not close established sockets or stop unrelated resolver I/O.
    pub async fn shutdown_inventory(&self) -> Result<()> {
        self.registry
            .shutdown()
            .await
            .map_err(|_| ResolveError::Cleanup)
    }

    /// Resolution starts from the exact target of an already authorized gateway
    /// request. No client-supplied DNS policy or system resolver is consulted.
    pub async fn resolve<'request, 'store>(
        &self,
        auth: &'request AuthorizedRequest<'store>,
        outer_deadline: Instant,
    ) -> Result<ResolvedOrigin<'request, 'store>> {
        let _slot = Arc::clone(&self.slots)
            .try_acquire_owned()
            .map_err(|_| ResolveError::Busy)?;
        let (domain, port) = match auth.scope() {
            AdmissionScope::Gateway(candidate) => {
                (candidate.domain().as_str().to_owned(), candidate.port())
            }
            _ => return Err(ResolveError::Admission),
        };
        let deadline = outer_deadline.min(Instant::now() + TOTAL);
        current(auth, &domain, port, deadline)?;
        let group = auth
            .decision()
            .origin_upstream()
            .ok_or(ResolveError::Unavailable)?;
        let document = auth.decision().gateway_configuration().document();
        let inventory = OriginInventory::with_registry(
            &self.aliases,
            document.listeners.iter().map(|l| l.bind.ip()),
            Arc::clone(&self.registry),
        )
        .map_err(|_| ResolveError::Unavailable)?;
        let mut addresses = Vec::new();
        for kind in [QueryKind::Ipv4, QueryKind::Ipv6] {
            let mut chain =
                ResponseChain::new(&domain, kind).map_err(|_| ResolveError::InvalidResponse)?;
            loop {
                current(auth, &domain, port, deadline)?;
                let mut result = None;
                for (index, member) in group.members().iter().enumerate() {
                    if index > 0 && group.fallback() != Fallback::OnFailure {
                        break;
                    }
                    let attempt = (Instant::now() + ATTEMPT).min(deadline);
                    let target = member.address();
                    #[cfg(test)]
                    let target = self.fixture.unwrap_or(target);
                    #[cfg(test)]
                    let check_upstream = self.fixture.is_none();
                    #[cfg(not(test))]
                    let check_upstream = true;
                    if check_upstream {
                        probe(&inventory, auth, &domain, port, target.ip(), attempt).await?;
                    }
                    let response = match member.transport() {
                        Transport::UdpTcp => {
                            let response =
                                udp_query(auth, &domain, port, &mut chain, kind, target, attempt)
                                    .await;
                            if matches!(response, Err(ResolveError::Truncated)) {
                                if check_upstream {
                                    probe(&inventory, auth, &domain, port, target.ip(), attempt)
                                        .await?;
                                }
                                tcp_query(auth, &domain, port, &mut chain, kind, target, attempt)
                                    .await
                            } else {
                                response
                            }
                        }
                        Transport::Tcp => {
                            tcp_query(auth, &domain, port, &mut chain, kind, target, attempt).await
                        }
                    };
                    match response {
                        Ok(step) => {
                            result = Some(Ok(step));
                            break;
                        }
                        Err(error @ (ResolveError::Admission | ResolveError::Cleanup)) => {
                            return Err(error);
                        }
                        Err(error) => {
                            result = Some(Err(error));
                        }
                    }
                }
                match result.ok_or(ResolveError::Unavailable)?? {
                    Some(ResponseStep::Follow(_)) => continue,
                    Some(ResponseStep::Addresses(mut found)) => {
                        addresses.append(&mut found);
                        break;
                    }
                    None => break, // No matching answer for this family; no negative caching.
                }
            }
        }
        let mut configured = self.aliases.clone();
        configured.extend(document.listeners.iter().map(|l| l.bind.ip()));
        if addresses.is_empty() {
            return Err(ResolveError::NoAddress);
        }
        let checked =
            screen_resolution(&addresses, &configured).map_err(|_| ResolveError::UnsafeAddress)?;
        for &address in checked.addresses() {
            probe(&inventory, auth, &domain, port, address, deadline).await?;
        }
        current(auth, &domain, port, deadline)?;
        Ok(ResolvedOrigin {
            owner: auth,
            domain,
            addresses: checked.addresses().to_vec(),
            port,
            aliases: self.aliases.clone(),
            deadline,
            registry: Arc::clone(&self.registry),
        })
    }
}
fn current(
    auth: &AuthorizedRequest<'_>,
    domain: &str,
    port: NonZeroU16,
    deadline: Instant,
) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(ResolveError::Timeout);
    }
    auth.verify_gateway_destination(domain, port)
        .map_err(|_| ResolveError::Admission)
}
async fn guarded<T>(
    auth: &AuthorizedRequest<'_>,
    domain: &str,
    port: NonZeroU16,
    deadline: Instant,
    operation: impl Future<Output = io::Result<T>>,
) -> Result<T> {
    tokio::pin!(operation);
    let output = timeout_at(
        deadline,
        poll_fn(|cx| {
            if let Err(error) = current(auth, domain, port, deadline) {
                return Poll::Ready(Err(error));
            }
            match operation.as_mut().poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(result) => Poll::Ready(result.map_err(|_| ResolveError::Io)),
            }
        }),
    )
    .await
    .map_err(|_| ResolveError::Timeout)??;
    current(auth, domain, port, deadline)?;
    Ok(output)
}
async fn probe(
    inventory: &OriginInventory,
    auth: &AuthorizedRequest<'_>,
    domain: &str,
    port: NonZeroU16,
    target: IpAddr,
    deadline: Instant,
) -> Result<()> {
    current(auth, domain, port, deadline)?;
    let mut probe = inventory
        .start(target, deadline)
        .map_err(|_| ResolveError::Unavailable)?;
    let result = timeout_at(
        deadline,
        poll_fn(|cx| {
            if let Err(error) = current(auth, domain, port, deadline) {
                return Poll::Ready(Err(error));
            }
            probe.poll(cx).map(|result| {
                result.map_err(|error| match error {
                    InventoryError::SelfAddress => ResolveError::UnsafeAddress,
                    InventoryError::Timeout => ResolveError::Timeout,
                    _ => ResolveError::Unavailable,
                })
            })
        }),
    )
    .await
    .map_err(|_| ResolveError::Timeout)
    .and_then(|r| r);
    match timeout_at(Instant::now() + CLEANUP, probe.finish()).await {
        Ok(Ok(())) => (),
        _ => return Err(ResolveError::Cleanup),
    }
    result?;
    current(auth, domain, port, deadline)
}
async fn udp_query(
    auth: &AuthorizedRequest<'_>,
    domain: &str,
    port: NonZeroU16,
    chain: &mut ResponseChain,
    kind: QueryKind,
    target: SocketAddr,
    deadline: Instant,
) -> Result<Option<ResponseStep>> {
    let (id, bytes) = wire_query(chain, kind)?;
    let bind = match target {
        SocketAddr::V4(_) => SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
        SocketAddr::V6(_) => SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
    };
    let socket = guarded(auth, domain, port, deadline, UdpSocket::bind(bind)).await?;
    guarded(auth, domain, port, deadline, socket.connect(target)).await?;
    let written = guarded(auth, domain, port, deadline, socket.send(&bytes)).await?;
    if written != bytes.len() {
        return Err(ResolveError::Io);
    }
    for _ in 0..MAX_PACKETS {
        let mut buffer = [0; MAX_WIRE + 1];
        let received = guarded(auth, domain, port, deadline, socket.recv(&mut buffer)).await?;
        if received > MAX_WIRE {
            return Err(ResolveError::InvalidResponse);
        }
        match chain.consume(&buffer[..received], id) {
            Ok(step) => return Ok(Some(step)),
            Err(ResponseError::NoAddress) => return Ok(None),
            Err(ResponseError::Mismatch) => continue,
            Err(ResponseError::Truncated) => return Err(ResolveError::Truncated),
            Err(_) => return Err(ResolveError::InvalidResponse),
        }
    }
    Err(ResolveError::InvalidResponse)
}

fn wire_query(chain: &ResponseChain, kind: QueryKind) -> Result<(u16, Vec<u8>)> {
    let name: Name = format!("{}.", chain.name())
        .parse()
        .map_err(|_| ResolveError::InvalidResponse)?;
    let mut query = Message::query(); // Qualified codec supplies a fresh random transaction ID.
    query.metadata.recursion_desired = true;
    query.add_query(Query::query(name, kind.record_type()));
    let id = query.metadata.id;
    let bytes = query.to_vec().map_err(|_| ResolveError::InvalidResponse)?;
    if bytes.len() > MAX_WIRE {
        return Err(ResolveError::InvalidResponse);
    }
    Ok((id, bytes))
}

async fn tcp_query(
    auth: &AuthorizedRequest<'_>,
    domain: &str,
    port: NonZeroU16,
    chain: &mut ResponseChain,
    kind: QueryKind,
    target: SocketAddr,
    deadline: Instant,
) -> Result<Option<ResponseStep>> {
    let (id, bytes) = wire_query(chain, kind)?;
    let mut frame = Vec::with_capacity(bytes.len() + 2);
    frame.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    frame.extend_from_slice(&bytes);
    let mut stream = guarded(auth, domain, port, deadline, TcpStream::connect(target)).await?;
    let mut operations = 0;
    let mut offset = 0;
    while offset < frame.len() {
        operations += 1;
        if operations > MAX_IO_OPS {
            return Err(ResolveError::InvalidResponse);
        }
        let count = guarded(auth, domain, port, deadline, stream.write(&frame[offset..])).await?;
        if count == 0 {
            return Err(ResolveError::Io);
        }
        offset += count;
    }
    let mut length = [0u8; 2];
    read_bounded(
        &mut stream,
        &mut length,
        &mut operations,
        auth,
        domain,
        port,
        deadline,
    )
    .await?;
    let length = u16::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_WIRE {
        return Err(ResolveError::InvalidResponse);
    }
    let mut response = vec![0u8; length];
    read_bounded(
        &mut stream,
        &mut response,
        &mut operations,
        auth,
        domain,
        port,
        deadline,
    )
    .await?;
    match chain.consume(&response, id) {
        Ok(step) => Ok(Some(step)),
        Err(ResponseError::NoAddress) => Ok(None),
        Err(_) => Err(ResolveError::InvalidResponse),
    }
}

async fn read_bounded(
    stream: &mut TcpStream,
    buffer: &mut [u8],
    operations: &mut usize,
    auth: &AuthorizedRequest<'_>,
    domain: &str,
    port: NonZeroU16,
    deadline: Instant,
) -> Result<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        *operations += 1;
        if *operations > MAX_IO_OPS {
            return Err(ResolveError::InvalidResponse);
        }
        let count = guarded(
            auth,
            domain,
            port,
            deadline,
            stream.read(&mut buffer[offset..]),
        )
        .await?;
        if count == 0 {
            return Err(ResolveError::Io);
        }
        offset += count;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use droutex_common::{ListenerId, OutboundAvailability, ProxyOutboundKind};
    use droutex_config::{
        CompletePublication, InputFormat, OutboundObservation, ReferenceCatalog, ResourceData,
        parse_configuration,
        snapshot::{SnapshotMetadata, SnapshotResource, SnapshotSequence},
    };
    use droutex_core::deployment::{Artifacts, Cancellation, Deployment};
    use droutex_policy::TcpCapability;
    use droutex_security::{SigningKey, SigningKeyId, SnapshotTrust};
    use hickory_proto::{
        op::{MessageType, OpCode},
        rr::{
            RData, Record,
            rdata::{A, CNAME},
        },
    };
    use serde_json::{Value, json};
    use std::{
        str::FromStr,
        time::{SystemTime, UNIX_EPOCH},
    };

    pub(crate) fn id<T: FromStr>(n: usize) -> T
    where
        T::Err: fmt::Debug,
    {
        format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
    }
    fn text(n: usize) -> String {
        id::<ListenerId>(n).to_string()
    }
    fn hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect()
    }
    pub(crate) fn keys() -> (SigningKey, SnapshotTrust) {
        let public = hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
            .try_into()
            .unwrap();
        let key = SigningKey::from_pkcs8(SigningKeyId::from_bytes([1;16]).unwrap(), hex("302e020100300506032b6570042204209d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"), &public).unwrap();
        let trust = SnapshotTrust::new(vec![(key.id(), public)]).unwrap();
        (key, trust)
    }
    pub(crate) fn document() -> Value {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        json!({"schemaVersion":2,"originUpstream":text(60),
            "profiles":[{"id":text(10),"name":"gateway","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"outbound","outbound":text(90)}}],
            "listeners":[{"id":text(22),"name":"gateway","enabled":true,"bind":"127.0.0.1:10443","transport":"gateway_tcp"}],
            "networkGroups":[{"id":text(50),"name":"test","trafficProfile":text(10)}],
            "gatewayGrants":[{"id":text(42),"source":"127.0.0.1","listener":text(22),"group":text(50),"targets":[{"mode":"exact","value":"service.test"}],"ports":[{"from":443,"to":443}],"outbounds":[text(90)],"lease":{"cause":"manual","issuedAt":now-10,"expiresAt":now+300}}],
            "outbounds":[{"id":text(90),"name":"direct","enabled":true,"transport":{"kind":"vps_direct"}}]})
    }
    const UPSTREAM: &[u8] = br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"udp_tcp","address":"1.1.1.1:53"}]}"#;
    pub(crate) fn apply(owner: &Deployment, doc: &Value, seq: u64) {
        let config = parse_configuration(
            &serde_json::to_vec(doc).unwrap(),
            InputFormat::Json,
            &ReferenceCatalog {
                upstreams: [id(60)].into(),
                ..Default::default()
            },
        )
        .unwrap();
        let resource = ResourceData {
            resource: SnapshotResource::Upstream(id(60)),
            bytes: UPSTREAM,
        };
        let publication = CompletePublication::new(config, vec![], &[resource], &[]).unwrap();
        let bytes = publication
            .encode(
                SnapshotMetadata {
                    node: id(1),
                    snapshot: id(2),
                    source: id(3),
                    sequence: SnapshotSequence::new(seq).unwrap(),
                },
                &keys().0,
            )
            .unwrap();
        owner
            .activate(
                bytes,
                Artifacts::new(vec![(
                    SnapshotResource::Upstream(id(60)),
                    UPSTREAM.to_vec(),
                )])
                .unwrap(),
                &Cancellation::default(),
            )
            .unwrap();
    }
    fn observations() -> Vec<OutboundObservation> {
        vec![OutboundObservation {
            id: id(90),
            kind: ProxyOutboundKind::GatewayDirect,
            availability: OutboundAvailability::Available,
            tcp: TcpCapability::Supported,
        }]
    }
    async fn admitted(owner: &Deployment) -> droutex_core::deployment::AuthorizedRequest<'_> {
        admitted_with(owner, observations()).await
    }
    async fn admitted_with(
        owner: &Deployment,
        facts: Vec<OutboundObservation>,
    ) -> droutex_core::deployment::AuthorizedRequest<'_> {
        use droutex_core::deployment::{AdmissionCode, AdmissionConsumption};
        let limit = Instant::now() + Duration::from_secs(3);
        let mut attempt = loop {
            assert!(
                Instant::now() < limit,
                "gateway fixture admission timed out"
            );
            match owner.prepare_gateway(
                id(22),
                "127.0.0.1".parse().unwrap(),
                "service.test",
                NonZeroU16::new(443).unwrap(),
                facts.clone(),
            ) {
                Ok(attempt) => break attempt,
                Err(error) if error.code() == AdmissionCode::CheckpointRequired => owner
                    .refresh_authorization_clock(&Cancellation::default())
                    .unwrap(),
                Err(error) if error.code() == AdmissionCode::Quota => {
                    tokio::time::sleep(Duration::from_millis(10)).await
                }
                Err(error) => panic!("gateway fixture admission: {error}"),
            }
        };
        loop {
            assert!(
                Instant::now() < limit,
                "gateway fixture consumption timed out"
            );
            match attempt.try_consume() {
                AdmissionConsumption::Authorized(auth) => return auth,
                AdmissionConsumption::Refused {
                    attempt: retained,
                    error,
                } if error.code() == AdmissionCode::CheckpointRequired => {
                    owner
                        .refresh_authorization_clock(&Cancellation::default())
                        .unwrap();
                    attempt = retained;
                }
                AdmissionConsumption::Refused { error, .. } => {
                    panic!("gateway fixture consumption: {error}")
                }
            }
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fixture_admission_refreshes_a_due_clock_checkpoint() {
        use droutex_core::deployment::AdmissionCode;
        let owner = Deployment::in_memory(id(1), keys().1);
        apply(&owner, &document(), 1);
        let limit = Instant::now() + Duration::from_secs(8);
        loop {
            assert!(
                Instant::now() < limit,
                "clock checkpoint did not become due"
            );
            match owner.prepare_gateway(
                id(22),
                "127.0.0.1".parse().unwrap(),
                "service.test",
                NonZeroU16::new(443).unwrap(),
                observations(),
            ) {
                Err(error) if error.code() == AdmissionCode::CheckpointRequired => break,
                Err(error) if error.code() == AdmissionCode::Quota => (),
                Ok(attempt) => drop(attempt),
                Err(error) => panic!("unexpected admission: {error}"),
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let auth = admitted(&owner).await;
        auth.verify_gateway_destination("service.test", NonZeroU16::new(443).unwrap())
            .unwrap();
    }
    fn fixture(address: SocketAddr) -> OriginResolver {
        assert!(address.ip().is_loopback() && address.port() != 0);
        let mut resolver = OriginResolver::new(vec![]).unwrap();
        resolver.fixture = Some(address);
        resolver
    }
    async fn serve_dns(socket: UdpSocket, answer_ip: Ipv4Addr, requests: usize) {
        let mut buf = [0; MAX_WIRE];
        for _ in 0..requests {
            let (size, peer) = socket.recv_from(&mut buf).await.unwrap();
            let query = Message::from_vec(&buf[..size]).unwrap();
            let question = query.queries[0].clone();
            let mut reply = Message::new(query.metadata.id, MessageType::Response, OpCode::Query);
            reply.add_query(question.clone());
            if question.query_type == hickory_proto::rr::RecordType::A {
                if question
                    .name
                    .to_ascii()
                    .eq_ignore_ascii_case("service.test.")
                {
                    reply.add_answer(Record::from_rdata(
                        question.name.clone(),
                        30,
                        RData::CNAME(CNAME("cdn.test.".parse().unwrap())),
                    ));
                } else {
                    reply.add_answer(Record::from_rdata(
                        question.name.clone(),
                        30,
                        RData::A(A(answer_ip)),
                    ));
                }
            }
            socket
                .send_to(&reply.to_vec().unwrap(), peer)
                .await
                .unwrap();
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn real_udp_resolution_uses_signed_origin_group_and_screens_result() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let server = tokio::spawn(serve_dns(socket, "8.8.8.8".parse().unwrap(), 3));
        let owner = Deployment::in_memory(id(1), keys().1);
        let doc = document();
        apply(&owner, &doc, 1);
        let auth = admitted(&owner).await;
        let result = fixture(address)
            .resolve(&auth, Instant::now() + Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(result.addresses(), &["8.8.8.8".parse::<IpAddr>().unwrap()]);
        assert_eq!(result.port().get(), 443);
        result.verify_current().unwrap();
        let mut revoked = doc.clone();
        revoked["gatewayGrants"][0]["source"] = json!("127.0.0.2");
        apply(&owner, &revoked, 2);
        assert_eq!(
            result.verify_current().unwrap_err(),
            ResolveError::Admission
        );
        timeout_at(Instant::now() + Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn real_udp_private_rebinding_answer_is_denied() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let server = tokio::spawn(serve_dns(socket, "10.0.0.1".parse().unwrap(), 3));
        let owner = Deployment::in_memory(id(1), keys().1);
        apply(&owner, &document(), 1);
        let auth = admitted(&owner).await;
        assert!(matches!(
            fixture(address)
                .resolve(&auth, Instant::now() + Duration::from_secs(5))
                .await,
            Err(ResolveError::UnsafeAddress)
        ));
        timeout_at(Instant::now() + Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn revoked_generation_cannot_complete_pending_dns_query() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let (received_tx, received_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let mut bytes = [0; MAX_WIRE];
            let (size, peer) = socket.recv_from(&mut bytes).await.unwrap();
            let request = Message::from_vec(&bytes[..size]).unwrap();
            let _ = received_tx.send(());
            let _ = release_rx.await;
            let mut response =
                Message::new(request.metadata.id, MessageType::Response, OpCode::Query);
            response.add_query(request.queries[0].clone());
            response.add_answer(Record::from_rdata(
                request.queries[0].name.clone(),
                30,
                RData::A(A("8.8.8.8".parse().unwrap())),
            ));
            socket
                .send_to(&response.to_vec().unwrap(), peer)
                .await
                .unwrap();
        });
        let owner = Deployment::in_memory(id(1), keys().1);
        let doc = document();
        apply(&owner, &doc, 1);
        let auth = admitted(&owner).await;
        let resolver = fixture(address);
        let change = async {
            timeout_at(Instant::now() + Duration::from_secs(2), received_rx)
                .await
                .unwrap()
                .unwrap();
            let mut revoked = doc.clone();
            revoked["gatewayGrants"][0]["source"] = json!("127.0.0.2");
            apply(&owner, &revoked, 2);
            release_tx.send(()).unwrap();
        };
        let (result, ()) = tokio::join!(
            resolver.resolve(&auth, Instant::now() + Duration::from_secs(5)),
            change
        );
        assert!(matches!(result, Err(ResolveError::Admission)));
        timeout_at(Instant::now() + Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap();
    }
    async fn paired_loopback() -> io::Result<(tokio::net::TcpListener, UdpSocket)> {
        tokio::time::timeout(Duration::from_secs(5), async {
            let mut last =
                io::Error::new(io::ErrorKind::AddrNotAvailable, "no shared TCP/UDP port");
            for _ in 0..64 {
                // Windows can reserve a port for one protocol but not the other.
                let udp = UdpSocket::bind("127.0.0.1:0").await?;
                let address = udp.local_addr()?;
                match tokio::net::TcpListener::bind(address).await {
                    Ok(tcp) => return Ok((tcp, udp)),
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::AddrInUse | io::ErrorKind::PermissionDenied
                        ) =>
                    {
                        last = error
                    }
                    Err(error) => return Err(error),
                }
            }
            Err(last)
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "TCP/UDP fixture setup timed out"))?
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn truncated_udp_falls_back_to_framed_tcp() {
        let (tcp, udp) = paired_loopback().await.unwrap();
        let address = tcp.local_addr().unwrap();
        let udp_server = tokio::spawn(async move {
            let mut bytes = [0; MAX_WIRE];
            for index in 0..2 {
                let (size, peer) = udp.recv_from(&mut bytes).await.unwrap();
                let request = Message::from_vec(&bytes[..size]).unwrap();
                let mut response =
                    Message::new(request.metadata.id, MessageType::Response, OpCode::Query);
                response.add_query(request.queries[0].clone());
                response.metadata.truncation = index == 0;
                udp.send_to(&response.to_vec().unwrap(), peer)
                    .await
                    .unwrap();
            }
        });
        let tcp_server = tokio::spawn(async move {
            let (mut stream, _) = tcp.accept().await.unwrap();
            let len = stream.read_u16().await.unwrap() as usize;
            let mut bytes = vec![0; len];
            stream.read_exact(&mut bytes).await.unwrap();
            let request = Message::from_vec(&bytes).unwrap();
            let mut response =
                Message::new(request.metadata.id, MessageType::Response, OpCode::Query);
            response.add_query(request.queries[0].clone());
            response.add_answer(Record::from_rdata(
                request.queries[0].name.clone(),
                30,
                RData::A(A("8.8.8.8".parse().unwrap())),
            ));
            let bytes = response.to_vec().unwrap();
            stream.write_u16(bytes.len() as u16).await.unwrap();
            stream.write_all(&bytes).await.unwrap();
        });
        let owner = Deployment::in_memory(id(1), keys().1);
        apply(&owner, &document(), 1);
        let auth = admitted(&owner).await;
        let resolver = fixture(address);
        let result = resolver
            .resolve(&auth, Instant::now() + Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(result.addresses(), &["8.8.8.8".parse::<IpAddr>().unwrap()]);
        timeout_at(Instant::now() + Duration::from_secs(2), udp_server)
            .await
            .unwrap()
            .unwrap();
        timeout_at(Instant::now() + Duration::from_secs(2), tcp_server)
            .await
            .unwrap()
            .unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn direct_connect_rejects_unchecked_candidates_and_revoked_admission() {
        let owner = Deployment::in_memory(id(1), keys().1);
        let doc = document();
        apply(&owner, &doc, 1);
        let auth = admitted(&owner).await;
        // Constructed only inside this module to exercise refusal before any dial.
        let origin = || ResolvedOrigin {
            owner: &auth,
            domain: "service.test".to_owned(),
            addresses: vec!["127.0.0.1".parse().unwrap()],
            port: NonZeroU16::new(443).unwrap(),
            aliases: vec![],
            deadline: Instant::now() + TOTAL,
            registry: Arc::new(Registry::new()),
        };
        assert!(matches!(
            origin().connect_direct(1, Instant::now() + TOTAL).await,
            Err(ResolveError::InvalidCandidate)
        ));
        assert!(matches!(
            origin().connect_direct(0, Instant::now() + TOTAL).await,
            Err(ResolveError::UnsafeAddress)
        ));
        assert!(matches!(
            origin().connect_direct(0, Instant::now()).await,
            Err(ResolveError::Timeout)
        ));
        let mut stale = origin();
        stale.deadline = Instant::now();
        assert_eq!(stale.verify_current(), Err(ResolveError::Timeout));
        assert!(matches!(
            stale.connect_direct(0, Instant::now() + TOTAL).await,
            Err(ResolveError::Timeout)
        ));
        let mut revoked = doc;
        revoked["gatewayGrants"][0]["source"] = json!("127.0.0.2");
        apply(&owner, &revoked, 2);
        assert!(matches!(
            origin().connect_direct(0, Instant::now() + TOTAL).await,
            Err(ResolveError::Admission)
        ));
        let polled = std::cell::Cell::new(false);
        let operation = poll_fn(|_| {
            polled.set(true);
            Poll::Ready(Ok::<_, io::Error>(()))
        });
        assert_eq!(
            guarded(
                &auth,
                "service.test",
                NonZeroU16::new(443).unwrap(),
                Instant::now() + TOTAL,
                operation
            )
            .await,
            Err(ResolveError::Admission)
        );
        assert!(
            !polled.get(),
            "revoked authorization must prevent even the first I/O poll"
        );
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn protected_proxy_policy_never_dials_direct() {
        let owner = Deployment::in_memory(id(1), keys().1);
        let mut doc = document();
        doc["outbounds"][0]["transport"] = json!({"kind":"http_connect_tls", "host":"proxy.test", "port":443, "tlsServerName":"proxy.test"});
        apply(&owner, &doc, 1);
        let auth = admitted_with(
            &owner,
            vec![OutboundObservation {
                id: id(90),
                kind: ProxyOutboundKind::HttpConnectTls,
                availability: OutboundAvailability::Available,
                tcp: TcpCapability::Supported,
            }],
        )
        .await;
        let origin = ResolvedOrigin {
            owner: &auth,
            domain: "service.test".to_owned(),
            addresses: vec!["8.8.8.8".parse().unwrap()],
            port: NonZeroU16::new(443).unwrap(),
            aliases: vec![],
            deadline: Instant::now() + TOTAL,
            registry: Arc::new(Registry::new()),
        };
        assert!(matches!(
            origin.connect_direct(0, Instant::now() + TOTAL).await,
            Err(ResolveError::UnsupportedOutbound)
        ));
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn direct_dial_uses_selected_numeric_target_and_owns_socket() {
        let dns = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let dns_address = dns.local_addr().unwrap();
        let dns_server = tokio::spawn(serve_dns(dns, "8.8.8.8".parse().unwrap(), 3));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let listener_address = listener.local_addr().unwrap();
        let owner = Deployment::in_memory(id(1), keys().1);
        apply(&owner, &document(), 1);
        let auth = admitted(&owner).await;
        let resolver = fixture(dns_address);
        let result = resolver
            .resolve(&auth, Instant::now() + TOTAL)
            .await
            .unwrap();
        let connection = result
            .connect_direct_with(0, Instant::now() + TOTAL, |target| async move {
                assert_eq!(target, "8.8.8.8:443".parse::<SocketAddr>().unwrap());
                // Test-only transport: the production closure connects `target` itself.
                TcpStream::connect(listener_address).await
            })
            .await
            .unwrap();
        let (mut peer, _) = timeout_at(Instant::now() + TOTAL, listener.accept())
            .await
            .unwrap()
            .unwrap();
        drop(connection);
        let mut byte = [0u8; 1];
        assert_eq!(
            timeout_at(Instant::now() + TOTAL, peer.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
        timeout_at(Instant::now() + TOTAL, dns_server)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn revocation_during_dial_drops_connected_socket_before_returning() {
        let dns = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let dns_address = dns.local_addr().unwrap();
        let dns_server = tokio::spawn(serve_dns(dns, "8.8.8.8".parse().unwrap(), 3));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let listener_address = listener.local_addr().unwrap();
        let owner = Deployment::in_memory(id(1), keys().1);
        let doc = document();
        apply(&owner, &doc, 1);
        let auth = admitted(&owner).await;
        let resolver = fixture(dns_address);
        let result = resolver
            .resolve(&auth, Instant::now() + TOTAL)
            .await
            .unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let connect = result.connect_direct_with(0, Instant::now() + TOTAL, |target| async move {
            assert_eq!(target, "8.8.8.8:443".parse::<SocketAddr>().unwrap());
            let stream = TcpStream::connect(listener_address).await?;
            let _ = started_tx.send(());
            let _ = release_rx.await;
            Ok(stream)
        });
        let change = async {
            timeout_at(Instant::now() + TOTAL, started_rx)
                .await
                .unwrap()
                .unwrap();
            let (mut peer, _) = timeout_at(Instant::now() + TOTAL, listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut revoked = doc;
            revoked["gatewayGrants"][0]["source"] = json!("127.0.0.2");
            apply(&owner, &revoked, 2);
            release_tx.send(()).unwrap();
            let mut byte = [0u8; 1];
            assert_eq!(
                timeout_at(Instant::now() + TOTAL, peer.read(&mut byte))
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );
        };
        let (answer, ()) = tokio::join!(connect, change);
        assert!(matches!(answer, Err(ResolveError::Admission)));
        timeout_at(Instant::now() + TOTAL, dns_server)
            .await
            .unwrap()
            .unwrap();
    }
}
