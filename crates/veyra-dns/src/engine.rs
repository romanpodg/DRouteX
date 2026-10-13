//! One DNS query in, at most one reply out (spec §9).

use crate::synth::{self, MAX_UDP_PAYLOAD};
use crate::upstream;
use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{DNSClass, RecordType};
use std::net::IpAddr;
use std::time::Duration;
use veyra_policy::{DnsConfig, DnsPlan, Mode, Name, Policy, QueryKind, Subject};

/// How the query arrived: UDP replies must fit the client's buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Udp,
    Tcp,
}

pub struct Engine {
    policy: Policy,
    config: DnsConfig,
    upstream_timeout: Duration,
}

impl Engine {
    /// Fails when the policy has no `dns:` section.
    pub fn new(policy: Policy) -> Result<Engine, String> {
        let config = policy
            .dns()
            .cloned()
            .ok_or("the config has no dns section")?;
        Ok(Engine {
            policy,
            config,
            upstream_timeout: Duration::from_secs(2),
        })
    }

    /// How long to wait for each upstream server before trying the next.
    pub fn with_upstream_timeout(mut self, wait: Duration) -> Engine {
        self.upstream_timeout = wait;
        self
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Answer one DNS message from `source`. `None` means send nothing: the
    /// source is not admitted (spec §9.6), or the bytes are not a DNS query.
    pub async fn handle(
        &self,
        source: IpAddr,
        bytes: &[u8],
        transport: Transport,
    ) -> Option<Vec<u8>> {
        let admitted = self.policy.admit(source)?;
        let request = Message::from_vec(bytes).ok()?;
        if request.metadata.message_type != MessageType::Query {
            return None;
        }
        let reply = self.respond(&admitted.subject, &request).await;
        encode(&request, reply, transport)
    }

    async fn respond(&self, subject: &Subject, request: &Message) -> Message {
        if request.metadata.op_code != OpCode::Query {
            return synth::reply(request, ResponseCode::NotImp);
        }
        let [query] = request.queries.as_slice() else {
            return synth::reply(request, ResponseCode::FormErr);
        };
        let asked = query.name().to_ascii();
        let decision = match Name::parse(&asked) {
            Ok(name) => self
                .policy
                .decide_in(subject, &name, Some(Mode::SmartdnsTcp)),
            Err(_) => self.policy.default_decision(subject, &asked),
        };
        match self.policy.dns_plan(&decision, kind(query)) {
            DnsPlan::Addresses(ips) => synth::addresses(request, &ips, self.config.synth_ttl),
            DnsPlan::NoData => synth::no_data(request, self.config.synth_ttl),
            DnsPlan::Forward { group } => self.forward(request, &group).await,
        }
    }

    async fn forward(&self, request: &Message, group: &str) -> Message {
        let servers = self
            .config
            .upstreams
            .get(group)
            .map_or(&[][..], Vec::as_slice);
        match upstream::exchange(servers, request, self.upstream_timeout).await {
            Some(reply) => reply,
            None => synth::reply(request, ResponseCode::ServFail),
        }
    }
}

fn kind(query: &Query) -> QueryKind {
    if query.query_class() != DNSClass::IN {
        return QueryKind::Other;
    }
    match query.query_type() {
        RecordType::A => QueryKind::A,
        RecordType::AAAA => QueryKind::Aaaa,
        RecordType::HTTPS => QueryKind::Https,
        RecordType::SVCB => QueryKind::Svcb,
        _ => QueryKind::Other,
    }
}

/// Over UDP, a reply larger than the client can take becomes a truncated
/// reply (TC set, no records) and the client retries over TCP.
fn encode(request: &Message, reply: Message, transport: Transport) -> Option<Vec<u8>> {
    let bytes = reply.to_vec().ok()?;
    if transport == Transport::Udp && bytes.len() > udp_limit(request) {
        return reply.truncate().to_vec().ok();
    }
    Some(bytes)
}

/// 512 bytes without EDNS, else the client's advertised size, at most 1232.
fn udp_limit(request: &Message) -> usize {
    request.edns.as_ref().map_or(512, |edns| {
        usize::from(edns.max_payload().clamp(512, MAX_UDP_PAYLOAD))
    })
}
