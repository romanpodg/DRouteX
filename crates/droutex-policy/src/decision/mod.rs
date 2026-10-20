//! D40 pure intents. Facts do not authenticate themselves; P2-05 binds the
//! generation, P3/P4 own live authorization, freshness, TLS and network checks.
mod local;
use crate::MAX_POLICY_ENTRIES;
use droutex_common::{
    AdministrativeState, DnsUpstreamId, OutboundAvailability, ProxyOutboundId, ProxyOutboundKind,
};
pub use local::{LocalAnswerIntent, local_answer_intent};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TcpCapability {
    Supported,
    Unsupported,
    Unknown,
}

/// An explicit observation, never proof of current transport/destination access.
/// No engine, tunnel or node mode is representable as a native MVP outbound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutboundFacts {
    pub id: ProxyOutboundId,
    pub state: AdministrativeState,
    pub kind: ProxyOutboundKind,
    pub availability: OutboundAvailability,
    pub tcp: TcpCapability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestTransport {
    Tcp,
    Udp,
    Unknown,
}

/// Caller-assessed compatibility only; P3-06/P4 still require actual separate
/// gateway identity, destination binding and checked/pinned addresses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GatewayReadiness {
    CompatibleTcp,
    Unavailable,
    Unknown,
}

/// Predicted origin of a connection decision, not observed egress/connectivity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionPoint {
    ClientOrigin,
    GatewayVps,
    ProtectedProxyExit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionRefusal {
    PolicyDenied,
    UnknownUpstream(DnsUpstreamId),
    UnknownOutbound(ProxyOutboundId),
    DisabledOutbound(ProxyOutboundId),
    UnavailableOutbound(ProxyOutboundId),
    UnknownAvailability(ProxyOutboundId),
    UnsupportedCapability(ProxyOutboundId),
    UnknownCapability(ProxyOutboundId),
    UnsupportedTransport,
    GatewayNotReady,
    ConnectionFailed(ProxyOutboundId),
    PrivateProfileMismatch,
    PrivateNameMismatch,
}
impl fmt::Display for ActionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("action refused or unavailable")
    }
}
impl Error for ActionRefusal {}

/// DNS intents never switch gateway policy or grant proxy access. Gateway means
/// a requested ingress whose traffic decision is independently made at the VPS.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DnsIntent {
    Resolve(DnsUpstreamId),
    Block,
    Gateway(ProxyOutboundId),
    Refuse(ActionRefusal),
}
impl DnsIntent {
    pub fn decision_point(self) -> Option<DecisionPoint> {
        match self {
            Self::Resolve(_) => Some(DecisionPoint::ClientOrigin),
            Self::Gateway(_) => Some(DecisionPoint::GatewayVps),
            Self::Block | Self::Refuse(_) => None,
        }
    }
}

/// Neither successful variant is a connector/admission permit. P4 must perform
/// current destination/grant/address/TLS checks even for an explicit direct route.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TcpIntent {
    Deny(ActionRefusal),
    GatewayDirect(ProxyOutboundId),
    ProtectedProxy(ProxyOutboundId),
}
impl TcpIntent {
    pub fn decision_point(self) -> Option<DecisionPoint> {
        match self {
            Self::GatewayDirect(_) => Some(DecisionPoint::GatewayVps),
            Self::ProtectedProxy(_) => Some(DecisionPoint::ProtectedProxyExit),
            Self::Deny(_) => None,
        }
    }

    /// A real failure may only deny, never infer a fallback/direct route.
    pub fn on_connection_failure(self) -> Self {
        match self {
            Self::GatewayDirect(id) | Self::ProtectedProxy(id) => {
                Self::Deny(ActionRefusal::ConnectionFailed(id))
            }
            Self::Deny(_) => self,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionInventoryError {
    EntryLimit,
    DuplicateUpstream(DnsUpstreamId),
    DuplicateOutbound(ProxyOutboundId),
}
impl fmt::Display for ActionInventoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid action inventory")
    }
}
impl Error for ActionInventoryError {}

/// Bounded immutable lookup of known references and explicit caller-trusted
/// observations. A known upstream ID proves neither resolver access nor health.
pub struct ActionResolver {
    upstreams: BTreeSet<DnsUpstreamId>,
    outbounds: BTreeMap<ProxyOutboundId, OutboundFacts>,
}
impl fmt::Debug for ActionResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActionResolver")
            .field("upstreams", &self.upstreams.len())
            .field("outbounds", &self.outbounds.len())
            .finish_non_exhaustive()
    }
}
impl ActionResolver {
    pub fn new(
        mut upstreams: Vec<DnsUpstreamId>,
        mut outbounds: Vec<OutboundFacts>,
    ) -> Result<Self, ActionInventoryError> {
        MAX_POLICY_ENTRIES
            .checked_sub(upstreams.len())
            .and_then(|n| n.checked_sub(outbounds.len()))
            .ok_or(ActionInventoryError::EntryLimit)?;
        upstreams.sort_unstable();
        outbounds.sort_unstable_by_key(|f| f.id);
        let mut known = BTreeSet::new();
        for id in upstreams {
            if !known.insert(id) {
                return Err(ActionInventoryError::DuplicateUpstream(id));
            }
        }
        let mut observed = BTreeMap::new();
        for fact in outbounds {
            if observed.insert(fact.id, fact).is_some() {
                return Err(ActionInventoryError::DuplicateOutbound(fact.id));
            }
        }
        Ok(Self {
            upstreams: known,
            outbounds: observed,
        })
    }

    pub fn dns_origin(&self, upstream: DnsUpstreamId) -> DnsIntent {
        if self.upstreams.contains(&upstream) {
            DnsIntent::Resolve(upstream)
        } else {
            DnsIntent::Refuse(ActionRefusal::UnknownUpstream(upstream))
        }
    }

    pub fn dns_gateway(&self, outbound: ProxyOutboundId, readiness: GatewayReadiness) -> DnsIntent {
        if readiness != GatewayReadiness::CompatibleTcp {
            return DnsIntent::Refuse(ActionRefusal::GatewayNotReady);
        }
        match self.usable_outbound(outbound) {
            Ok(_) => DnsIntent::Gateway(outbound),
            Err(reason) => DnsIntent::Refuse(reason),
        }
    }

    pub fn tcp(&self, outbound: ProxyOutboundId, transport: RequestTransport) -> TcpIntent {
        if transport != RequestTransport::Tcp {
            return TcpIntent::Deny(ActionRefusal::UnsupportedTransport);
        }
        match self.usable_outbound(outbound) {
            Ok(ProxyOutboundKind::GatewayDirect) => TcpIntent::GatewayDirect(outbound),
            Ok(ProxyOutboundKind::HttpConnectTls) => TcpIntent::ProtectedProxy(outbound),
            Err(reason) => TcpIntent::Deny(reason),
        }
    }

    fn usable_outbound(&self, id: ProxyOutboundId) -> Result<ProxyOutboundKind, ActionRefusal> {
        let fact = self
            .outbounds
            .get(&id)
            .ok_or(ActionRefusal::UnknownOutbound(id))?;
        if fact.state != AdministrativeState::Enabled {
            return Err(ActionRefusal::DisabledOutbound(id));
        }
        match fact.availability {
            OutboundAvailability::Unknown => return Err(ActionRefusal::UnknownAvailability(id)),
            OutboundAvailability::Unavailable => {
                return Err(ActionRefusal::UnavailableOutbound(id));
            }
            OutboundAvailability::Available => {}
        }
        match fact.tcp {
            TcpCapability::Unknown => Err(ActionRefusal::UnknownCapability(id)),
            TcpCapability::Unsupported => Err(ActionRefusal::UnsupportedCapability(id)),
            TcpCapability::Supported => Ok(fact.kind),
        }
    }
}
