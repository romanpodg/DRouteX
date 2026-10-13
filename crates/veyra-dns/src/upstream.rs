//! Forwarding to upstream resolvers.

use hickory_proto::op::{Message, MessageType};
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;

/// Ask each server in turn; the first valid reply wins. `None` when every
/// server failed, which the caller turns into SERVFAIL (spec §20.3).
pub(crate) async fn exchange(
    servers: &[SocketAddr],
    request: &Message,
    wait: Duration,
) -> Option<Message> {
    let mut query = request.clone();
    query.metadata.recursion_desired = true;
    let bytes = query.to_vec().ok()?;
    for &server in servers {
        match udp(server, &bytes, &query, wait).await {
            Ok(reply) => return Some(reply),
            Err(e) => tracing::debug!(%server, error = %e, "upstream failed"),
        }
    }
    None
}

/// One query over a fresh socket: the OS picks a random source port, which
/// with the client's random id makes off-path spoofing hard (spec §19.1).
// ponytail: a socket per query; pool sockets if upstream QPS ever matters.
async fn udp(
    server: SocketAddr,
    bytes: &[u8],
    query: &Message,
    wait: Duration,
) -> io::Result<Message> {
    let local: SocketAddr = if server.is_ipv4() {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (Ipv6Addr::UNSPECIFIED, 0).into()
    };
    let socket = UdpSocket::bind(local).await?;
    socket.connect(server).await?;
    socket.send(bytes).await?;
    let mut buf = vec![0; 65_535];
    timeout(wait, async {
        loop {
            let len = socket.recv(&mut buf).await?;
            if let Ok(reply) = Message::from_vec(&buf[..len]) {
                if answers(&reply, query) {
                    return Ok(reply);
                }
            }
        }
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "no reply"))?
}

/// A reply belongs to our query only with the same id and question.
pub(crate) fn answers(reply: &Message, query: &Message) -> bool {
    reply.metadata.message_type == MessageType::Response
        && reply.metadata.id == query.metadata.id
        && reply.queries == query.queries
}
