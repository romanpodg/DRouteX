//! What Veyra sends upstream: its own id and its own OPT record, never the
//! client's (spec §19.1 spoofing, §9.7 ECS off by default).

mod support;

use hickory_proto::op::Edns;
use hickory_proto::rr::RecordType;
use hickory_proto::rr::rdata::opt::{ClientSubnet, EdnsOption};
use std::net::Ipv4Addr;
use support::{a_records, ask, engine, query, upstream};
use veyra_dns::Transport::Udp;

const ORIGIN: Ipv4Addr = Ipv4Addr::new(93, 184, 215, 14);

#[tokio::test]
async fn upstream_queries_carry_their_own_id() {
    let up = upstream(|q| a_records(q, &[ORIGIN], 300)).await;
    let engine = engine(&[up.addr]);
    for name in ["a.example.", "b.example.", "c.example."] {
        let reply = ask(&engine, "127.0.0.1", &query(name, RecordType::A), Udp)
            .await
            .unwrap();
        assert_eq!(reply.metadata.id, 0x4242, "the client gets its own id back");
        assert_eq!(reply.answers.len(), 1, "{name}");
    }
    let ids: Vec<u16> = up.received().iter().map(|q| q.metadata.id).collect();
    assert!(
        ids.iter().any(|id| *id != 0x4242),
        "upstream ids must not be the client's: {ids:?}"
    );
}

#[tokio::test]
async fn client_edns_options_never_reach_the_upstream() {
    let up = upstream(|q| a_records(q, &[ORIGIN], 300)).await;
    let engine = engine(&[up.addr]);
    let mut q = query("example.org.", RecordType::A);
    let mut edns = Edns::new();
    edns.set_max_payload(65_535);
    edns.set_dnssec_ok(true);
    let subnet = ClientSubnet::new("198.51.100.0".parse().unwrap(), 24, 0);
    edns.options_mut().insert(EdnsOption::Subnet(subnet));
    q.set_edns(edns);
    ask(&engine, "127.0.0.1", &q, Udp).await.unwrap();

    let sent = up.received();
    let edns = sent[0]
        .edns
        .as_ref()
        .expect("the upstream query keeps EDNS");
    assert!(edns.options().options.is_empty(), "{:?}", edns.options());
    assert_eq!(edns.max_payload(), 1232);
    assert!(edns.flags().dnssec_ok, "the DO bit is kept");
}
