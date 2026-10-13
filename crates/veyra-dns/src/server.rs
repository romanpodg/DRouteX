//! UDP and TCP listeners for the engine (spec §9.1, §9.6).

use crate::engine::{Engine, Transport};
use crate::upstream::{read_frame, write_frame};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};

/// Queries in progress at once. Beyond this, UDP queries are dropped and TCP
/// queries wait (spec §19.2: limits).
const MAX_IN_FLIGHT: usize = 1024;
/// An idle TCP connection is closed after this long.
const TCP_IDLE: Duration = Duration::from_secs(10);

/// Bound sockets. Binding first lets the caller learn the real addresses
/// (for `127.0.0.1:0`) before serving.
pub struct Listeners {
    udp: Vec<UdpSocket>,
    tcp: Vec<TcpListener>,
}

impl Listeners {
    /// UDP and TCP on each address; TCP takes the port UDP actually got.
    pub async fn bind(addrs: &[SocketAddr]) -> io::Result<Listeners> {
        let (mut udp, mut tcp) = (Vec::new(), Vec::new());
        for addr in addrs {
            let socket = UdpSocket::bind(addr).await?;
            tcp.push(TcpListener::bind(socket.local_addr()?).await?);
            udp.push(socket);
        }
        Ok(Listeners { udp, tcp })
    }

    /// The bound addresses, one per `listen` entry.
    pub fn local_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        self.udp.iter().map(UdpSocket::local_addr).collect()
    }

    /// Serve until the future is dropped.
    pub async fn serve(self, engine: Arc<Engine>) {
        let permits = Arc::new(Semaphore::new(MAX_IN_FLIGHT));
        let mut tasks = JoinSet::new();
        for socket in self.udp {
            tasks.spawn(serve_udp(Arc::new(socket), engine.clone(), permits.clone()));
        }
        for listener in self.tcp {
            tasks.spawn(serve_tcp(listener, engine.clone(), permits.clone()));
        }
        while tasks.join_next().await.is_some() {}
    }
}

async fn serve_udp(socket: Arc<UdpSocket>, engine: Arc<Engine>, permits: Arc<Semaphore>) {
    let mut buf = vec![0; 65_535];
    loop {
        // Windows reports an ICMP "port unreachable" for an earlier reply as an
        // error on the next receive. It says nothing about this socket: go on.
        let Ok((len, peer)) = socket.recv_from(&mut buf).await else {
            continue;
        };
        let Ok(permit) = permits.clone().try_acquire_owned() else {
            continue;
        };
        let query = buf[..len].to_vec();
        let (socket, engine) = (socket.clone(), engine.clone());
        tokio::spawn(async move {
            if let Some(reply) = engine.handle(peer.ip(), &query, Transport::Udp).await {
                let _ = socket.send_to(&reply, peer).await;
            }
            drop(permit);
        });
    }
}

async fn serve_tcp(listener: TcpListener, engine: Arc<Engine>, permits: Arc<Semaphore>) {
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(e) => {
                // Out of file descriptors and similar: back off instead of spinning.
                tracing::warn!(error = %e, "accept failed");
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        // Not admitted: close at once, before reading a byte (spec §9.6).
        if engine.policy().admit(peer.ip()).is_none() {
            continue;
        }
        tokio::spawn(serve_connection(
            stream,
            peer,
            engine.clone(),
            permits.clone(),
        ));
    }
}

async fn serve_connection(
    mut stream: TcpStream,
    peer: SocketAddr,
    engine: Arc<Engine>,
    permits: Arc<Semaphore>,
) {
    while let Ok(Ok(query)) = timeout(TCP_IDLE, read_frame(&mut stream)).await {
        let Ok(_permit) = permits.acquire().await else {
            return;
        };
        let Some(reply) = engine.handle(peer.ip(), &query, Transport::Tcp).await else {
            return;
        };
        if write_frame(&mut stream, &reply).await.is_err() {
            return;
        }
    }
}
