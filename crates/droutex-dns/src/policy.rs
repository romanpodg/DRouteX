//! Profile-scoped local answers; these records carry no gateway authority.
use crate::{DnsError, ErrorCode, Result, wire};
use droutex_common::ProfileId;
use droutex_policy::{CanonicalDomain, PreparedPrivateRecord, PrivateRecordValue};
use hickory_proto::{
    op::{Message, ResponseCode},
    rr::{
        RData, Record, RecordType,
        rdata::{A, AAAA, CNAME},
    },
};

pub(super) fn local_answer(
    request: &Message,
    profile: ProfileId,
    records: &[PreparedPrivateRecord],
    budget: &crate::budget::Budget,
) -> Result<Option<Message>> {
    let question = request
        .queries
        .first()
        .ok_or(DnsError(ErrorCode::Malformed))?;
    let domain: CanonicalDomain = question
        .name
        .to_ascii()
        .parse()
        .map_err(|_| DnsError(ErrorCode::Scope))?;
    if !records
        .iter()
        .any(|record| record.profile == profile && record.owner == domain)
    {
        return Ok(None);
    }
    let _credit = budget.wire(4 * 1024 * 1024)?;
    let mut reply = wire::response(request, ResponseCode::NoError);
    let mut unique = std::collections::HashSet::new();
    for record in records
        .iter()
        .filter(|record| record.profile == profile && record.owner == domain)
    {
        // Identical records are legal input but must not be duplicated in an RRset.
        if let Some(data) = record_data(&record.value, question.query_type)?
            && unique.insert(data.clone())
        {
            reply
                .answers
                .push(Record::from_rdata(question.name.clone(), record.ttl, data));
        }
    }
    if !wire::fits_complete(&reply, crate::MAX_TCP_BYTES) {
        return Err(DnsError(ErrorCode::Unavailable));
    }
    Ok(Some(reply))
}

fn record_data(value: &PrivateRecordValue, query_type: RecordType) -> Result<Option<RData>> {
    Ok(match value {
        PrivateRecordValue::Ipv4(ip) if query_type == RecordType::A => Some(RData::A(A(*ip))),
        PrivateRecordValue::Ipv6(ip) if query_type == RecordType::AAAA => {
            Some(RData::AAAA(AAAA(*ip)))
        }
        PrivateRecordValue::Cname(target) => {
            let name = format!("{}.", target.as_str())
                .parse()
                .map_err(|_| DnsError(ErrorCode::State))?;
            Some(RData::CNAME(CNAME(name)))
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use droutex_common::PrivateDnsRecordId;
    use hickory_proto::{
        op::{MessageType, OpCode, Query},
        rr::Name,
    };

    fn id<T: std::str::FromStr>(n: u8) -> T
    where
        T::Err: std::fmt::Debug,
    {
        format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
    }
    fn record(
        id_num: u8,
        profile: u8,
        owner: &str,
        value: PrivateRecordValue,
    ) -> PreparedPrivateRecord {
        PreparedPrivateRecord {
            id: id::<PrivateDnsRecordId>(id_num),
            profile: id(profile),
            owner: owner.parse().unwrap(),
            ttl: 42,
            value,
        }
    }
    fn query(name: &str, kind: RecordType) -> Message {
        let mut message = Message::new(1, MessageType::Query, OpCode::Query);
        message.add_query(Query::query(name.parse::<Name>().unwrap(), kind));
        message
    }

    #[test]
    fn local_records_are_scoped_and_preserve_types_and_aliases() {
        let records = vec![
            record(
                1,
                1,
                "example.test",
                PrivateRecordValue::Ipv4("192.0.2.1".parse().unwrap()),
            ),
            record(
                2,
                1,
                "example.test",
                PrivateRecordValue::Ipv6("2001:db8::1".parse().unwrap()),
            ),
            record(
                3,
                2,
                "example.test",
                PrivateRecordValue::Ipv4("192.0.2.2".parse().unwrap()),
            ),
            record(
                4,
                1,
                "alias.test",
                PrivateRecordValue::Cname("example.test".parse().unwrap()),
            ),
        ];
        let budget = crate::budget::Budget::new();
        let answer = local_answer(
            &query("ExAmPlE.test.", RecordType::A),
            id(1),
            &records,
            &budget,
        )
        .unwrap()
        .unwrap();
        assert_eq!(answer.answers.len(), 1);
        assert_eq!(
            answer.answers[0].data,
            RData::A(A("192.0.2.1".parse().unwrap()))
        );
        assert_eq!(answer.answers[0].ttl, 42);
        let other = local_answer(
            &query("example.test.", RecordType::A),
            id(2),
            &records,
            &budget,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            other.answers[0].data,
            RData::A(A("192.0.2.2".parse().unwrap()))
        );
        let ipv6 = local_answer(
            &query("example.test.", RecordType::AAAA),
            id(1),
            &records,
            &budget,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            ipv6.answers[0].data,
            RData::AAAA(AAAA("2001:db8::1".parse().unwrap()))
        );
        assert!(
            local_answer(
                &query("example.test.", RecordType::TXT),
                id(1),
                &records,
                &budget
            )
            .unwrap()
            .unwrap()
            .answers
            .is_empty()
        );
        assert!(
            local_answer(
                &query("example.test.", RecordType::A),
                id(3),
                &records,
                &budget
            )
            .unwrap()
            .is_none()
        );
        let alias = local_answer(
            &query("alias.test.", RecordType::AAAA),
            id(1),
            &records,
            &budget,
        )
        .unwrap()
        .unwrap();
        assert!(
            matches!(&alias.answers[0].data, RData::CNAME(target) if target.0.to_ascii() == "example.test.")
        );
        assert!(!alias.metadata.authentic_data);
    }
}
