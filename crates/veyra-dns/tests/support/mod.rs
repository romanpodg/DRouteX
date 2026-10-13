//! Shared test helpers: a fake upstream DNS server, query builders and an engine
//! over a small policy.
#![allow(dead_code)] // each test file uses a different subset

use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::rdata::A;
use hickory_proto::rr::{Name, RData, Record, RecordType};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::net::UdpSocket;
use veyra_dns::{Engine, Transport};
use veyra_policy::{Bundle, Policy};

/// A fake upstream and how many queries it has received.
pub struct Upstream {
    pub addr: SocketAddr,
    queries: Arc<AtomicUsize>,
}

impl Upstream {
    pub fn queries(&self) -> usize {
        self.queries.load(Ordering::SeqCst)
    }
}

/// A UDP DNS server on 127.0.0.1 that answers every query with `reply(query)`.
pub async fn upstream(reply: impl Fn(&Message) -> Message + Send + Sync + 'static) -> Upstream {
    let socket = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind fake upstream");
    let addr = socket.local_addr().expect("fake upstream address");
    let queries = Arc::new(AtomicUsize::new(0));
    let counter = queries.clone();
    tokio::spawn(async move {
        let mut buf = vec![0; 4096];
        loop {
            let Ok((len, from)) = socket.recv_from(&mut buf).await else {
                continue;
            };
            let Ok(query) = Message::from_vec(&buf[..len]) else {
                continue;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            let bytes = reply(&query).to_vec().expect("encode fake reply");
            let _ = socket.send_to(&bytes, from).await;
        }
    });
    Upstream { addr, queries }
}

/// An address where nothing listens.
pub async fn dead_upstream() -> SocketAddr {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind");
    socket.local_addr().expect("address")
}

/// A reply to `query` carrying `ips` as A records.
pub fn a_records(query: &Message, ips: &[Ipv4Addr], ttl: u32) -> Message {
    let mut reply = Message::response(query.metadata.id, OpCode::Query);
    reply.metadata.recursion_desired = query.metadata.recursion_desired;
    reply.metadata.recursion_available = true;
    reply.add_queries(query.queries.iter().cloned());
    for ip in ips {
        let name = query.queries[0].name().clone();
        reply.add_answer(Record::from_rdata(name, ttl, RData::A(A(*ip))));
    }
    reply
}

pub fn query(name: &str, record_type: RecordType) -> Message {
    let mut message = Message::new(0x4242, MessageType::Query, OpCode::Query);
    message.metadata.recursion_desired = true;
    message.add_query(Query::query(
        Name::from_ascii(name).expect("test name"),
        record_type,
    ));
    message
}

/// An engine for a policy with:
/// - 127.0.0.1 admitted as `home` (default direct), 127.0.0.2 as `strict` (default block);
/// - `suffix:ads.example` blocked, `suffix:media.example` proxied via de-01 (203.0.113.10);
/// - the `default` upstream group set to `servers`.
pub fn engine(servers: &[SocketAddr]) -> Engine {
    let servers = servers
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let yaml = format!(
        r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: {{ name: engine }}
spec:
  nodes: [{{ id: de-01, gateway: {{ ipv4: 203.0.113.10 }} }}]
  profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]
  admission:
    - {{ cidr: 127.0.0.1/32, profile: home }}
    - {{ cidr: 127.0.0.2/32, profile: strict }}
  rules:
    - {{ id: ads, scope: default, match: {{ domains: ["suffix:ads.example"] }}, action: {{ type: block }} }}
    - {{ id: media, scope: default, match: {{ domains: ["suffix:media.example"] }}, action: {{ type: proxy_node, node: de-01 }} }}
  dns:
    listen: ["127.0.0.1:5353"]
    upstreams:
      - {{ id: default, servers: [{servers}] }}
"#
    );
    let bundle = Bundle::from_yaml(&yaml).expect("test YAML parses");
    let (policy, _) = Policy::compile(&bundle).expect("test bundle compiles");
    Engine::new(policy)
        .expect("policy has a dns section")
        .with_upstream_timeout(Duration::from_millis(300))
}

/// Send `query` from `source` and decode the reply, if any.
pub async fn ask(
    engine: &Engine,
    source: &str,
    query: &Message,
    transport: Transport,
) -> Option<Message> {
    let bytes = query.to_vec().expect("encode query");
    let source = source.parse().expect("test source address");
    let reply = engine.handle(source, &bytes, transport).await?;
    Some(Message::from_vec(&reply).expect("engine replies with valid DNS"))
}
