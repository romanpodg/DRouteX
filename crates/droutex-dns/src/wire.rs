use crate::{DnsError, ErrorCode, MAX_TCP_BYTES, MAX_UDP_BYTES, Result};
use droutex_config::DnsAction;
use droutex_core::deployment::{AdmissionScope, AuthorizedRequest, Decision};
use droutex_policy::{CanonicalDomain, PolicySelection};
use hickory_proto::{
    op::{Edns, Header, Message, MessageType, OpCode, Query, ResponseCode},
    rr::{DNSClass, Name, RData, Record, RecordType},
    serialize::binary::{BinDecodable, BinDecoder, BinEncodable, BinEncoder},
};
use std::future::Future;

// Derived from the accepted subset: header + maximum wire name/question + OPT
// header + 4096 option bytes. The TCP framing limit remains 65535.
const MAX_QUERY_BYTES: usize = 12 + 255 + 4 + 12 + 4096;

pub(super) trait Processor: Send + Sync + 'static {
    fn answer(
        &self,
        request: &Message,
        selection: PolicySelection<'_, DnsAction>,
        decision: &Decision<'_>,
    ) -> impl Future<Output = Result<Message>> + Send;
}
pub(super) struct UnavailableProcessor;
impl Processor for UnavailableProcessor {
    async fn answer(
        &self,
        request: &Message,
        _: PolicySelection<'_, DnsAction>,
        _: &Decision<'_>,
    ) -> Result<Message> {
        Ok(response(request, ResponseCode::ServFail))
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<Message> {
    let malformed = || DnsError(ErrorCode::Malformed);
    if bytes.len() > MAX_QUERY_BYTES {
        return Err(malformed());
    }
    let mut decoder = BinDecoder::new(bytes);
    let header = Header::read(&mut decoder).map_err(|_| malformed())?;
    if header.metadata.message_type != MessageType::Query
        || header.counts.queries != 1
        || header.counts.answers != 0
        || header.counts.authorities != 0
        || header.counts.additionals > 1
    {
        return Err(malformed());
    }
    let question = Query::read(&mut decoder).map_err(|_| malformed())?;
    let mut request = Message::new(
        header.metadata.id,
        MessageType::Query,
        header.metadata.op_code,
    );
    request.metadata = header.metadata;
    request.add_query(question);
    if header.counts.additionals == 1 {
        // Inspect the original owner before Message's EDNS conversion discards it.
        let mut owner = decoder.clone(u16::try_from(decoder.index()).map_err(|_| malformed())?);
        Name::read(&mut owner).map_err(|_| malformed())?;
        let rdata_start = owner.index() + 10; // Fixed RR metadata after the library-decoded owner.
        let record = Record::read(&mut decoder).map_err(|_| malformed())?;
        if !record.name.is_root()
            || !matches!(record.data, RData::OPT(_) | RData::Update0(RecordType::OPT))
        {
            return Err(malformed());
        }
        let mut edns = Edns::from(&record);
        let options = edns.options().as_ref();
        let original_length = decoder
            .index()
            .checked_sub(rdata_start)
            .ok_or_else(malformed)?;
        // OPT::read_data clears unfinished options instead of failing. Require
        // every original RDATA byte to remain accounted for after library decode.
        let decoded_length = options
            .iter()
            .map(|(_, option)| 4 + usize::from(option.len()))
            .sum::<usize>();
        if options.len() > 16 || original_length > 4096 || original_length != decoded_length {
            return Err(malformed());
        }
        // Options are validated, then discarded. Even the internal processor
        // receives no ECS/client extension data to accidentally forward later.
        edns.options_mut().options.clear();
        request.set_edns(edns);
    }
    if !decoder.is_empty() {
        return Err(malformed());
    }
    Ok(request)
}

pub(super) fn response(request: &Message, code: ResponseCode) -> Message {
    let mut reply = Message::new(
        request.metadata.id,
        MessageType::Response,
        request.metadata.op_code,
    );
    reply.metadata.recursion_desired = request.metadata.recursion_desired;
    reply.metadata.response_code = code;
    // A protocol copy preserves the original wire question, including its type.
    reply.queries.clone_from(&request.queries);
    if request.edns.is_some() {
        let mut edns = Edns::new();
        edns.set_max_payload(MAX_UDP_BYTES as u16);
        reply.set_edns(edns);
    }
    reply
}

pub(super) async fn answer<P: Processor>(
    request: &Message,
    auth: &AuthorizedRequest<'_>,
    processor: &P,
) -> Result<Message> {
    if request
        .edns
        .as_ref()
        .is_some_and(|edns| edns.version() != 0)
    {
        return Ok(response(request, ResponseCode::BADVERS));
    }
    let question = request
        .queries
        .first()
        .ok_or(DnsError(ErrorCode::Malformed))?;
    let domain = question.name.to_ascii().parse::<CanonicalDomain>();
    if request.metadata.op_code != OpCode::Query
        || question.query_class != DNSClass::IN
        || matches!(
            question.query_type,
            RecordType::AXFR | RecordType::IXFR | RecordType::OPT
        )
        || domain.is_err()
    {
        return Ok(response(request, ResponseCode::NotImp));
    }
    let domain = domain.map_err(|_| DnsError(ErrorCode::Scope))?;
    let AdmissionScope::LegacyDns(binding) = auth.scope() else {
        return Err(DnsError(ErrorCode::Scope));
    };
    let selection = auth
        .decision()
        .compiled_configuration()
        .policy()
        .evaluate_dns(binding, &domain)
        .map_err(|_| DnsError(ErrorCode::Scope))?;
    let processed = processor.answer(request, selection, auth.decision()).await;
    let mut reply = match processed {
        Ok(reply) => reply,
        Err(_) => response(request, ResponseCode::ServFail),
    };
    // The internal processor supplies records/rcode only. It cannot change the
    // admitted question, assert validation, or echo client extension options.
    let clean = response(request, reply.metadata.response_code);
    reply.metadata = clean.metadata;
    reply.queries = clean.queries;
    reply.edns = clean.edns;
    reply.signature = None;
    Ok(reply)
}

pub(super) fn encode(
    request: &Message,
    reply: &Message,
    received: usize,
    tcp: bool,
) -> Result<Vec<u8>> {
    let ceiling = if tcp {
        MAX_TCP_BYTES
    } else {
        usize::from(
            request
                .edns
                .as_ref()
                .map_or(512, |edns| edns.max_payload().max(512)),
        )
        .min(MAX_UDP_BYTES)
        .min(received.saturating_mul(3))
    };
    encode_with_ceiling(request, reply, ceiling)
        .or_else(|_| encode_with_ceiling(request, &reply.truncate(), ceiling))
}

fn encode_with_ceiling(request: &Message, reply: &Message, ceiling: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(ceiling);
    let mut encoder = BinEncoder::new(&mut bytes);
    encoder.set_max_size(u16::try_from(ceiling).map_err(|_| DnsError(ErrorCode::State))?);
    reply
        .emit(&mut encoder)
        .map_err(|_| DnsError(ErrorCode::Unavailable))?;
    // Do not trust an encoder success if even the mandatory question did not fit.
    let mut decoder = BinDecoder::new(&bytes);
    let check = Message::read(&mut decoder).map_err(|_| DnsError(ErrorCode::State))?;
    if bytes.len() > ceiling
        || !decoder.is_empty()
        || check.queries != request.queries
        || check.metadata.id != request.metadata.id
        || check.metadata.message_type != MessageType::Response
    {
        return Err(DnsError(ErrorCode::Unavailable));
    }
    Ok(bytes)
}
