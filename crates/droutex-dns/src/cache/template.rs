use crate::wire;
use droutex_config::upstream::Transport;
use hickory_proto::{
    op::{Header, Message, ResponseCode},
    rr::{DNSClass, RData},
    serialize::binary::{BinDecodable, BinDecoder, BinEncodable, BinEncoder},
};
use tokio::time::{Duration, Instant};

// Successful candidate metadata is supplied exclusively by the composed forwarder.
// The first transport receive is a conservative floor for the chosen candidate:
// foreign frames, library scheduling, validation/checkpoint waits and cleanup
// can shorten cache life, never extend it.
pub(crate) struct UpstreamAnswer {
    pub message: Message,
    pub received: Instant,
    pub member: usize,
    pub transport: Transport,
}
pub(crate) struct Template {
    bytes: [u8; 4096],
    len: usize,
    received: Instant,
    expires: Instant,
    member: usize,
    transport: Transport,
}
impl Template {
    pub(crate) fn build(answer: &UpstreamAnswer) -> Option<Self> {
        let message = &answer.message;
        if message.metadata.truncation || message.queries.len() != 1 {
            return None;
        }
        let mut ttl = match message.metadata.response_code {
            ResponseCode::NoError if !message.answers.is_empty() => {
                if message
                    .authorities
                    .iter()
                    .any(|rr| matches!(rr.data, RData::SOA(_)))
                {
                    return None;
                }
                3600
            }
            ResponseCode::NXDomain | ResponseCode::NoError if message.answers.is_empty() => {
                let mut soa = message.authorities.iter().filter_map(|rr| {
                    if let RData::SOA(data) = &rr.data {
                        Some((rr, data))
                    } else {
                        None
                    }
                });
                let (record, data) = soa.next()?;
                if soa.next().is_some()
                    || record.dns_class != DNSClass::IN
                    || !record.name.zone_of(&message.queries[0].name)
                    || message
                        .authorities
                        .iter()
                        .any(|rr| matches!(rr.data, RData::NS(_)))
                {
                    return None;
                }
                60.min(data.minimum).min(record.ttl)
            }
            _ => return None,
        };
        for rr in message
            .answers
            .iter()
            .chain(&message.authorities)
            .chain(&message.additionals)
        {
            if matches!(rr.data, RData::OPT(_) | RData::Update0(_)) || rr.dns_class != DNSClass::IN
            {
                return None;
            }
            ttl = ttl.min(if rr.ttl & (1 << 31) == 0 { rr.ttl } else { 0 });
        }
        if ttl == 0 {
            return None;
        }
        let expires = answer
            .received
            .checked_add(Duration::from_secs(u64::from(ttl)))?;
        let mut encoded = Vec::with_capacity(4096);
        let mut encoder = BinEncoder::new(&mut encoded);
        encoder.set_max_size(4096);
        message.emit(&mut encoder).ok()?;
        let header = Header::read(&mut BinDecoder::new(&encoded)).ok()?;
        if encoded.len() > 4096
            || header.metadata.truncation
            || usize::from(header.counts.answers) != message.answers.len()
            || usize::from(header.counts.authorities) != message.authorities.len()
            || usize::from(header.counts.additionals)
                != message.additionals.len() + usize::from(message.edns.is_some())
        {
            return None;
        }
        let mut bytes = [0; 4096];
        bytes[..encoded.len()].copy_from_slice(&encoded);
        Some(Self {
            bytes,
            len: encoded.len(),
            received: answer.received,
            expires,
            member: answer.member,
            transport: answer.transport,
        })
    }
    pub fn fresh(&self, now: Instant) -> bool {
        now >= self.received && now < self.expires
    }
    pub fn copy(&self) -> Self {
        Self {
            bytes: self.bytes,
            len: self.len,
            received: self.received,
            expires: self.expires,
            member: self.member,
            transport: self.transport,
        }
    }
    pub fn reply(&self, request: &Message, now: Instant) -> Option<wire::Reply> {
        if !self.fresh(now) {
            return None;
        }
        let mut message = Message::from_vec(&self.bytes[..self.len]).ok()?;
        if message.queries != request.queries {
            return None;
        }
        let ttl = u32::try_from(self.expires.checked_duration_since(now)?.as_secs()).ok()?;
        for rr in message
            .answers
            .iter_mut()
            .chain(&mut message.authorities)
            .chain(&mut message.additionals)
        {
            rr.ttl = rr.ttl.min(ttl);
        }
        Some(wire::Reply {
            message: wire::sanitize(request, message, true),
            expires: Some(self.expires),
        })
    }
}
