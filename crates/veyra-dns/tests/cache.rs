//! Caching upstream answers (spec §9.3) and TCP fallback to upstreams.

mod support;

use hickory_proto::op::{Message, ResponseCode};
use hickory_proto::rr::RecordType;
use std::net::Ipv4Addr;
use support::{a_records, ask, engine, query, truncating_upstream, upstream};
use veyra_dns::Transport::{Tcp, Udp};

const ORIGIN: Ipv4Addr = Ipv4Addr::new(93, 184, 215, 14);

#[tokio::test]
async fn a_repeated_question_is_answered_from_the_cache() {
    let up = upstream(|q| a_records(q, &[ORIGIN], 300)).await;
    let engine = engine(&[up.addr]);
    ask(
        &engine,
        "127.0.0.1",
        &query("example.org.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();

    let mut again = query("EXAMPLE.org.", RecordType::A);
    again.metadata.id = 0x0bad;
    let reply = ask(&engine, "127.0.0.1", &again, Udp).await.unwrap();
    assert_eq!(
        up.queries(),
        1,
        "the second question must not reach the upstream"
    );
    assert_eq!(reply.metadata.id, 0x0bad);
    assert_eq!(reply.queries[0].name().to_ascii(), "EXAMPLE.org.");
    assert_eq!(reply.answers.len(), 1);
}

#[tokio::test]
async fn different_record_types_are_cached_separately() {
    let up = upstream(|q| a_records(q, &[ORIGIN], 300)).await;
    let engine = engine(&[up.addr]);
    for record_type in [RecordType::A, RecordType::TXT, RecordType::A] {
        ask(
            &engine,
            "127.0.0.1",
            &query("example.org.", record_type),
            Udp,
        )
        .await
        .unwrap();
    }
    assert_eq!(up.queries(), 2);
}

#[tokio::test]
async fn servfail_is_not_cached() {
    let up = upstream(|q| {
        let mut reply = a_records(q, &[], 0);
        reply.metadata.response_code = ResponseCode::ServFail;
        reply
    })
    .await;
    let engine = engine(&[up.addr]);
    for _ in 0..2 {
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
    assert_eq!(up.queries(), 2);
}

#[tokio::test]
async fn a_truncated_upstream_reply_is_retried_over_tcp() {
    let many: Vec<Ipv4Addr> = (1..=40).map(|i| Ipv4Addr::new(192, 0, 2, i)).collect();
    let up = truncating_upstream(move |q: &Message| a_records(q, &many, 300)).await;
    let engine = engine(&[up.addr]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("big.example.", RecordType::A),
        Tcp,
    )
    .await
    .unwrap();
    assert!(!reply.metadata.truncation);
    assert_eq!(reply.answers.len(), 40);
}

#[tokio::test]
async fn cached_answers_never_cross_into_another_profile() {
    // REQ-DNS-004: `home` (default direct) caches example.org; `strict`
    // (default block) asking the same name must still be blocked.
    let up = upstream(|q| a_records(q, &[ORIGIN], 300)).await;
    let engine = engine(&[up.addr]);
    let q = query("example.org.", RecordType::A);
    let home = ask(&engine, "127.0.0.1", &q, Udp).await.unwrap();
    assert_eq!(home.answers.len(), 1);
    let strict = ask(&engine, "127.0.0.2", &q, Udp).await.unwrap();
    let blocked = matches!(&strict.answers[..], [r] if r.data == hickory_proto::rr::RData::A(hickory_proto::rr::rdata::A(Ipv4Addr::UNSPECIFIED)));
    assert!(blocked, "{strict:?}");
    assert_eq!(up.queries(), 1);
}
