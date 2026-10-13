//! Responses Veyra writes itself (spec §9.5): synthesized addresses, NODATA
//! with a synthesized SOA, and error codes.

use hickory_proto::op::{Edns, Message, ResponseCode};
use hickory_proto::rr::rdata::{A, AAAA, SOA};
use hickory_proto::rr::{Name, RData, Record};
use std::net::IpAddr;

/// The largest UDP reply Veyra sends or advertises (DNS Flag Day 2020).
pub(crate) const MAX_UDP_PAYLOAD: u16 = 1232;

/// A reply skeleton for `request`: same id, opcode, question and RD; RA set;
/// AA and AD clear, since synthesized data is never DNSSEC-validated (spec §9.5);
/// an OPT record when the request had one, echoing its DO bit.
pub(crate) fn reply(request: &Message, code: ResponseCode) -> Message {
    let mut reply = Message::response(request.metadata.id, request.metadata.op_code);
    reply.metadata.recursion_desired = request.metadata.recursion_desired;
    reply.metadata.recursion_available = true;
    reply.metadata.response_code = code;
    reply.add_queries(request.queries.iter().cloned());
    if let Some(asked) = &request.edns {
        let mut edns = Edns::new();
        edns.set_max_payload(MAX_UDP_PAYLOAD);
        edns.set_dnssec_ok(asked.flags().dnssec_ok);
        reply.set_edns(edns);
    }
    reply
}

/// A and AAAA records for the question name.
pub(crate) fn addresses(request: &Message, ips: &[IpAddr], ttl: u32) -> Message {
    let mut reply = reply(request, ResponseCode::NoError);
    if let Some(query) = request.queries.first() {
        for ip in ips {
            let data = match ip {
                IpAddr::V4(v4) => RData::A(A(*v4)),
                IpAddr::V6(v6) => RData::AAAA(AAAA(*v6)),
            };
            reply.add_answer(Record::from_rdata(query.name().clone(), ttl, data));
        }
    }
    reply
}

/// NOERROR without records. The SOA's TTL and minimum are both `ttl`, so
/// negative caching (the smaller of the two, RFC 2308) lasts `ttl` (spec §9.5).
pub(crate) fn no_data(request: &Message, ttl: u32) -> Message {
    let mut reply = reply(request, ResponseCode::NoError);
    if let Some(query) = request.queries.first() {
        let name = |text: &str| Name::from_ascii(text).unwrap_or_else(|_| Name::root());
        let soa = SOA::new(
            name("veyra.invalid."),
            name("hostmaster.veyra.invalid."),
            1,
            3600,
            600,
            86400,
            ttl,
        );
        reply.add_authority(Record::from_rdata(
            query.name().clone(),
            ttl,
            RData::SOA(soa),
        ));
    }
    reply
}
