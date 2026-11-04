//! Fail-closed listener lifecycle; no TLS processing or readiness is exposed.
use crate::origin::resolver::OriginResolver;
use droutex_common::ListenerId;
use droutex_config::ListenerTransport;
use droutex_core::deployment::{Decision, Deployment};
use std::{collections::BTreeSet, error::Error, fmt, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::watch, task::JoinSet, time::timeout};

const MAX_LISTENERS: usize = 16;
const CLEANUP: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListenerError {
    Configuration,
    Unavailable,
    Io,
    Cleanup,
}
impl fmt::Display for ListenerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gateway listener {self:?}")
    }
}
impl Error for ListenerError {}
type Result<T> = std::result::Result<T, ListenerError>;

/// A trusted host-owned listening socket and its signed listener identity.
pub struct ListenerSocket {
    pub id: ListenerId,
    pub socket: TcpListener,
}
#[derive(Clone, Copy)]
struct Binding {
    id: ListenerId,
    local: SocketAddr,
}
impl Binding {
    fn check(self, decision: &Decision<'_>) -> Result<()> {
        let document = decision.gateway_configuration().document();
        if self.local.ip().is_unspecified()
            || document.schema_version != 2
            || !document.listeners.iter().any(|listener| {
                listener.id == self.id
                    && listener.enabled
                    && listener.bind == self.local
                    && listener.transport == ListenerTransport::GatewayTcp
            })
        {
            return Err(ListenerError::Configuration);
        }
        Ok(())
    }
}

/// Serve a bounded set of configured sockets until shutdown. Until TLS, current
/// admission and passthrough exist, every accepted stream is immediately closed.
/// This function deliberately does not establish SmartDNS gateway readiness.
/// The host must retain and poll this future, and signal shutdown before dropping it.
pub async fn serve_closed(
    deployment: Arc<Deployment>,
    sockets: Vec<ListenerSocket>,
    mut shutdown: watch::Receiver<bool>,
    resolver: Arc<OriginResolver>,
) -> Result<()> {
    if sockets.is_empty() || sockets.len() > MAX_LISTENERS {
        return Err(ListenerError::Configuration);
    }
    let current = deployment.pin().map_err(|_| ListenerError::Unavailable)?;
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
        binding.check(&current)?;
        bindings.push((binding, listener.socket));
    }
    drop(current);
    let mut tasks = JoinSet::new();
    for (binding, socket) in bindings {
        let deployment = Arc::clone(&deployment);
        let mut stop = shutdown.clone();
        tasks.spawn(async move {
            loop {
                if *stop.borrow() { return Ok(()); }
                tokio::select! {
                    changed = stop.changed() => if changed.is_err() || *stop.borrow_and_update() { return Ok(()); },
                    accepted = socket.accept() => {
                        let (stream, _) = accepted.map_err(|_| ListenerError::Io)?;
                        // Even if the socket survived a publication, only its current
                        // exact listener identity and address may remain active.
                        let current = deployment.pin().map_err(|_| ListenerError::Unavailable)?;
                        binding.check(&current)?;
                        drop(stream);
                    }
                }
            }
        });
    }
    let outcome = loop {
        if *shutdown.borrow() {
            break Ok(());
        }
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow_and_update() { break Ok(()); }
            },
            result = tasks.join_next() => break match result {
                Some(Ok(Err(error))) => Err(error),
                Some(Ok(Ok(()))) if *shutdown.borrow() || shutdown.has_changed().is_err() => Ok(()),
                _ => Err(ListenerError::Unavailable),
            },
        }
    };
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    timeout(CLEANUP, resolver.shutdown_inventory())
        .await
        .map_err(|_| ListenerError::Cleanup)?
        .map_err(|_| ListenerError::Cleanup)?;
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::origin::resolver::tests::{apply, document, id, keys};
    use serde_json::json;
    use tokio::{io::AsyncReadExt, net::TcpStream};

    async fn setup() -> (Arc<Deployment>, TcpListener, Arc<OriginResolver>) {
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut doc = document();
        doc["listeners"][0]["bind"] = json!(socket.local_addr().unwrap().to_string());
        let owner = Arc::new(Deployment::in_memory(id(1), keys().1));
        apply(&owner, &doc, 1);
        (
            owner,
            socket,
            Arc::new(OriginResolver::new(vec![]).unwrap()),
        )
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn configured_listener_refuses_traffic_and_shutdown_joins_tasks() {
        let (owner, socket, resolver) = setup().await;
        let address = socket.local_addr().unwrap();
        let (stop, receiver) = watch::channel(false);
        let server = tokio::spawn(serve_closed(
            owner,
            vec![ListenerSocket { id: id(22), socket }],
            receiver,
            resolver,
        ));
        let mut client = TcpStream::connect(address).await.unwrap();
        let mut byte = [0u8; 1];
        assert_eq!(
            timeout(Duration::from_secs(2), client.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
        stop.send(false).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(
            !server.is_finished(),
            "a false notification cannot stop the listener"
        );
        let mut next = TcpStream::connect(address).await.unwrap();
        assert_eq!(
            timeout(Duration::from_secs(2), next.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
        stop.send(true).unwrap();
        timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mismatched_listener_is_refused_before_serving() {
        let (owner, socket, resolver) = setup().await;
        let (_stop, receiver) = watch::channel(false);
        assert_eq!(
            timeout(
                Duration::from_secs(2),
                serve_closed(
                    owner,
                    vec![ListenerSocket { id: id(23), socket }],
                    receiver,
                    resolver
                )
            )
            .await
            .unwrap(),
            Err(ListenerError::Configuration)
        );
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn changed_listener_configuration_stops_old_socket() {
        let (owner, socket, resolver) = setup().await;
        let address = socket.local_addr().unwrap();
        let (_stop, receiver) = watch::channel(false);
        let server = tokio::spawn(serve_closed(
            Arc::clone(&owner),
            vec![ListenerSocket { id: id(22), socket }],
            receiver,
            resolver,
        ));
        let mut initial = TcpStream::connect(address).await.unwrap();
        let mut byte = [0u8; 1];
        assert_eq!(
            timeout(Duration::from_secs(2), initial.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
        let mut changed = document();
        changed["listeners"][0]["bind"] = json!("127.0.0.1:10444");
        apply(&owner, &changed, 2);
        let _client = TcpStream::connect(address).await.unwrap();
        assert_eq!(
            timeout(Duration::from_secs(3), server)
                .await
                .unwrap()
                .unwrap(),
            Err(ListenerError::Configuration)
        );
    }
}
