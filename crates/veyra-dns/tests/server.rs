//! UDP and TCP listeners (spec §9.1, §9.6).

mod support;

use hickory_proto::op::Message;
use hickory_proto::rr::{RData, RecordType};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use support::{dead_upstream, engine, query};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream, UdpSocket};
use tokio::time::timeout;
use veyra_dns::Listeners;

/// Serve the support engine on 127.0.0.1 with an OS-chosen port.
async fn serve() -> SocketAddr {
    let listeners = Listeners::bind(&["127.0.0.1:0".parse().expect("address")])
        .await
        .expect("bind listeners");
    let addr = listeners.local_addrs().expect("bound addresses")[0];
    let engine = Arc::new(engine(&[dead_upstream().await]));
    tokio::spawn(listeners.serve(engine));
    addr
}

fn blocked(reply: &Message) -> bool {
    matches!(&reply.answers[..], [record] if matches!(record.data, RData::A(a) if a.0 == Ipv4Addr::UNSPECIFIED))
}

async fn udp_ask(server: SocketAddr, from: &str, query: &Message) -> Option<Message> {
    let socket = UdpSocket::bind(from).await.expect("bind client");
    socket
        .send_to(&query.to_vec().expect("encode"), server)
        .await
        .expect("send");
    let mut buf = vec![0; 4096];
    let len = timeout(Duration::from_millis(500), socket.recv(&mut buf))
        .await
        .ok()?
        .expect("receive");
    Some(Message::from_vec(&buf[..len]).expect("valid reply"))
}

async fn tcp_ask(stream: &mut TcpStream, query: &Message) -> Message {
    let bytes = query.to_vec().expect("encode");
    let len = u16::try_from(bytes.len()).expect("fits").to_be_bytes();
    stream.write_all(&len).await.expect("write length");
    stream.write_all(&bytes).await.expect("write query");
    let mut len = [0u8; 2];
    stream.read_exact(&mut len).await.expect("read length");
    let mut buf = vec![0; usize::from(u16::from_be_bytes(len))];
    stream.read_exact(&mut buf).await.expect("read reply");
    Message::from_vec(&buf).expect("valid reply")
}

#[tokio::test]
async fn answers_over_udp() {
    let server = serve().await;
    let reply = udp_ask(server, "127.0.0.1:0", &query("ads.example.", RecordType::A))
        .await
        .expect("a reply");
    assert!(blocked(&reply), "{reply:?}");
}

#[tokio::test]
async fn one_tcp_connection_carries_several_queries() {
    let server = serve().await;
    let mut stream = TcpStream::connect(server).await.expect("connect");
    for name in ["ads.example.", "www.ads.example."] {
        let reply = tcp_ask(&mut stream, &query(name, RecordType::A)).await;
        assert!(blocked(&reply), "{name}: {reply:?}");
    }
}

#[tokio::test]
async fn strangers_get_silence_over_udp() {
    let server = serve().await;
    let reply = udp_ask(server, "127.0.0.3:0", &query("ads.example.", RecordType::A)).await;
    assert!(reply.is_none());
}

#[tokio::test]
async fn strangers_get_a_closed_tcp_connection() {
    let server = serve().await;
    let socket = TcpSocket::new_v4().expect("socket");
    socket
        .bind("127.0.0.3:0".parse().unwrap())
        .expect("bind client");
    let mut stream = socket.connect(server).await.expect("connect");
    let mut byte = [0u8; 1];
    let read = timeout(Duration::from_secs(2), stream.read(&mut byte))
        .await
        .expect("the server closes the connection promptly");
    assert!(matches!(read, Ok(0) | Err(_)), "{read:?}");
}
