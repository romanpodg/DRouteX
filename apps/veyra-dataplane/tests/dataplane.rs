//! End-to-end: the `veyra-dataplane` binary serving DNS from a policy file.

use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{Name, RData, RecordType};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
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

/// Start the binary and wait for its "listening on <addr>" line.
fn start(config: &str) -> (Server, SocketAddr) {
    let path = write_config("serve", config);
    let mut child = Command::new(env!("CARGO_BIN_EXE_veyra-dataplane"))
        .args(["--config".as_ref(), path.as_os_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start veyra-dataplane");
    let stdout = child.stdout.take().expect("stdout");
    let server = Server(child);
    let mut line = String::new();
    BufReader::new(stdout)
        .read_line(&mut line)
        .expect("read startup line");
    let addr = line
        .split_whitespace()
        .nth(2)
        .and_then(|a| a.parse().ok())
        .unwrap_or_else(|| panic!("unexpected startup line {line:?}"));
    (server, addr)
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
    let (_server, addr) = start(CONFIG);

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
