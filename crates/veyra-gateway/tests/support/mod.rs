//! Shared test helpers: ClientHello bytes, a fake upstream DNS server, an echo
//! origin, and a gateway over a small policy.
#![allow(dead_code)] // each test file uses a different subset

use hickory_proto::op::{Message, OpCode};
use hickory_proto::rr::rdata::A;
use hickory_proto::rr::{RData, Record, RecordType};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpSocket, TcpStream, UdpSocket};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use veyra_gateway::{Gateway, Refusal, Service};
use veyra_policy::{Bundle, Cidr, Policy};

/// A UDP DNS server on 127.0.0.1 that answers every A query with `ips` and
/// every other query with no records.
pub async fn upstream(ips: &[Ipv4Addr]) -> SocketAddr {
    let ips = ips.to_vec();
    upstream_with(move |_| ips.clone()).await
}

/// A UDP DNS server on 127.0.0.1 whose answer to the n-th A query (from 0)
/// is `answer(n)`; other queries get no records.
pub async fn upstream_with(answer: impl Fn(usize) -> Vec<Ipv4Addr> + Send + 'static) -> SocketAddr {
    let socket = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind fake upstream");
    let addr = socket.local_addr().expect("fake upstream address");
    tokio::spawn(async move {
        let mut buf = vec![0; 4096];
        let mut asked = 0;
        loop {
            let Ok((len, from)) = socket.recv_from(&mut buf).await else {
                continue;
            };
            let Ok(query) = Message::from_vec(&buf[..len]) else {
                continue;
            };
            let Some(question) = query.queries.first().cloned() else {
                continue;
            };
            let mut reply = Message::response(query.metadata.id, OpCode::Query);
            reply.add_query(question.clone());
            if question.query_type() == RecordType::A {
                for ip in answer(asked) {
                    let name = question.name().clone();
                    reply.add_answer(Record::from_rdata(name, 60, RData::A(A(ip))));
                }
                asked += 1;
            }
            let bytes = reply.to_vec().expect("encode fake reply");
            let _ = socket.send_to(&bytes, from).await;
        }
    });
    addr
}

/// A TCP server on 127.0.0.1 that echoes every byte back.
pub struct Origin {
    pub port: u16,
    connections: Arc<AtomicUsize>,
}

impl Origin {
    /// Connections accepted so far.
    pub fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }
}

pub async fn origin() -> Origin {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind origin");
    let port = listener.local_addr().expect("origin address").port();
    let connections = Arc::new(AtomicUsize::new(0));
    let counter = connections.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                continue;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let (mut read, mut write) = stream.split();
                let _ = tokio::io::copy(&mut read, &mut write).await;
            });
        }
    });
    Origin { port, connections }
}

/// A gateway for node de-01 (203.0.113.10) whose origins resolve through
/// `upstream` and listen on `origin_port`. 127.0.0.1 is admitted as `home` and
/// may be dialed despite the egress rules. Rules:
/// - home: `suffix:media.example` via de-01, `login.media.example` direct;
/// - strict (default block): `suffix:video.example` via de-01;
/// - everyone: `suffix:us.example` via us-01, `tunnel.example` via de-01 in
///   tunnel mode only, `suffix:ads.example` blocked.
pub fn gateway(upstream: SocketAddr, origin_port: u16) -> Gateway {
    let yaml = format!(
        r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: {{ name: gateway }}
spec:
  nodes:
    - {{ id: de-01, gateway: {{ ipv4: 203.0.113.10 }} }}
    - {{ id: us-01, gateway: {{ ipv4: 198.51.100.20 }} }}
  profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]
  admission: [{{ cidr: 127.0.0.1/32, profile: home }}]
  rules:
    - {{ id: media, scope: {{ profile: home }}, match: {{ domains: ["suffix:media.example"] }}, action: {{ type: proxy_node, node: de-01 }} }}
    - {{ id: login, scope: {{ profile: home }}, match: {{ domains: [login.media.example] }}, action: {{ type: direct }} }}
    - {{ id: video, scope: {{ profile: strict }}, match: {{ domains: ["suffix:video.example"] }}, action: {{ type: proxy_node, node: de-01 }} }}
    - {{ id: us, scope: default, match: {{ domains: ["suffix:us.example"] }}, action: {{ type: proxy_node, node: us-01 }} }}
    - {{ id: tunnel-only, scope: default, match: {{ domains: [tunnel.example] }}, action: {{ type: proxy_node, node: de-01 }}, modesAllowed: [tunnel] }}
    - {{ id: ads, scope: default, match: {{ domains: ["suffix:ads.example"] }}, action: {{ type: block }} }}
  dns:
    listen: ["127.0.0.1:0"]
    upstreams: [{{ id: default, servers: ["{upstream}"] }}]
  gateway: {{ node: de-01, tls: ["127.0.0.1:0"], http: ["127.0.0.1:0"] }}
"#
    );
    let bundle = Bundle::from_yaml(&yaml).expect("test YAML parses");
    let (policy, _warnings) = Policy::compile(&bundle).expect("test bundle compiles");
    Gateway::new(policy)
        .expect("policy has a gateway section")
        .with_origin_ports(origin_port, origin_port)
        .with_timeouts(Duration::from_millis(500), Duration::from_millis(300))
        .allowing(Cidr::parse("127.0.0.1/32").expect("network"))
}

/// Connect from `source` to a fresh listener and hand the accepted end to the
/// gateway: the client end, and the gateway's verdict when it is done.
pub async fn connect(
    gateway: Arc<Gateway>,
    source: &str,
    service: Service,
) -> (TcpStream, JoinHandle<Result<(), Refusal>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let socket = TcpSocket::new_v4().expect("socket");
    let source = format!("{source}:0").parse().expect("source address");
    socket.bind(source).expect("bind client");
    let addr = listener.local_addr().expect("listener address");
    let client = socket.connect(addr).await.expect("connect");
    let (accepted, _) = listener.accept().await.expect("accept");
    let verdict = tokio::spawn(async move { gateway.handle(accepted, service).await });
    (client, verdict)
}

/// Send `bytes` and read as many back: what the echo origin returned.
pub async fn round_trip(client: &mut TcpStream, bytes: &[u8]) -> Vec<u8> {
    client.write_all(bytes).await.expect("write");
    let mut back = vec![0; bytes.len()];
    timeout(Duration::from_secs(5), client.read_exact(&mut back))
        .await
        .expect("the echo arrives in time")
        .expect("read the echo");
    back
}

/// A ClientHello record naming `sni` (none for `None`), then the `extra`
/// extensions as (type, body).
pub fn client_hello(sni: Option<&str>, extra: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut extensions = Vec::new();
    let mut push = |kind: u16, body: &[u8]| {
        extensions.extend(kind.to_be_bytes());
        extensions.extend(len16(body.len()));
        extensions.extend(body);
    };
    if let Some(name) = sni {
        push(0, &server_name(name));
    }
    for (kind, body) in extra {
        push(*kind, body);
    }
    let mut body = vec![3, 3]; // legacy_version: TLS 1.2
    body.extend([7; 32]); // random
    body.push(0); // no session id
    body.extend([0, 2, 0x13, 0x01]); // one cipher suite: TLS_AES_128_GCM_SHA256
    body.extend([1, 0]); // compression: null only
    body.extend(len16(extensions.len()));
    body.extend(extensions);
    let mut record = vec![22, 3, 1];
    record.extend(len16(body.len() + 4));
    record.push(1); // ClientHello
    record.extend(&u32::try_from(body.len()).expect("fits").to_be_bytes()[1..]);
    record.extend(body);
    record
}

/// The body of a server_name extension with one host_name entry.
pub fn server_name(name: &str) -> Vec<u8> {
    let mut entry = vec![0]; // host_name
    entry.extend(len16(name.len()));
    entry.extend(name.as_bytes());
    let mut body = len16(entry.len()).to_vec();
    body.extend(entry);
    body
}

fn len16(len: usize) -> [u8; 2] {
    u16::try_from(len)
        .expect("length fits in 16 bits")
        .to_be_bytes()
}

/// Bytes from a hex dump; whitespace is ignored.
pub fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    digits
        .chunks(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII");
            u8::from_str_radix(pair, 16).expect("hex digit pair")
        })
        .collect()
}
