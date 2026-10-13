//! The DNS engine: admission, synthesis (spec §9.5) and forwarding.

mod support;

use hickory_proto::op::{Edns, Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::{RData, RecordType};
use std::net::{IpAddr, Ipv4Addr};
use support::{a_records, ask, dead_upstream, engine, query, upstream};
use veyra_dns::Transport::{Tcp, Udp};

fn addresses(reply: &Message) -> Vec<IpAddr> {
    reply
        .answers
        .iter()
        .filter_map(|r| match &r.data {
            RData::A(a) => Some(IpAddr::V4(a.0)),
            RData::AAAA(aaaa) => Some(IpAddr::V6(aaaa.0)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn drops_sources_that_are_not_admitted() {
    let engine = engine(&[dead_upstream().await]);
    let reply = ask(
        &engine,
        "10.9.9.9",
        &query("ads.example.", RecordType::A),
        Udp,
    )
    .await;
    assert!(reply.is_none());
}

#[tokio::test]
async fn drops_garbage_and_responses() {
    let engine = engine(&[dead_upstream().await]);
    let source = "127.0.0.1".parse().unwrap();
    assert!(engine.handle(source, &[1, 2, 3], Udp).await.is_none());
    let mut response = query("ads.example.", RecordType::A);
    response.metadata.message_type = MessageType::Response;
    assert!(ask(&engine, "127.0.0.1", &response, Udp).await.is_none());
}

#[tokio::test]
async fn blocked_names_get_null_addresses() {
    let engine = engine(&[dead_upstream().await]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("WWW.Ads.Example.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(reply.metadata.id, 0x4242);
    assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
    assert!(reply.metadata.recursion_available);
    assert!(!reply.metadata.authentic_data);
    assert_eq!(reply.queries[0].name().to_ascii(), "WWW.Ads.Example.");
    assert_eq!(addresses(&reply), vec![IpAddr::V4(Ipv4Addr::UNSPECIFIED)]);
    assert_eq!(reply.answers[0].ttl, 60);
}

#[tokio::test]
async fn proxied_names_get_the_gateway_and_nodata_elsewhere() {
    let engine = engine(&[dead_upstream().await]);
    let a = ask(
        &engine,
        "127.0.0.1",
        &query("media.example.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(
        addresses(&a),
        vec!["203.0.113.10".parse::<IpAddr>().unwrap()]
    );
    for record_type in [RecordType::AAAA, RecordType::HTTPS] {
        let reply = ask(
            &engine,
            "127.0.0.1",
            &query("media.example.", record_type),
            Udp,
        )
        .await
        .unwrap();
        assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
        assert!(reply.answers.is_empty(), "{record_type}");
        let soa = &reply.authorities[0];
        assert_eq!(soa.ttl, 60);
        let RData::SOA(soa) = &soa.data else {
            panic!("expected a SOA, got {:?}", soa.data)
        };
        assert_eq!(soa.minimum, 60);
    }
}

#[tokio::test]
async fn direct_names_are_forwarded() {
    let origin = Ipv4Addr::new(93, 184, 215, 14);
    let up = upstream(move |q| a_records(q, &[origin], 300)).await;
    let engine = engine(&[up.addr]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("example.org.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(addresses(&reply), vec![IpAddr::V4(origin)]);
    assert_eq!(reply.answers[0].ttl, 300);
    assert_eq!(up.queries(), 1);
}

#[tokio::test]
async fn names_that_do_not_normalize_use_the_profile_default() {
    let engine = engine(&[dead_upstream().await]);
    let reply = ask(
        &engine,
        "127.0.0.2",
        &query("*.example.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(addresses(&reply), vec![IpAddr::V4(Ipv4Addr::UNSPECIFIED)]);
}

#[tokio::test]
async fn dead_upstreams_give_servfail() {
    let engine = engine(&[dead_upstream().await]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("example.org.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(reply.metadata.response_code, ResponseCode::ServFail);
}

#[tokio::test]
async fn a_dead_upstream_falls_through_to_the_next() {
    let origin = Ipv4Addr::new(93, 184, 215, 14);
    let up = upstream(move |q| a_records(q, &[origin], 300)).await;
    let engine = engine(&[dead_upstream().await, up.addr]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("example.org.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(addresses(&reply), vec![IpAddr::V4(origin)]);
}

#[tokio::test]
async fn other_opcodes_and_question_counts_are_refused_politely() {
    let engine = engine(&[dead_upstream().await]);
    let mut status = query("example.org.", RecordType::A);
    status.metadata.op_code = OpCode::Status;
    let reply = ask(&engine, "127.0.0.1", &status, Udp).await.unwrap();
    assert_eq!(reply.metadata.response_code, ResponseCode::NotImp);

    let mut two = query("a.example.", RecordType::A);
    two.add_queries(query("b.example.", RecordType::A).queries);
    let reply = ask(&engine, "127.0.0.1", &two, Udp).await.unwrap();
    assert_eq!(reply.metadata.response_code, ResponseCode::FormErr);
}

#[tokio::test]
async fn edns_queries_get_edns_answers_without_the_ad_bit() {
    let engine = engine(&[dead_upstream().await]);
    let mut q = query("ads.example.", RecordType::A);
    let mut edns = Edns::new();
    edns.set_max_payload(1232);
    edns.set_dnssec_ok(true);
    q.set_edns(edns);
    let reply = ask(&engine, "127.0.0.1", &q, Udp).await.unwrap();
    let edns = reply.edns.expect("an OPT record in the reply");
    assert!(edns.flags().dnssec_ok);
    assert!(!reply.metadata.authentic_data);
    assert!(
        reply
            .answers
            .iter()
            .all(|r| r.record_type() != RecordType::RRSIG)
    );
}

#[tokio::test]
async fn oversized_udp_answers_are_truncated_but_tcp_gets_everything() {
    let many: Vec<Ipv4Addr> = (1..=40).map(|i| Ipv4Addr::new(192, 0, 2, i)).collect();
    let up = upstream(move |q| a_records(q, &many, 300)).await;
    let engine = engine(&[up.addr]);
    let q = query("big.example.", RecordType::A);

    let udp = ask(&engine, "127.0.0.1", &q, Udp).await.unwrap();
    assert!(udp.metadata.truncation);
    assert!(udp.answers.is_empty());

    let tcp = ask(&engine, "127.0.0.1", &q, Tcp).await.unwrap();
    assert!(!tcp.metadata.truncation);
    assert_eq!(tcp.answers.len(), 40);
}
