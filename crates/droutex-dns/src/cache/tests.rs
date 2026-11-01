use super::*;
use crate::wire;
use droutex_config::{snapshot::SnapshotSequence, upstream::Transport};
use hickory_proto::{
    op::{Edns, ResponseCode},
    rr::{
        Name, RData, Record,
        rdata::{A, CNAME, NS, SOA, TXT},
    },
};
use std::{net::Ipv4Addr, sync::Arc};
use tokio::time::Duration;

fn id<T: std::str::FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn request() -> Message {
    wire::parse(
        b"\x04\xd2\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01",
    )
    .unwrap()
}
fn key(n: u16, scope: u64, sequence: u64) -> Key {
    let mut name = [0; 255];
    name[..2].copy_from_slice(&n.to_be_bytes());
    Key {
        generation: SnapshotMetadata {
            node: id(1),
            snapshot: id(2),
            source: id(3),
            sequence: SnapshotSequence::new(sequence).unwrap(),
        },
        scope: Scope {
            endpoint: id(20),
            profile: id(30),
            group: id(scope),
        },
        upstream: id(60),
        resource: ContentHash::of(b"group"),
        name,
        name_len: 2,
        query_type: 1,
        class: 1,
        domain: Domain::ForwardedD53NonDnssec,
    }
}
fn answer(ttl: u32, now: Instant) -> UpstreamAnswer {
    let mut message = wire::response(&request(), ResponseCode::NoError);
    message.answers.push(Record::from_rdata(
        "example.test.".parse().unwrap(),
        ttl,
        RData::A(A(Ipv4Addr::new(203, 0, 113, 9))),
    ));
    UpstreamAnswer {
        message,
        received: now,
        member: 0,
        transport: Transport::UdpTcp,
    }
}
fn soa(owner: &str, ttl: u32, minimum: u32) -> Record {
    Record::from_rdata(
        owner.parse::<Name>().unwrap(),
        ttl,
        RData::SOA(SOA::new(
            "ns.test.".parse().unwrap(),
            "hostmaster.test.".parse().unwrap(),
            1,
            100,
            100,
            100,
            minimum,
        )),
    )
}
fn negative(code: ResponseCode, ttl: u32, minimum: u32, now: Instant) -> UpstreamAnswer {
    let mut a = answer(100, now);
    a.message.answers.clear();
    a.message.metadata.response_code = code;
    a.message.authorities.push(soa("test.", ttl, minimum));
    a
}

#[test]
fn packet_expires_at_the_earliest_real_record_and_never_refreshes() {
    let now = Instant::now();
    let mut a = answer(60, now);
    a.message.additionals.push(Record::from_rdata(
        "other.test.".parse().unwrap(),
        5,
        RData::A(A(Ipv4Addr::LOCALHOST)),
    ));
    let template = Template::build(&a).unwrap();
    for (elapsed, ttl) in [(0, 5), (1500, 3), (4999, 0)] {
        let hit = template
            .reply(&request(), now + Duration::from_millis(elapsed))
            .unwrap();
        assert_eq!(hit.message.answers[0].ttl, ttl);
        assert_eq!(hit.message.additionals[0].ttl, ttl);
    }
    assert!(
        template
            .reply(&request(), now + Duration::from_secs(5))
            .is_none()
    );
    assert!(
        template
            .reply(&request(), now - Duration::from_millis(1))
            .is_none()
    );
    let cache = Cache::new();
    cache.insert(key(0, 1, 1), template, now);
    assert!(
        cache
            .get(&key(0, 1, 1), now + Duration::from_secs(4))
            .is_some()
    );
    assert!(
        cache
            .get(&key(0, 1, 1), now + Duration::from_secs(5))
            .is_none()
    );
}

#[test]
fn positive_ceiling_zero_high_bit_and_cname_ttl_are_enforced() {
    let now = Instant::now();
    for ttl in [0, 0x80000001, u32::MAX] {
        assert!(Template::build(&answer(ttl, now)).is_none(), "{ttl}");
    }
    let t = Template::build(&answer(7200, now)).unwrap();
    assert_eq!(
        t.reply(&request(), now).unwrap().message.answers[0].ttl,
        3600
    );
    assert!(!t.fresh(now + Duration::from_secs(3600)));
    let mut a = answer(60, now);
    a.message.answers.insert(
        0,
        Record::from_rdata(
            "example.test.".parse().unwrap(),
            2,
            RData::CNAME(CNAME("target.test.".parse().unwrap())),
        ),
    );
    let t = Template::build(&a).unwrap();
    assert!(!t.fresh(now + Duration::from_secs(2)));
}

#[test]
fn negative_lifetime_uses_soa_and_is_not_a_positive_answer() {
    let now = Instant::now();
    for code in [ResponseCode::NXDomain, ResponseCode::NoError] {
        for (ttl, minimum, expected) in [(120, 120, 60), (20, 80, 20), (90, 7, 7)] {
            let t = Template::build(&negative(code, ttl, minimum, now)).unwrap();
            let hit = t.reply(&request(), now + Duration::from_secs(1)).unwrap();
            assert_eq!(hit.message.metadata.response_code, code);
            assert_eq!(hit.message.authorities[0].ttl, expected - 1);
            let RData::SOA(data) = &hit.message.authorities[0].data else {
                panic!()
            };
            assert_eq!(data.minimum, minimum);
            assert!(!t.fresh(now + Duration::from_secs(u64::from(expected))));
        }
    }
    let mut a = negative(ResponseCode::NoError, 60, 60, now);
    a.message.answers.push(Record::from_rdata(
        "example.test.".parse().unwrap(),
        60,
        RData::CNAME(CNAME("target.test.".parse().unwrap())),
    ));
    assert!(
        Template::build(&a).is_none(),
        "CNAME NODATA entered the positive path"
    );
}

#[test]
fn ambiguous_negative_and_every_failure_rcode_bypass_cache() {
    let now = Instant::now();
    for code in [
        ResponseCode::ServFail,
        ResponseCode::Refused,
        ResponseCode::FormErr,
        ResponseCode::NotImp,
        ResponseCode::BADVERS,
    ] {
        let mut a = negative(ResponseCode::NXDomain, 60, 60, now);
        a.message.metadata.response_code = code;
        assert!(Template::build(&a).is_none(), "{code:?}");
    }
    for owner in ["unrelated.test.", "ample.test.", "example.test.bad."] {
        let mut a = negative(ResponseCode::NXDomain, 60, 60, now);
        a.message.authorities[0] = soa(owner, 60, 60);
        assert!(Template::build(&a).is_none(), "{owner}");
    }
    for owner in ["example.test.", "test.", "."] {
        let mut a = negative(ResponseCode::NXDomain, 60, 60, now);
        a.message.authorities[0] = soa(owner, 60, 60);
        assert!(Template::build(&a).is_some(), "{owner}");
    }
    for shape in 0..6 {
        let mut a = negative(ResponseCode::NXDomain, 60, 60, now);
        match shape {
            0 => a.message.authorities.clear(),
            1 => a.message.authorities.push(soa("test.", 60, 60)),
            2 => a.message.authorities.push(Record::from_rdata(
                "test.".parse().unwrap(),
                60,
                RData::NS(NS("ns.test.".parse().unwrap())),
            )),
            3 => a.message.answers.push(Record::from_rdata(
                "example.test.".parse().unwrap(),
                60,
                RData::CNAME(CNAME("target.test.".parse().unwrap())),
            )),
            4 => a.message.authorities[0].ttl = 0,
            _ => a.message.metadata.truncation = true,
        }
        assert!(Template::build(&a).is_none(), "shape {shape}");
    }
}

#[test]
fn reply_uses_current_id_question_rd_edns_and_does_not_claim_authenticity() {
    let now = Instant::now();
    let t = Template::build(&answer(60, now)).unwrap();
    let mut q = request();
    q.metadata.id = 987;
    q.metadata.recursion_desired = false;
    q.queries[0].name = Name::from_ascii("EXAMPLE.test.").unwrap();
    q.set_edns(Edns::new());
    let r = t.reply(&q, now).unwrap().message;
    assert_eq!(r.metadata.id, 987);
    assert_eq!(r.queries[0].name.to_ascii(), "EXAMPLE.test.");
    assert!(!r.metadata.recursion_desired);
    assert!(!r.metadata.authentic_data);
    assert!(!r.metadata.authoritative);
    assert!(r.metadata.recursion_available);
    assert_eq!(r.edns.unwrap().max_payload(), 1232);
    q.queries[0].query_type = hickory_proto::rr::RecordType::AAAA;
    assert!(t.reply(&q, now).is_none());
}

#[test]
fn oversized_template_is_not_silently_cached_as_truncated_success() {
    let now = Instant::now();
    let mut a = answer(60, now);
    for _ in 0..20 {
        a.message.answers.push(Record::from_rdata(
            "example.test.".parse().unwrap(),
            60,
            RData::TXT(TXT::new(vec!["x".repeat(250)])),
        ));
    }
    assert!(Template::build(&a).is_none());
}

#[test]
fn every_key_dimension_isolated_and_duplicate_writers_cannot_reset_age() {
    let now = Instant::now();
    let cache = Cache::new();
    let original = key(0, 1, 1);
    cache.insert(original, Template::build(&answer(60, now)).unwrap(), now);
    for dimension in 0..12 {
        let mut different = original;
        match dimension {
            0 => different.generation.node = id(99),
            1 => different.generation.snapshot = id(99),
            2 => different.generation.sequence = SnapshotSequence::new(2).unwrap(),
            3 => different.scope.endpoint = id(99),
            4 => different.scope.profile = id(99),
            5 => different.scope.group = id(99),
            6 => different.upstream = id(99),
            7 => different.resource = ContentHash::of(b"another"),
            8 => different.name[0] = 99,
            9 => different.name_len += 1,
            10 => different.query_type = 28,
            _ => different.class = 3,
        }
        assert!(
            cache.get(&different, now).is_none(),
            "dimension {dimension}"
        );
    }
    cache.insert(
        original,
        Template::build(&answer(7200, now + Duration::from_secs(5))).unwrap(),
        now + Duration::from_secs(5),
    );
    assert!(
        cache
            .get(&original, now + Duration::from_secs(60))
            .is_none()
    );
}

#[test]
fn fixed_storage_scope_fifo_and_global_fifo_obey_actual_allocation_bounds() {
    let now = Instant::now();
    let cache = Cache::new();
    let template = Template::build(&answer(60, now)).unwrap();
    {
        let state = cache.state.lock().unwrap();
        assert!(
            size_of::<Cache>()
                + state.as_ref().unwrap().slots.capacity() * size_of::<Option<Entry>>()
                <= BYTES
        );
        assert!(SCOPE_ENTRIES * size_of::<Option<Entry>>() <= SCOPE_BYTES);
        println!(
            "D54 Key={} Template={} UpstreamAnswer={} Outcome={} Result={}",
            size_of::<Key>(),
            size_of::<Template>(),
            size_of::<UpstreamAnswer>(),
            size_of::<crate::forward::Outcome>(),
            size_of::<crate::Result<crate::forward::Outcome>>()
        );
        println!(
            "D54 Cache={} Slot={} arena={} scope={}",
            size_of::<Cache>(),
            size_of::<Option<Entry>>(),
            state.as_ref().unwrap().slots.capacity() * size_of::<Option<Entry>>(),
            SCOPE_ENTRIES * size_of::<Option<Entry>>()
        );
    }
    for n in 0..=256 {
        cache.insert(key(n, 1, 1), template.copy(), now);
    }
    assert!(cache.get(&key(0, 1, 1), now).is_none());
    assert!(cache.get(&key(1, 1, 1), now).is_some());
    // Hits do not promote old entries.
    cache.insert(key(257, 1, 1), template.copy(), now);
    assert!(cache.get(&key(1, 1, 1), now).is_none());
    for scope in 2..=8 {
        for n in 0..256 {
            cache.insert(key(n, scope, 1), template.copy(), now);
        }
    }
    cache.insert(key(0, 9, 1), template.copy(), now);
    assert!(cache.get(&key(2, 1, 1), now).is_none());
    assert!(cache.get(&key(0, 2, 1), now).is_some());
    assert_eq!(
        cache
            .state
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .flatten()
            .count(),
        2048
    );
}

#[test]
fn retired_writers_cannot_clear_new_generation_and_expired_storage_is_reusable() {
    let now = Instant::now();
    let cache = Cache::new();
    let template = Template::build(&answer(60, now)).unwrap();
    cache.insert(key(0, 1, 1), template.copy(), now);
    cache.insert(key(0, 1, 2), template.copy(), now);
    assert!(cache.get(&key(0, 1, 1), now).is_none());
    cache.insert(key(1, 1, 1), template.copy(), now);
    assert!(cache.get(&key(0, 1, 2), now).is_some());
    let later = now + Duration::from_secs(60);
    cache.insert(
        key(2, 1, 2),
        Template::build(&answer(60, later)).unwrap(),
        later,
    );
    assert_eq!(
        cache
            .state
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .flatten()
            .count(),
        1
    );
    // A request cannot insert a candidate whose cleanup already consumed its TTL.
    cache.insert(key(3, 1, 2), template, later);
    assert!(cache.get(&key(3, 1, 2), later).is_none());
}

#[test]
fn lock_contention_is_nonwaiting_and_parallel_scopes_remain_bounded() {
    let now = Instant::now();
    let cache = Arc::new(Cache::new());
    let locked = cache.state.lock().unwrap();
    assert!(cache.get(&key(0, 1, 1), now).is_none());
    cache.insert(
        key(0, 1, 1),
        Template::build(&answer(60, now)).unwrap(),
        now,
    );
    assert_eq!(cache.failures.load(Ordering::Relaxed), 2);
    drop(locked);
    std::thread::scope(|scope| {
        for n in 1..=8 {
            let cache = Arc::clone(&cache);
            scope.spawn(move || {
                for name in 0..300 {
                    cache.insert(
                        key(name, n, 1),
                        Template::build(&answer(60, now)).unwrap(),
                        now,
                    );
                }
            });
        }
    });
    let state = cache.state.lock().unwrap();
    let state = state.as_ref().unwrap();
    assert!(state.slots.iter().flatten().count() <= 2048);
    for group in 1..=8 {
        assert!(
            state
                .slots
                .iter()
                .flatten()
                .filter(|entry| entry.key.scope.group == id(group))
                .count()
                <= 256
        );
    }
}

#[test]
fn fifo_counter_exhaustion_rebases_without_reusing_an_old_age() {
    let now = Instant::now();
    let cache = Cache::new();
    cache.insert(
        key(0, 1, 1),
        Template::build(&answer(60, now)).unwrap(),
        now,
    );
    cache.state.lock().unwrap().as_mut().unwrap().next = u64::MAX;
    cache.insert(key(1, 1, 1), Template::build(&answer(1, now)).unwrap(), now);
    assert!(cache.get(&key(0, 1, 1), now).is_none());
    assert!(
        cache
            .get(&key(1, 1, 1), now + Duration::from_secs(1))
            .is_none()
    );
}

#[test]
fn reply_ttl_accounts_for_the_whole_pending_send_window() {
    let now = Instant::now();
    let template = Template::build(&answer(60, now)).unwrap();
    let mut reply = template.reply(&request(), now).unwrap();
    assert_eq!(
        reply
            .prepare_send(now, now + Duration::from_secs(2))
            .unwrap(),
        now + Duration::from_secs(2)
    );
    assert_eq!(reply.message.answers[0].ttl, 58);
    let mut near = template
        .reply(&request(), now + Duration::from_millis(59500))
        .unwrap();
    assert_eq!(
        near.prepare_send(
            now + Duration::from_millis(59500),
            now + Duration::from_secs(61)
        )
        .unwrap(),
        now + Duration::from_secs(60)
    );
    assert_eq!(near.message.answers[0].ttl, 0);
    assert_eq!(
        near.prepare_send(now + Duration::from_secs(60), now + Duration::from_secs(61))
            .unwrap_err()
            .code(),
        ErrorCode::Timeout
    );
}
