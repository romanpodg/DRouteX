//! End-to-end: the `veyra-dataplane` binary serving DNS from a policy file.

use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{Name, RData, RecordType};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const CONFIG: &str = r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: e2e }
spec:
  nodes: [{ id: de-01, gateway: { ipv4: 203.0.113.10 } }]
  profiles: [{ id: home }]
  admission: [{ cidr: 127.0.0.1/32, profile: home }]
  rules:
    - { id: ads, scope: default, match: { domains: ["suffix:ads.example"] }, action: { type: block } }
    - { id: media, scope: default, match: { domains: ["suffix:media.example"] }, action: { type: proxy_node, node: de-01 } }
  dns:
    listen: ["127.0.0.1:0"]
    upstreams: [{ id: default, servers: ["127.0.0.1:9"] }]
"#;

/// Kills the server when the test ends, pass or fail.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn write_config(name: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("veyra-e2e-{}-{name}.yaml", std::process::id()));
    std::fs::write(&path, text).expect("write config");
    path
}

/// Start the binary on a config file of its own (tests run in parallel) and
/// read its first `lines` "listening on <addr> (...)" lines.
fn start(name: &str, config: &str, lines: usize) -> (Server, Vec<SocketAddr>) {
    let path = write_config(name, config);
    let mut child = Command::new(env!("CARGO_BIN_EXE_veyra-dataplane"))
        .args(["--config".as_ref(), path.as_os_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start veyra-dataplane");
    let stdout = child.stdout.take().expect("stdout");
    let server = Server(child);
    // Read on a thread: a binary that prints fewer lines fails the test instead
    // of hanging it, and later output never meets a closed pipe.
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let addrs = (0..lines)
        .map(|_| {
            let line = rx
                .recv_timeout(Duration::from_secs(10))
                .expect("a listening line within 10 s");
            line.split_whitespace()
                .nth(2)
                .and_then(|a| a.parse().ok())
                .unwrap_or_else(|| panic!("unexpected startup line {line:?}"))
        })
        .collect();
    (server, addrs)
}

fn query(name: &str, record_type: RecordType) -> Vec<u8> {
    let mut message = Message::new(7, MessageType::Query, OpCode::Query);
    message.metadata.recursion_desired = true;
    message.add_query(Query::query(
        Name::from_ascii(name).expect("name"),
        record_type,
    ));
    message.to_vec().expect("encode")
}

#[test]
fn serves_synthesized_answers_over_udp_and_tcp() {
    let (_server, addrs) = start("dns", CONFIG, 1);
    let addr = addrs[0];

    let udp = UdpSocket::bind("127.0.0.1:0").expect("bind client");
    udp.set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    udp.send_to(&query("ads.example.", RecordType::A), addr)
        .expect("send");
    let mut buf = [0; 4096];
    let len = udp.recv(&mut buf).expect("UDP reply");
    let reply = Message::from_vec(&buf[..len]).expect("valid reply");
    assert!(matches!(&reply.answers[0].data, RData::A(a) if a.0 == Ipv4Addr::UNSPECIFIED));

    let mut tcp = TcpStream::connect(addr).expect("connect");
    tcp.set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let bytes = query("media.example.", RecordType::AAAA);
    let len = u16::try_from(bytes.len()).expect("fits").to_be_bytes();
    tcp.write_all(&len).expect("write length");
    tcp.write_all(&bytes).expect("write query");
    let mut len = [0u8; 2];
    tcp.read_exact(&mut len).expect("read length");
    let mut buf = vec![0; usize::from(u16::from_be_bytes(len))];
    tcp.read_exact(&mut buf).expect("read reply");
    let reply = Message::from_vec(&buf).expect("valid reply");
    assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
    assert!(
        reply.answers.is_empty(),
        "AAAA without a gateway IPv6 is NODATA"
    );
}

#[test]
fn refuses_a_config_without_a_dns_section() {
    let rules_only = CONFIG.split("  dns:").next().expect("config text");
    let path = write_config("nodns", rules_only);
    let out = Command::new(env!("CARGO_BIN_EXE_veyra-dataplane"))
        .args(["--config".as_ref(), path.as_os_str()])
        .output()
        .expect("run veyra-dataplane");
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no dns section"), "{err}");
}

/// True when the peer closed the connection, false when the read timed out.
fn closed_by_peer(stream: &mut TcpStream) -> bool {
    match stream.read(&mut [0u8; 1]) {
        Ok(0) => true,
        Ok(_) => false,
        Err(e) => !matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut),
    }
}

#[test]
fn the_gateway_refuses_unrouted_names_and_rejects_udp() {
    let gateway = "  gateway: { node: de-01, tls: [\"127.0.0.1:0\"], http: [\"127.0.0.1:0\"] }\n";
    let (_server, addrs) = start("gateway", &format!("{CONFIG}{gateway}"), 3);
    let (tls, http) = (addrs[1], addrs[2]);

    for (addr, head) in [
        (tls, &b"not a ClientHello"[..]),
        (http, &b"GET / HTTP/1.1\r\nHost: example.org\r\n\r\n"[..]),
    ] {
        let mut client = TcpStream::connect(addr).expect("connect");
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("timeout");
        client.write_all(head).expect("write");
        assert!(closed_by_peer(&mut client), "{addr}");
    }

    // UDP to the TLS port is rejected (ICMP port unreachable), not dropped, so
    // HTTP/3 clients fall back to TCP at once (spec §10.1, REQ-GW-002).
    let udp = UdpSocket::bind("127.0.0.1:0").expect("bind client");
    udp.connect(tls).expect("connect");
    udp.set_read_timeout(Some(Duration::from_secs(1)))
        .expect("timeout");
    udp.send(b"QUIC initial").expect("send");
    let err = udp
        .recv(&mut [0u8; 64])
        .expect_err("nothing serves UDP on the gateway port");
    assert!(
        matches!(
            err.kind(),
            ErrorKind::ConnectionRefused | ErrorKind::ConnectionReset
        ),
        "{err:?}"
    );
}
