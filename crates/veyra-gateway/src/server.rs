//! TCP listeners for the gateway (spec §10.1): accept, cap, hand off.

use crate::gateway::Gateway;
use crate::sniff::Service;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::sleep;
use veyra_policy::GatewayConfig;

/// Connections served at once when the open-file limit is unknown (not Linux);
/// on Linux the cap follows the limit (`session_cap`). Beyond the cap, new
/// connections are closed at once (spec §10.1 limits).
const MAX_SESSIONS: usize = 1024;

/// Bound sockets. Binding first lets the caller learn the real addresses
/// (for `127.0.0.1:0`) before serving.
pub struct Listeners {
    bound: Vec<(TcpListener, Service)>,
    max_sessions: usize,
}

impl Listeners {
    /// TCP only. UDP on the same ports stays closed, so an HTTP/3 client gets
    /// ICMP port unreachable and falls back to TCP at once (spec §10.1).
    pub async fn bind(config: &GatewayConfig) -> io::Result<Listeners> {
        let tls = config.tls.iter().map(|a| (a, Service::Tls));
        let http = config.http.iter().map(|a| (a, Service::Http));
        let mut bound = Vec::new();
        for (addr, service) in tls.chain(http) {
            bound.push((TcpListener::bind(addr).await?, service));
        }
        let limits = std::fs::read_to_string("/proc/self/limits").ok();
        Ok(Listeners {
            bound,
            max_sessions: session_cap(limits.as_deref().and_then(soft_file_limit)),
        })
    }

    /// Serve at most `n` connections at once.
    pub fn with_max_sessions(mut self, n: usize) -> Listeners {
        self.max_sessions = n;
        self
    }

    /// The bound addresses: TLS, then HTTP, each in `gateway:` order.
    pub fn local_addrs(&self) -> io::Result<Vec<(SocketAddr, Service)>> {
        self.bound
            .iter()
            .map(|(listener, service)| Ok((listener.local_addr()?, *service)))
            .collect()
    }

    /// Serve until the future is dropped.
    pub async fn serve(self, gateway: Arc<Gateway>) {
        let permits = Arc::new(Semaphore::new(self.max_sessions));
        let mut tasks = JoinSet::new();
        for (listener, service) in self.bound {
            tasks.spawn(serve_listener(
                listener,
                service,
                gateway.clone(),
                permits.clone(),
            ));
        }
        while tasks.join_next().await.is_some() {}
    }
}

async fn serve_listener(
    listener: TcpListener,
    service: Service,
    gateway: Arc<Gateway>,
    permits: Arc<Semaphore>,
) {
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            Err(e) => {
                // Out of file descriptors and similar: back off instead of spinning.
                tracing::warn!(error = %e, "accept failed");
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        // At capacity: close at once rather than queue.
        let Ok(permit) = permits.clone().try_acquire_owned() else {
            continue;
        };
        let gateway = gateway.clone();
        tokio::spawn(async move {
            // The reason only: no names or addresses in logs (spec §19.3).
            if let Err(reason) = gateway.handle(stream, service).await {
                tracing::debug!(%reason, ?service, "connection refused");
            }
            drop(permit);
        });
    }
}

/// Sessions the open-file limit can carry. A session holds two TCP sockets,
/// and two UDP sockets while it resolves its origin; DNS in the same process
/// keeps the other half of the limit. `MAX_SESSIONS` when the limit is unknown.
fn session_cap(file_limit: Option<u64>) -> usize {
    file_limit.map_or(MAX_SESSIONS, |limit| {
        usize::try_from(limit / 8).unwrap_or(usize::MAX).max(1)
    })
}

/// The soft "Max open files" limit from Linux's `/proc/self/limits`.
fn soft_file_limit(limits: &str) -> Option<u64> {
    let line = limits.lines().find(|l| l.starts_with("Max open files"))?;
    line.split_whitespace().nth(3)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: &str = "\
Limit                     Soft Limit           Hard Limit           Units     
Max cpu time              unlimited            unlimited            seconds   
Max open files            1024                 524288               files     
";

    #[test]
    fn the_session_cap_fits_the_open_file_limit() {
        assert_eq!(soft_file_limit(LIMITS), Some(1024));
        assert_eq!(
            session_cap(Some(1024)),
            128,
            "systemd's default soft limit for a service"
        );
        assert_eq!(session_cap(Some(65_536)), 8192);
        assert_eq!(session_cap(Some(4)), 1);
        assert_eq!(session_cap(None), MAX_SESSIONS, "no /proc: not Linux");
        let unlimited =
            "Max open files            unlimited            unlimited            files\n";
        assert_eq!(soft_file_limit(unlimited), None);
    }
}
