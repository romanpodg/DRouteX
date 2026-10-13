//! One client connection (spec §6.2, §10.1): admit the source, read the
//! destination, check its route, resolve it, check every address, connect,
//! then pass bytes both ways untouched.

use crate::egress::Egress;
use crate::sniff::{Service, Sniff, sniff};
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
use tokio::net::TcpStream;
use tokio::time::timeout;
use veyra_dns::lookup;
use veyra_policy::{Cidr, DEFAULT_UPSTREAM_GROUP, Name, Policy};

/// How long a client may take to name its destination.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
/// How long each upstream resolver may take to answer.
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(2);
/// How long each origin address may take to accept.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Why a client was turned away before reaching an origin. The codes of spec
/// §20.4 where one fits.
#[derive(Debug, thiserror::Error)]
pub enum Refusal {
    /// The source is not admitted (spec §9.6, REQ-GW-004).
    #[error("SOURCE_NOT_ADMITTED")]
    SourceNotAdmitted,
    /// No SNI or `Host`, or one that is not a domain name.
    #[error("no destination name")]
    NoName,
    /// DNS never sends clients here for this name in mode B (spec §10.2).
    #[error("not routed to this node")]
    NotRouted,
    #[error("origin does not resolve")]
    Unresolved,
    /// Every address the name resolved to is forbidden (spec §19.2 item 4).
    #[error("EGRESS_BLOCKED")]
    EgressBlocked,
    #[error("origin unreachable")]
    Unreachable,
    /// The client was too slow to name its destination, or the socket failed.
    #[error("client: {0}")]
    Client(#[from] io::Error),
}

/// The gateway of one node: which names it passes and how it reaches origins.
pub struct Gateway {
    policy: Policy,
    node: String,
    resolvers: Vec<SocketAddr>,
    egress: Egress,
    tls_port: u16,
    http_port: u16,
    hello_timeout: Duration,
    upstream_timeout: Duration,
}

impl Gateway {
    /// Fails when the policy has no `gateway:` section.
    pub fn new(policy: Policy) -> Result<Gateway, String> {
        let node = policy
            .gateway_config()
            .ok_or("the config has no gateway section")?
            .node
            .clone();
        // Origins resolve through the default upstream group, never through
        // Veyra's own DNS, which would answer with a gateway address (spec §10.3).
        let resolvers = policy
            .dns()
            .and_then(|dns| dns.upstreams.get(DEFAULT_UPSTREAM_GROUP))
            .cloned()
            .unwrap_or_default();
        let egress = Egress::new(&policy);
        Ok(Gateway {
            policy,
            node,
            resolvers,
            egress,
            tls_port: 443,
            http_port: 80,
            hello_timeout: HELLO_TIMEOUT,
            upstream_timeout: UPSTREAM_TIMEOUT,
        })
    }

    /// Dial origins on other ports than 443 and 80, such as test origins.
    pub fn with_origin_ports(mut self, tls: u16, http: u16) -> Gateway {
        self.tls_port = tls;
        self.http_port = http;
        self
    }

    /// How long a client may take to name its destination, and how long each
    /// upstream resolver may take to answer.
    pub fn with_timeouts(mut self, hello: Duration, upstream: Duration) -> Gateway {
        self.hello_timeout = hello;
        self.upstream_timeout = upstream;
        self
    }

    /// Let the gateway dial `network` despite the egress rules, such as a test
    /// origin on loopback.
    pub fn allowing(mut self, network: Cidr) -> Gateway {
        self.egress.allow(network);
        self
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Serve one connection to the end. `Ok` once bytes flowed to an origin
    /// and both sides closed; `Err` says why the client was turned away.
    pub async fn handle(&self, mut client: TcpStream, service: Service) -> Result<(), Refusal> {
        // Not admitted: close before reading a byte (spec §9.6).
        if self.policy.admit(client.peer_addr()?.ip()).is_none() {
            return Err(Refusal::SourceNotAdmitted);
        }
        let (head, name) = timeout(self.hello_timeout, read_head(&mut client, service))
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
        if !self.policy.routes_to(&self.node, &name) {
            return Err(Refusal::NotRouted);
        }
        let addrs = lookup(&self.resolvers, name.as_str(), self.upstream_timeout).await;
        if addrs.is_empty() {
            return Err(Refusal::Unresolved);
        }
        let allowed: Vec<IpAddr> = addrs
            .into_iter()
            .filter(|ip| self.egress.allows(*ip))
            .collect();
        if allowed.is_empty() {
            return Err(Refusal::EgressBlocked);
        }
        let port = match service {
            Service::Tls => self.tls_port,
            Service::Http => self.http_port,
        };
        let mut origin = connect(&allowed, port).await.ok_or(Refusal::Unreachable)?;
        origin
            .write_all(&head)
            .await
            .map_err(|_| Refusal::Unreachable)?;
        let _ = client.set_nodelay(true);
        let _ = origin.set_nodelay(true);
        // A close or reset on either side ends the passthrough; it is not a refusal.
        // Later requests on a kept-alive HTTP connection are not read: they go to
        // the origin address the first `Host` chose.
        // ponytail: no idle timeout once bytes flow; add one if stale sessions
        // fill the session cap.
        let _ = copy_bidirectional(&mut client, &mut origin).await;
        Ok(())
    }
}

/// Read until the ClientHello or request head names the destination. Returns
/// every byte read, to be sent to the origin unchanged.
async fn read_head(client: &mut TcpStream, service: Service) -> Result<(Vec<u8>, Name), Refusal> {
    let mut head = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let len = client.read(&mut buf).await?;
        if len == 0 {
            return Err(Refusal::NoName);
        }
        head.extend_from_slice(&buf[..len]);
        match sniff(service, &head) {
            Sniff::More => {}
            Sniff::Name(name) => return Ok((head, name)),
            Sniff::Refuse => return Err(Refusal::NoName),
        }
    }
}

/// The first address that accepts, in resolver order (IPv4 first).
// ponytail: one address after another; Happy Eyeballs (RFC 8305) if
// dual-stack origins with a dead family stall connections.
async fn connect(addrs: &[IpAddr], port: u16) -> Option<TcpStream> {
    for ip in addrs {
        if let Ok(Ok(stream)) = timeout(CONNECT_TIMEOUT, TcpStream::connect((*ip, port))).await {
            return Some(stream);
        }
    }
    None
}
