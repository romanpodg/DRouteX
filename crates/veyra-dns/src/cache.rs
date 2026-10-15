//! Upstream answers cached for their TTL (spec §9.3). Synthesized answers are
//! never cached: they come from the policy on every query.

use hickory_proto::op::{Message, ResponseCode};
use hickory_proto::rr::{DNSClass, RData, RecordType};
use std::collections::HashMap;
use std::time::Instant;

/// No answer is kept longer than a day.
const MAX_TTL: u32 = 86_400;

/// Everything that changes what the upstream answers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    name: String,
    record_type: RecordType,
    class: DNSClass,
    group: String,
    edns: bool,
    dnssec_ok: bool,
    checking_disabled: bool,
}

impl Key {
    /// `None` for a request without a question.
    pub(crate) fn new(request: &Message, group: &str) -> Option<Key> {
        let query = request.queries.first()?;
        Some(Key {
            name: query.name().to_lowercase().to_ascii(),
            record_type: query.query_type(),
            class: query.query_class(),
            group: group.into(),
            edns: request.edns.is_some(),
            dnssec_ok: request.edns.as_ref().is_some_and(|e| e.flags().dnssec_ok),
            checking_disabled: request.metadata.checking_disabled,
        })
    }
}

struct Entry {
    reply: Message,
    stored: Instant,
    ttl: u32,
}

pub(crate) struct Cache {
    entries: HashMap<Key, Entry>,
    capacity: usize,
}

impl Cache {
    pub(crate) fn new(capacity: usize) -> Cache {
        Cache {
            entries: HashMap::new(),
            capacity,
        }
    }

    /// The cached reply with every TTL reduced by its age, or `None` once expired.
    pub(crate) fn get(&mut self, key: &Key, now: Instant) -> Option<Message> {
        let entry = self.entries.get(key)?;
        let age = age(entry.stored, now);
        if age >= entry.ttl {
            self.entries.remove(key);
            return None;
        }
        let mut reply = entry.reply.clone();
        let records = reply
            .answers
            .iter_mut()
            .chain(reply.authorities.iter_mut())
            .chain(reply.additionals.iter_mut());
        for record in records {
            record.ttl = record.ttl.saturating_sub(age);
        }
        Some(reply)
    }

    // ponytail: no eviction of live entries; a full cache stops caching new
    // answers until some expire. Add LRU if hit rates ever matter.
    pub(crate) fn put(&mut self, key: Key, reply: &Message, now: Instant) {
        let Some(ttl) = cache_ttl(reply) else {
            return;
        };
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&key) {
            self.entries.retain(|_, e| age(e.stored, now) < e.ttl);
            if self.entries.len() >= self.capacity {
                return;
            }
        }
        let entry = Entry {
            reply: reply.clone(),
            stored: now,
            ttl,
        };
        self.entries.insert(key, entry);
    }
}

fn age(stored: Instant, now: Instant) -> u32 {
    u32::try_from(now.saturating_duration_since(stored).as_secs()).unwrap_or(u32::MAX)
}

/// RFC 2308: positive answers live for their smallest TTL; NXDOMAIN and NODATA
/// for the smaller of the SOA's TTL and minimum. SERVFAIL, truncated replies and
/// anything else are not cached.
fn cache_ttl(reply: &Message) -> Option<u32> {
    if reply.metadata.truncation {
        return None;
    }
    let ttl = match reply.metadata.response_code {
        ResponseCode::NoError if !reply.answers.is_empty() => {
            reply.answers.iter().map(|r| r.ttl).min()?
        }
        ResponseCode::NoError | ResponseCode::NXDomain => {
            reply.authorities.iter().find_map(|r| match &r.data {
                RData::SOA(soa) => Some(r.ttl.min(soa.minimum)),
                _ => None,
            })?
        }
        _ => return None,
    };
    Some(ttl.min(MAX_TTL)).filter(|ttl| *ttl > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_proto::op::{Edns, MessageType, OpCode, Query};
    use hickory_proto::rr::rdata::{A, SOA};
    use hickory_proto::rr::{Name, Record};
    use std::net::Ipv4Addr;
    use std::time::Duration;

    fn request(name: &str, record_type: RecordType) -> Message {
        let mut message = Message::new(1, MessageType::Query, OpCode::Query);
        message.add_query(Query::query(Name::from_ascii(name).unwrap(), record_type));
        message
    }

    fn answer(ttls: &[u32]) -> Message {
        let request = request("example.org.", RecordType::A);
        let mut reply = Message::response(1, OpCode::Query);
        reply.add_queries(request.queries.clone());
        for ttl in ttls {
            let data = RData::A(A(Ipv4Addr::LOCALHOST));
            reply.add_answer(Record::from_rdata(
                request.queries[0].name().clone(),
                *ttl,
                data,
            ));
        }
        reply
    }

    fn negative(code: ResponseCode, soa_ttl: u32, minimum: u32) -> Message {
        let mut reply = answer(&[]);
        reply.metadata.response_code = code;
        let soa = SOA::new(Name::root(), Name::root(), 1, 1, 1, 1, minimum);
        reply.add_authority(Record::from_rdata(Name::root(), soa_ttl, RData::SOA(soa)));
        reply
    }

    #[test]
    fn positive_answers_live_for_their_smallest_ttl_capped_at_a_day() {
        assert_eq!(cache_ttl(&answer(&[300, 60])), Some(60));
        assert_eq!(cache_ttl(&answer(&[1_000_000])), Some(86_400));
        assert_eq!(cache_ttl(&answer(&[0])), None);
    }

    #[test]
    fn negative_answers_live_for_the_smaller_soa_ttl() {
        assert_eq!(
            cache_ttl(&negative(ResponseCode::NXDomain, 900, 60)),
            Some(60)
        );
        assert_eq!(
            cache_ttl(&negative(ResponseCode::NoError, 30, 600)),
            Some(30)
        );
        assert_eq!(cache_ttl(&answer(&[])), None, "NODATA without a SOA");
    }

    #[test]
    fn failures_and_truncated_replies_are_not_cached() {
        assert_eq!(cache_ttl(&negative(ResponseCode::ServFail, 60, 60)), None);
        let mut truncated = answer(&[300]);
        truncated.metadata.truncation = true;
        assert_eq!(cache_ttl(&truncated), None);
    }

    #[test]
    fn hits_age_their_ttls_and_expire() {
        let mut cache = Cache::new(10);
        let key = Key::new(&request("example.org.", RecordType::A), "default").unwrap();
        let start = Instant::now();
        cache.put(key.clone(), &answer(&[300, 60]), start);
        let hit = cache.get(&key, start + Duration::from_secs(20)).unwrap();
        let ttls: Vec<u32> = hit.answers.iter().map(|r| r.ttl).collect();
        assert_eq!(ttls, vec![280, 40]);
        assert!(cache.get(&key, start + Duration::from_secs(60)).is_none());
    }

    #[test]
    fn keys_ignore_name_case_but_not_type_group_or_dnssec() {
        let base = Key::new(&request("Example.ORG.", RecordType::A), "default");
        assert_eq!(
            base,
            Key::new(&request("example.org.", RecordType::A), "default")
        );
        assert_ne!(
            base,
            Key::new(&request("example.org.", RecordType::AAAA), "default")
        );
        assert_ne!(
            base,
            Key::new(&request("example.org.", RecordType::A), "corp")
        );
        let mut dnssec = request("example.org.", RecordType::A);
        let mut edns = Edns::new();
        edns.set_dnssec_ok(true);
        dnssec.set_edns(edns);
        assert_ne!(base, Key::new(&dnssec, "default"));
    }

    #[test]
    fn a_full_cache_makes_room_only_by_dropping_expired_entries() {
        let mut cache = Cache::new(1);
        let start = Instant::now();
        let first = Key::new(&request("a.example.", RecordType::A), "default").unwrap();
        let second = Key::new(&request("b.example.", RecordType::A), "default").unwrap();
        cache.put(first, &answer(&[60]), start);
        let later = start + Duration::from_secs(1);
        cache.put(second.clone(), &answer(&[60]), later);
        assert!(cache.get(&second, later).is_none(), "full of live entries");
        let expired = start + Duration::from_secs(61);
        cache.put(second.clone(), &answer(&[60]), expired);
        assert!(
            cache.get(&second, expired).is_some(),
            "the expired entry made room"
        );
    }
}
