//! Forwarding to upstream resolvers.

use crate::synth::MAX_UDP_PAYLOAD;
use hickory_proto::op::{Edns, Message, MessageType};
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::timeout;

/// Ask each server in turn; the first valid reply wins. `None` when every
/// server failed, which the caller turns into SERVFAIL (spec §20.3).
pub(crate) async fn exchange(
    servers: &[SocketAddr],
    request: &Message,
    wait: Duration,
) -> Option<Message> {
    let query = upstream_query(request);
    let bytes = query.to_vec().ok()?;
    for &server in servers {
        let reply = match udp(server, &bytes, &query, wait).await {
            // A truncated reply means the full one only fits over TCP.
            Ok(reply) if reply.metadata.truncation => tcp(server, &bytes, &query, wait).await,
            other => other,
        };
        match reply {
            Ok(reply) => return Some(reply),
            Err(e) => tracing::debug!(%server, error = %e, "upstream failed"),
        }
    }
    None
}

/// The query Veyra sends upstream for `request`: a random id of its own, so a
/// spoofed reply must guess it as well as the source port (spec §19.1), and an
/// OPT record of its own carrying only the DO bit, so client options such as a
/// client subnet never reach the upstream or the shared cache (spec §9.7).
fn upstream_query(request: &Message) -> Message {
    let mut query = Message::query();
    query.metadata.recursion_desired = true;
    query.metadata.checking_disabled = request.metadata.checking_disabled;
    query.add_queries(request.queries.iter().cloned());
    if let Some(asked) = &request.edns {
        let mut edns = Edns::new();
        edns.set_max_payload(MAX_UDP_PAYLOAD);
        edns.set_dnssec_ok(asked.flags().dnssec_ok);
        query.set_edns(edns);
    }
    query
}

/// One query over TCP with the 2-byte length prefix of RFC 1035 §4.2.2.
async fn tcp(
    server: SocketAddr,
    bytes: &[u8],
    query: &Message,
    wait: Duration,
) -> io::Result<Message> {
    timeout(wait, async {
        let mut stream = TcpStream::connect(server).await?;
        write_frame(&mut stream, bytes).await?;
        let reply = read_frame(&mut stream).await?;
        let reply =
            Message::from_vec(&reply).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if answers(&reply, query) {
            Ok(reply)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "reply does not match the query",
            ))
        }
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "no reply"))?
}

/// Read one length-prefixed DNS message from a TCP stream.
pub(crate) async fn read_frame(stream: &mut (impl AsyncRead + Unpin)) -> io::Result<Vec<u8>> {
    let mut len = [0u8; 2];
    stream.read_exact(&mut len).await?;
    let mut message = vec![0; usize::from(u16::from_be_bytes(len))];
    stream.read_exact(&mut message).await?;
    Ok(message)
}

/// Write one length-prefixed DNS message to a TCP stream.
pub(crate) async fn write_frame(
    stream: &mut (impl AsyncWrite + Unpin),
    message: &[u8],
) -> io::Result<()> {
    let len = u16::try_from(message.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "message longer than 65535 bytes",
        )
    })?;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(message).await
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
