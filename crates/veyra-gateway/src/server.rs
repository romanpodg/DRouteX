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

/// Connections served at once; beyond this, new ones are closed at once
/// (spec §10.1 limits). Each holds two sockets: keep it under the file limit.
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
        Ok(Listeners {
            bound,
            max_sessions: MAX_SESSIONS,
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
