//! What DNS answers for a decision: the table of spec §9.5. Pure, so the DNS
//! engine and `veyra explain` share it without either owning the wire format.

use crate::compile::Policy;
use crate::decide::Decision;
use crate::dns_config::DEFAULT_UPSTREAM_GROUP;
use crate::model::{Action, GatewaySpec};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// The query types §9.5 treats differently. Everything else, including any
/// class other than IN, is `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryKind {
    A,
    Aaaa,
    Https,
    Svcb,
    Other,
}

/// How to answer one query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsPlan {
    /// Ask this upstream group and pass its answer through unchanged.
    Forward { group: String },
    /// Answer with these addresses (A or AAAA by family), TTL `synthTtl`.
    Addresses(Vec<IpAddr>),
    /// NOERROR without records, with a synthesized SOA (spec §9.5 NODATA).
    NoData,
}

impl Policy {
    /// The §9.5 answer for `decision`, which must come from
    /// `decide_in(.., Some(Mode::SmartdnsTcp))`: only rules that allow
    /// `smartdns_tcp` may send DNS clients to a gateway.
    pub fn dns_plan(&self, decision: &Decision, kind: QueryKind) -> DnsPlan {
        use QueryKind::*;
        let forward = || DnsPlan::Forward {
            group: decision
                .upstream_group
                .clone()
                .unwrap_or_else(|| DEFAULT_UPSTREAM_GROUP.into()),
        };
        match &decision.route {
            Action::Direct {} => forward(),
            // Null addresses fail fast; NXDOMAIN would be cached and invite
            // clients to try a fallback resolver (spec §9.5).
            Action::Block {} => match kind {
                A => DnsPlan::Addresses(vec![Ipv4Addr::UNSPECIFIED.into()]),
                Aaaa => DnsPlan::Addresses(vec![Ipv6Addr::UNSPECIFIED.into()]),
                _ => DnsPlan::NoData,
            },
            Action::ProxyNode { node, .. } => match (kind, self.gateway(node)) {
                (A, Some(gateway)) => DnsPlan::Addresses(vec![gateway.ipv4.into()]),
                (Aaaa, Some(GatewaySpec { ipv6: Some(v6), .. })) => {
                    DnsPlan::Addresses(vec![(*v6).into()])
                }
                // Other types do not steer the connection, so they resolve normally.
                (Other, _) => forward(),
                // HTTPS/SVCB carry origin address hints and ECH (spec §9.5), an AAAA
                // without a gateway IPv6 would leak the origin: answer nothing.
                _ => DnsPlan::NoData,
            },
        }
    }
}
