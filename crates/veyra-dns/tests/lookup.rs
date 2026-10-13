//! Origin lookups for the gateway (spec §10.3): straight to the upstream, no policy.

mod support;

use hickory_proto::op::{Message, OpCode, ResponseCode};
use hickory_proto::rr::rdata::{A, AAAA, CNAME};
use hickory_proto::rr::{Name, RData, Record, RecordType};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use support::{dead_upstream, upstream};
use veyra_dns::lookup;

const WAIT: Duration = Duration::from_millis(300);
const V4: Ipv4Addr = Ipv4Addr::new(93, 184, 215, 14);
const V6: Ipv6Addr = Ipv6Addr::new(0x2606, 0x2800, 0x21f, 0, 0, 0, 0, 0x14);

/// A reply to `query` with `code` and `answers`.
fn reply(query: &Message, code: ResponseCode, answers: Vec<Record>) -> Message {
    let mut reply = Message::response(query.metadata.id, OpCode::Query);
    reply.metadata.response_code = code;
    reply.add_queries(query.queries.iter().cloned());
    for record in answers {
        reply.add_answer(record);
    }
    reply
}

/// Any name is a CNAME to `edge.example.`, which has `V4` and `V6`.
fn behind_a_cname(query: &Message) -> Message {
    let edge = Name::from_ascii("edge.example.").expect("name");
    let alias = RData::CNAME(CNAME(edge.clone()));
    let address = match query.queries[0].query_type() {
        RecordType::A => RData::A(A(V4)),
        _ => RData::AAAA(AAAA(V6)),
    };
    let answers = vec![
        Record::from_rdata(query.queries[0].name().clone(), 300, alias),
        Record::from_rdata(edge, 300, address),
    ];
    reply(query, ResponseCode::NoError, answers)
}

#[tokio::test]
async fn returns_ipv4_then_ipv6_through_cnames() {
    let up = upstream(behind_a_cname).await;
    let addrs = lookup(&[up.addr], "www.origin.example", WAIT).await;
    assert_eq!(addrs, vec![IpAddr::V4(V4), IpAddr::V6(V6)]);

    let mut asked: Vec<(String, RecordType)> = up
        .received()
        .iter()
        .map(|q| (q.queries[0].name().to_ascii(), q.queries[0].query_type()))
        .collect();
    asked.sort();
    assert_eq!(
        asked,
        [
            ("www.origin.example.".to_string(), RecordType::A),
            ("www.origin.example.".to_string(), RecordType::AAAA)
        ]
    );
}

#[tokio::test]
async fn errors_and_dead_upstreams_resolve_to_nothing() {
    let nxdomain = upstream(|q| reply(q, ResponseCode::NXDomain, Vec::new())).await;
    assert!(
        lookup(&[nxdomain.addr], "gone.example", WAIT)
            .await
            .is_empty()
    );
    let dead = dead_upstream().await;
    assert!(lookup(&[dead], "www.origin.example", WAIT).await.is_empty());
    assert!(lookup(&[], "www.origin.example", WAIT).await.is_empty());
}
