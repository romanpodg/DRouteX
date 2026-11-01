//! D47 transport, D53 bounded policy-selected forwarding and D54 scoped caching.
mod budget;
mod cache;
mod clock;
mod forward;
mod policy;
mod runtime;
mod wire;

use droutex_common::ListenerId;
use droutex_core::deployment::Deployment;
use std::{error::Error, fmt, sync::Arc};
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::watch,
};

pub const MAX_LISTENERS: usize = 16;
pub const MAX_UDP_BYTES: usize = 1232;
pub const MAX_TCP_BYTES: usize = u16::MAX as usize;
pub const MAX_WIRE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    Configuration,
    Unavailable,
    Malformed,
    Scope,
    Admission,
    Io,
    State,
    Timeout,
    ShutdownIncomplete,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DnsError(ErrorCode);
impl DnsError {
    pub const fn code(self) -> ErrorCode {
        self.0
    }
}
impl fmt::Display for DnsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DNS {:?}", self.0)
    }
}
impl Error for DnsError {}
type Result<T> = std::result::Result<T, DnsError>;

/// Explicit prebound socket plus configured identity. Runtime checks its actual
/// local address/transport against the current signed configuration per request.
/// Constructing this data supplies no peer admission or alternate Profile.
pub enum ListenerSocket {
    Udp { id: ListenerId, socket: UdpSocket },
    Tcp { id: ListenerId, socket: TcpListener },
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShutdownReport {
    pub answered: u64,
    pub denied: u64,
    pub malformed: u64,
    pub timed_out: u64,
    pub clock_failures: u64,
}

/// One DNS runtime owns a Deployment value. Sharing its Arc with CP/gateway
/// consumers does not allow constructing another DNS owner from the same value.
/// The node-global budgets survive all listeners and applied generations.
pub struct DnsNode {
    deployment: Arc<Deployment>,
}
impl DnsNode {
    pub fn new(deployment: Deployment) -> Self {
        Self {
            deployment: Arc::new(deployment),
        }
    }
    pub fn deployment(&self) -> &Arc<Deployment> {
        &self.deployment
    }
    /// Runs on the caller's Tokio runtime. No bind or runtime starts implicitly.
    /// Signal shutdown through the watch channel and await this future for a clean
    /// result. Dropping/aborting it cannot claim graceful or durable completion.
    pub async fn serve(
        self,
        sockets: Vec<ListenerSocket>,
        shutdown: watch::Receiver<bool>,
    ) -> Result<ShutdownReport> {
        self.serve_with_aliases(sockets, Vec::new(), shutdown).await
    }
    /// Trusted operator-declared public/NAT/service aliases supplement the fresh
    /// OS inventory. Aliases supply no client, Profile or destination authority.
    pub async fn serve_with_aliases(
        self,
        sockets: Vec<ListenerSocket>,
        aliases: Vec<std::net::IpAddr>,
        shutdown: watch::Receiver<bool>,
    ) -> Result<ShutdownReport> {
        runtime::serve(
            self.deployment,
            sockets,
            shutdown,
            forward::Forwarder::new(aliases)?,
        )
        .await
    }
}

#[cfg(test)]
mod tests;
