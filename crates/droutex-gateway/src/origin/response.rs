//! Bounded DNS response and CNAME-chain interpretation; no network or dial authority.
//!
//! A transport must independently bind each response to its exact upstream peer,
//! query, deadline and current admission. These results are untrusted DNS data;
//! they must be screened against a fresh self inventory before every numeric dial.
use droutex_policy::CanonicalDomain;
use hickory_proto::{
    op::{Header, Message, MessageType, OpCode, Query, ResponseCode},
    rr::{DNSClass, Name, RData, Record, RecordType},
    serialize::binary::{BinDecodable, BinDecoder},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    net::IpAddr,
};

const MAX_MESSAGE: usize = 4096;
const MAX_RECORDS: usize = 64;
const MAX_ALIASES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryKind {
    Ipv4,
    Ipv6,
}
impl QueryKind {
    pub(super) fn record_type(self) -> RecordType {
        match self {
            Self::Ipv4 => RecordType::A,
            Self::Ipv6 => RecordType::AAAA,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseError {
    InvalidName,
    Malformed,
    Mismatch,
    Unsupported,
    Truncated,
    Ambiguous,
    AliasLoop,
    AliasLimit,
    NoAddress,
}
impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "origin DNS response {self:?}")
    }
}
impl Error for ResponseError {}

/// Data only: no address in this result is checked against node inventory or authorized for dialing.
#[derive(Debug, Eq, PartialEq)]
pub enum ResponseStep {
    Follow(String),
    Addresses(Vec<IpAddr>),
}

/// Tracks canonical alias history across replies, never across separate resolutions.
pub struct ResponseChain {
    current: CanonicalDomain,
    kind: QueryKind,
    visited: BTreeSet<CanonicalDomain>,
    finished: bool,
}
impl ResponseChain {
    pub fn new(name: &str, kind: QueryKind) -> Result<Self, ResponseError> {
        let current: CanonicalDomain = name.parse().map_err(|_| ResponseError::InvalidName)?;
        let mut visited = BTreeSet::new();
        visited.insert(current.clone());
        Ok(Self {
            current,
            kind,
            visited,
            finished: false,
        })
    }
    pub fn name(&self) -> &str {
        self.current.as_str()
    }

    /// Consume a complete reply to this exact question. No remote fallback is implied.
    pub fn consume(
        &mut self,
        bytes: &[u8],
        expected_id: u16,
    ) -> Result<ResponseStep, ResponseError> {
        if self.finished {
            return Err(ResponseError::Unsupported);
        }
        if bytes.len() > MAX_MESSAGE {
            return Err(ResponseError::Malformed);
        }
        let mut preflight = BinDecoder::new(bytes);
        let header = Header::read(&mut preflight).map_err(|_| ResponseError::Malformed)?;
        if bytes[3] & 0x40 != 0 {
            return Err(ResponseError::Malformed);
        }
        let count = usize::from(header.counts.answers)
            + usize::from(header.counts.authorities)
            + usize::from(header.counts.additionals);
        if header.counts.queries != 1 || count > MAX_RECORDS {
            return Err(ResponseError::Malformed);
        }
        if header.metadata.id != expected_id
            || header.metadata.message_type != MessageType::Response
            || header.metadata.op_code != OpCode::Query
        {
            return Err(ResponseError::Mismatch);
        }
        if header.metadata.response_code != ResponseCode::NoError {
            return Err(ResponseError::Unsupported);
        }
        validate_wire_records(&mut preflight, &header)?;
        let mut decoder = BinDecoder::new(bytes);
        let message = Message::read(&mut decoder).map_err(|_| ResponseError::Malformed)?;
        if !decoder.is_empty() {
            return Err(ResponseError::Malformed);
        }
        let query = &message.queries[0];
        if query.query_class != DNSClass::IN
            || query.query_type != self.kind.record_type()
            || canonical(&query.name.to_ascii())? != self.current
        {
            return Err(ResponseError::Mismatch);
        }
        if header.metadata.truncation {
            return Err(ResponseError::Truncated);
        }
        let mut aliases = BTreeMap::new();
        let mut addresses: BTreeMap<CanonicalDomain, Vec<IpAddr>> = BTreeMap::new();
        let mut other_owners = BTreeSet::new();
        for record in &message.answers {
            if record.dns_class != DNSClass::IN {
                return Err(ResponseError::Malformed);
            }
            let owner = canonical(&record.name.to_ascii())?;
            // Conservative subset: any other answer at a CNAME owner is
            // ambiguous here, including DNSSEC data. It is never used to dial.
            if !matches!(record.data, RData::CNAME(_)) {
                other_owners.insert(owner.clone());
            }
            match &record.data {
                RData::CNAME(target) => {
                    let target = canonical(&target.0.to_ascii())?;
                    if aliases.insert(owner, target).is_some() {
                        return Err(ResponseError::Ambiguous);
                    }
                }
                RData::A(ip) if self.kind == QueryKind::Ipv4 => {
                    addresses.entry(owner).or_default().push(IpAddr::V4(ip.0))
                }
                RData::AAAA(ip) if self.kind == QueryKind::Ipv6 => {
                    addresses.entry(owner).or_default().push(IpAddr::V6(ip.0))
                }
                _ => (),
            }
        }
        let mut current = self.current.clone();
        let mut visited = self.visited.clone();
        loop {
            let alias = aliases.get(&current);
            let values = addresses.get(&current);
            if alias.is_some() && other_owners.contains(&current) {
                return Err(ResponseError::Ambiguous);
            }
            if let Some(values) = values {
                if values.len() > super::MAX_ANSWER_ADDRESSES {
                    return Err(ResponseError::Malformed);
                }
                self.current = current;
                self.visited = visited;
                self.finished = true;
                return Ok(ResponseStep::Addresses(values.clone()));
            }
            let Some(next) = alias else {
                if current == self.current {
                    return Err(ResponseError::NoAddress);
                }
                self.current = current;
                self.visited = visited;
                return Ok(ResponseStep::Follow(self.current.as_str().to_owned()));
            };
            if !visited.insert(next.clone()) {
                return Err(ResponseError::AliasLoop);
            }
            if visited.len() > MAX_ALIASES + 1 {
                return Err(ResponseError::AliasLimit);
            }
            current = next.clone();
        }
    }
}
fn canonical(name: &str) -> Result<CanonicalDomain, ResponseError> {
    name.parse().map_err(|_| ResponseError::InvalidName)
}

fn validate_wire_records(
    decoder: &mut BinDecoder<'_>,
    header: &Header,
) -> Result<(), ResponseError> {
    Query::read(decoder).map_err(|_| ResponseError::Malformed)?;
    let mut opt_seen = false;
    for (section, count) in [
        header.counts.answers,
        header.counts.authorities,
        header.counts.additionals,
    ]
    .into_iter()
    .enumerate()
    {
        for _ in 0..count {
            let index = u16::try_from(decoder.index()).map_err(|_| ResponseError::Malformed)?;
            let mut owner = decoder.clone(index);
            Name::read(&mut owner).map_err(|_| ResponseError::Malformed)?;
            let data_start = owner
                .index()
                .checked_add(10)
                .ok_or(ResponseError::Malformed)?;
            let record = Record::read(decoder).map_err(|_| ResponseError::Malformed)?;
            match &record.data {
                RData::OPT(options) => {
                    if section != 2 || opt_seen || !record.name.is_root() || record.ttl >> 16 != 0 {
                        return Err(ResponseError::Unsupported);
                    }
                    opt_seen = true;
                    let decoded = options
                        .as_ref()
                        .iter()
                        .map(|(_, option)| 4 + usize::from(option.len()))
                        .sum::<usize>();
                    if decoder.index().checked_sub(data_start) != Some(decoded) {
                        return Err(ResponseError::Malformed);
                    }
                }
                RData::Update0(RecordType::OPT) => {
                    if section != 2 || opt_seen || !record.name.is_root() || record.ttl >> 16 != 0 {
                        return Err(ResponseError::Unsupported);
                    }
                    opt_seen = true;
                    if decoder.index() != data_start {
                        return Err(ResponseError::Malformed);
                    }
                }
                RData::Update0(_) => return Err(ResponseError::Malformed),
                _ if record.dns_class != DNSClass::IN => return Err(ResponseError::Malformed),
                _ => (),
            }
        }
    }
    if !decoder.is_empty() {
        return Err(ResponseError::Malformed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_proto::{
        op::Query,
        rr::{
            Name, Record,
            rdata::{A, AAAA, CNAME},
        },
    };
    fn name(s: &str) -> Name {
        s.parse().unwrap()
    }
    fn reply(domain: &str, kind: QueryKind, records: Vec<Record>) -> Vec<u8> {
        let mut message = Message::new(42, MessageType::Response, OpCode::Query);
        message.add_query(Query::query(name(domain), kind.record_type()));
        for record in records {
            message.add_answer(record);
        }
        message.to_vec().unwrap()
    }
    fn alias(from: &str, to: &str) -> Record {
        Record::from_rdata(name(from), 30, RData::CNAME(CNAME(name(to))))
    }
    fn address(from: &str, addr: &str) -> Record {
        let data = match addr.parse::<IpAddr>().unwrap() {
            IpAddr::V4(ip) => RData::A(A(ip)),
            IpAddr::V6(ip) => RData::AAAA(AAAA(ip)),
        };
        Record::from_rdata(name(from), 30, data)
    }
    #[test]
    fn cname_chain_uses_only_reachable_answers_and_preserves_query_family() {
        let mut chain = ResponseChain::new("Origin.Example.", QueryKind::Ipv4).unwrap();
        let wire = reply(
            "origin.example.",
            QueryKind::Ipv4,
            vec![
                address("unrelated.example.", "9.9.9.9"),
                alias("origin.example.", "cdn.example."),
                address("cdn.example.", "1.1.1.1"),
                address("cdn.example.", "2606:4700:4700::1111"),
            ],
        );
        assert_eq!(
            chain.consume(&wire, 42).unwrap(),
            ResponseStep::Addresses(vec!["1.1.1.1".parse().unwrap()])
        );
        assert_eq!(chain.consume(&wire, 42), Err(ResponseError::Unsupported));
    }
    #[test]
    fn cname_history_survives_followups_and_refuses_loops_without_mutating_state() {
        let mut chain = ResponseChain::new("one.example", QueryKind::Ipv6).unwrap();
        let first = reply(
            "one.example.",
            QueryKind::Ipv6,
            vec![alias("one.example.", "two.example.")],
        );
        assert_eq!(
            chain.consume(&first, 42).unwrap(),
            ResponseStep::Follow("two.example".into())
        );
        let looped = reply(
            "two.example.",
            QueryKind::Ipv6,
            vec![alias("two.example.", "one.example.")],
        );
        assert_eq!(chain.consume(&looped, 42), Err(ResponseError::AliasLoop));
        assert_eq!(chain.name(), "two.example");
        let valid = reply(
            "two.example.",
            QueryKind::Ipv6,
            vec![address("two.example.", "2606:4700:4700::1111")],
        );
        assert_eq!(
            chain.consume(&valid, 42).unwrap(),
            ResponseStep::Addresses(vec!["2606:4700:4700::1111".parse().unwrap()])
        );
    }
    #[test]
    fn refuses_wrong_id_question_type_trailing_bytes_and_ambiguous_records() {
        let original = reply(
            "one.example.",
            QueryKind::Ipv4,
            vec![address("one.example.", "1.1.1.1")],
        );
        let mut chain = ResponseChain::new("one.example", QueryKind::Ipv4).unwrap();
        assert_eq!(chain.consume(&original, 43), Err(ResponseError::Mismatch));
        let other = reply(
            "two.example.",
            QueryKind::Ipv4,
            vec![address("two.example.", "1.1.1.1")],
        );
        assert_eq!(chain.consume(&other, 42), Err(ResponseError::Mismatch));
        let ipv6 = reply(
            "one.example.",
            QueryKind::Ipv6,
            vec![address("one.example.", "2606:4700:4700::1111")],
        );
        assert_eq!(chain.consume(&ipv6, 42), Err(ResponseError::Mismatch));
        let mut trailing = original.clone();
        trailing.push(0);
        assert_eq!(chain.consume(&trailing, 42), Err(ResponseError::Malformed));
        let both = reply(
            "one.example.",
            QueryKind::Ipv4,
            vec![
                alias("one.example.", "two.example."),
                address("one.example.", "1.1.1.1"),
            ],
        );
        assert_eq!(chain.consume(&both, 42), Err(ResponseError::Ambiguous));
        let repeated = reply(
            "one.example.",
            QueryKind::Ipv4,
            vec![
                alias("one.example.", "two.example."),
                alias("one.example.", "three.example."),
            ],
        );
        assert_eq!(chain.consume(&repeated, 42), Err(ResponseError::Ambiguous));
        let cross_family = reply(
            "one.example.",
            QueryKind::Ipv4,
            vec![
                alias("one.example.", "two.example."),
                address("one.example.", "2606:4700:4700::1111"),
            ],
        );
        assert_eq!(
            chain.consume(&cross_family, 42),
            Err(ResponseError::Ambiguous)
        );
        let mut reserved = original.clone();
        reserved[3] |= 0x40;
        assert_eq!(chain.consume(&reserved, 42), Err(ResponseError::Malformed));
    }
    #[test]
    fn eight_alias_hop_limit_spans_separate_responses() {
        let mut chain = ResponseChain::new("n0.example", QueryKind::Ipv4).unwrap();
        for hop in 0..8 {
            let from = format!("n{hop}.example.");
            let to = format!("n{}.example.", hop + 1);
            let message = reply(&from, QueryKind::Ipv4, vec![alias(&from, &to)]);
            assert!(matches!(
                chain.consume(&message, 42),
                Ok(ResponseStep::Follow(_))
            ));
        }
        let ninth = reply(
            "n8.example.",
            QueryKind::Ipv4,
            vec![alias("n8.example.", "n9.example.")],
        );
        assert_eq!(chain.consume(&ninth, 42), Err(ResponseError::AliasLimit));
    }

    #[test]
    fn fresh_resolution_must_be_screened_again_when_answers_change() {
        fn lookup(ip: &str) -> Vec<IpAddr> {
            let wire = reply(
                "one.example.",
                QueryKind::Ipv4,
                vec![address("one.example.", ip)],
            );
            let mut chain = ResponseChain::new("one.example", QueryKind::Ipv4).unwrap();
            let ResponseStep::Addresses(addresses) = chain.consume(&wire, 42).unwrap() else {
                panic!("expected addresses")
            };
            addresses
        }
        assert!(super::super::screen_resolution(&lookup("1.1.1.1"), &[]).is_ok());
        assert_eq!(
            super::super::screen_resolution(&lookup("10.0.0.1"), &[]),
            Err(super::super::ScreeningError::UnsafeAddress)
        );
    }

    #[test]
    fn rejects_truncation_rcode_excess_records_and_incomplete_additional_data() {
        let original = reply(
            "one.example.",
            QueryKind::Ipv4,
            vec![address("one.example.", "1.1.1.1")],
        );
        let mut chain = ResponseChain::new("one.example", QueryKind::Ipv4).unwrap();
        let mut truncated = original.clone();
        truncated[2] |= 0x02;
        assert_eq!(chain.consume(&truncated, 42), Err(ResponseError::Truncated));
        assert_eq!(chain.consume(&truncated, 43), Err(ResponseError::Mismatch));
        let mut wrong_question = reply("two.example.", QueryKind::Ipv4, vec![]);
        wrong_question[2] |= 0x02;
        assert_eq!(
            chain.consume(&wrong_question, 42),
            Err(ResponseError::Mismatch)
        );
        let mut nxdomain = original.clone();
        nxdomain[3] |= 0x03;
        assert_eq!(
            chain.consume(&nxdomain, 42),
            Err(ResponseError::Unsupported)
        );
        let mut excessive = original.clone();
        excessive[6..8].copy_from_slice(&65_u16.to_be_bytes());
        assert_eq!(chain.consume(&excessive, 42), Err(ResponseError::Malformed));
        let mut additional = original.clone();
        additional[10..12].copy_from_slice(&1_u16.to_be_bytes());
        assert_eq!(
            chain.consume(&additional, 42),
            Err(ResponseError::Malformed)
        );
        assert!(matches!(
            chain.consume(&original, 42),
            Ok(ResponseStep::Addresses(_))
        ));
    }

    #[test]
    fn accepts_valid_edns_and_rejects_malformed_options_or_extended_errors() {
        let original = reply(
            "one.example.",
            QueryKind::Ipv4,
            vec![address("one.example.", "1.1.1.1")],
        );
        let mut edns = original.clone();
        edns[10..12].copy_from_slice(&1_u16.to_be_bytes());
        let opt_start = edns.len();
        edns.extend_from_slice(&[0, 0, 41, 4, 208, 0, 0, 0, 0, 0, 0]);
        let mut malformed = edns.clone();
        // A complete option code without its length is silently discarded by
        // the codec unless the caller accounts for the original RDATA bytes.
        malformed[opt_start + 10] = 2;
        malformed.extend_from_slice(&[0, 1]);
        let mut extended_error = edns.clone();
        extended_error[opt_start + 5] = 1;
        let mut duplicate = edns.clone();
        duplicate[10..12].copy_from_slice(&2_u16.to_be_bytes());
        duplicate.extend_from_slice(&[0, 0, 41, 4, 208, 0, 0, 0, 0, 0, 0]);
        let mut chain = ResponseChain::new("one.example", QueryKind::Ipv4).unwrap();
        assert_eq!(chain.consume(&malformed, 42), Err(ResponseError::Malformed));
        assert_eq!(
            chain.consume(&extended_error, 42),
            Err(ResponseError::Unsupported)
        );
        assert_eq!(
            chain.consume(&duplicate, 42),
            Err(ResponseError::Unsupported)
        );
        assert!(matches!(
            chain.consume(&edns, 42),
            Ok(ResponseStep::Addresses(_))
        ));
    }

    #[test]
    fn additional_glue_is_not_used_as_an_origin_answer() {
        let mut message = Message::new(42, MessageType::Response, OpCode::Query);
        message.add_query(Query::query(name("one.example."), RecordType::A));
        message.add_additional(address("one.example.", "1.1.1.1"));
        let mut chain = ResponseChain::new("one.example", QueryKind::Ipv4).unwrap();
        assert_eq!(
            chain.consume(&message.to_vec().unwrap(), 42),
            Err(ResponseError::NoAddress)
        );
    }
}
