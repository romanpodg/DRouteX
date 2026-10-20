//! D40 adapters for existing wire actions; no alternate action vocabulary,
//! serialized field, connector or same-generation publication proof is added.
use crate::{
    CheckedConfiguration, ConfigError, DnsAction, OutboundKind, ReferenceCatalog, TrafficAction,
};
use droutex_common::{
    AdministrativeState, EndpointId, NetworkGroupId, OutboundAvailability, ProfileId,
    ProxyOutboundId, ProxyOutboundKind,
};
use droutex_policy::{
    ActionInventoryError, ActionRefusal, ActionResolver, DnsIntent, GatewayReadiness,
    MAX_POLICY_ENTRIES, OutboundFacts, PolicyScope, PolicySelection, RequestTransport, RuleSource,
    TcpCapability, TcpIntent,
};
use std::{borrow::Borrow, collections::BTreeMap, error::Error, fmt};

/// Caller-trusted observation only. Administrative state always comes from config.
/// Its kind must match the current declaration; P4 owns freshness and live checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutboundObservation {
    pub id: ProxyOutboundId,
    pub kind: ProxyOutboundKind,
    pub availability: OutboundAvailability,
    pub tcp: TcpCapability,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ActionPreparationError {
    Configuration(ConfigError),
    Inventory(ActionInventoryError),
    DuplicateObservation(ProxyOutboundId),
    UnknownObservation(ProxyOutboundId),
    KindMismatch(ProxyOutboundId),
}
impl fmt::Display for ActionPreparationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid action preparation inputs")
    }
}
impl Error for ActionPreparationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Configuration(e) => Some(e),
            Self::Inventory(e) => Some(e),
            _ => None,
        }
    }
}

impl CheckedConfiguration {
    /// Revalidate CURRENT references; observations cannot enable a disabled
    /// declaration or substitute another kind. Missing evidence remains unknown.
    /// P2-05 must separately bind COMPLETE validated policy/resources to this same
    /// generation. Known public IDs are neither resolver access nor proxy authority.
    pub fn prepare_action_resolver(
        &self,
        current: &ReferenceCatalog,
        mut observations: Vec<OutboundObservation>,
    ) -> Result<ActionResolver, ActionPreparationError> {
        if observations.len() > MAX_POLICY_ENTRIES {
            return Err(ActionPreparationError::Inventory(
                ActionInventoryError::EntryLimit,
            ));
        }
        MAX_POLICY_ENTRIES
            .checked_sub(current.upstreams.len())
            .and_then(|n| n.checked_sub(self.document().outbounds.len()))
            .ok_or(ActionPreparationError::Inventory(
                ActionInventoryError::EntryLimit,
            ))?;
        self.revalidate(current)
            .map_err(ActionPreparationError::Configuration)?;
        observations.sort_unstable_by_key(|o| o.id);
        if let Some(pair) = observations
            .windows(2)
            .find(|pair| pair[0].id == pair[1].id)
        {
            return Err(ActionPreparationError::DuplicateObservation(pair[0].id));
        }
        let known: BTreeMap<_, _> = self
            .document()
            .outbounds
            .iter()
            .map(|o| (o.id, o))
            .collect();
        for observation in &observations {
            let outbound = known
                .get(&observation.id)
                .ok_or(ActionPreparationError::UnknownObservation(observation.id))?;
            if observation.kind != native_kind(&outbound.transport) {
                return Err(ActionPreparationError::KindMismatch(observation.id));
            }
        }
        let observed: BTreeMap<_, _> = observations.into_iter().map(|o| (o.id, o)).collect();
        let facts = self
            .document()
            .outbounds
            .iter()
            .map(|outbound| {
                let observation = observed.get(&outbound.id);
                OutboundFacts {
                    id: outbound.id,
                    state: if outbound.enabled {
                        AdministrativeState::Enabled
                    } else {
                        AdministrativeState::Disabled
                    },
                    kind: native_kind(&outbound.transport),
                    availability: observation
                        .map_or(OutboundAvailability::Unknown, |o| o.availability),
                    tcp: observation.map_or(TcpCapability::Unknown, |o| o.tcp),
                }
            })
            .collect();
        ActionResolver::new(current.upstreams.iter().copied().collect(), facts)
            .map_err(ActionPreparationError::Inventory)
    }
}

fn native_kind(kind: &OutboundKind) -> ProxyOutboundKind {
    match kind {
        OutboundKind::VpsDirect {} => ProxyOutboundKind::GatewayDirect,
        OutboundKind::HttpConnectTls { .. } => ProxyOutboundKind::HttpConnectTls,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidActionScope;
impl fmt::Display for InvalidActionScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("action selection has the wrong decision scope")
    }
}
impl Error for InvalidActionScope {}

/// Context from a DNS selection, not a gateway identity or effective route.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DnsActionResolution {
    endpoint: EndpointId,
    profile: ProfileId,
    source: Option<RuleSource>,
    intent: DnsIntent,
}
impl DnsActionResolution {
    pub fn endpoint(self) -> EndpointId {
        self.endpoint
    }
    pub fn profile(self) -> ProfileId {
        self.profile
    }
    pub fn source(self) -> Option<RuleSource> {
        self.source
    }
    pub fn intent(self) -> DnsIntent {
        self.intent
    }
}

/// Independent group provenance; this is not destination authorization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcpActionResolution {
    group: NetworkGroupId,
    profile: ProfileId,
    source: Option<RuleSource>,
    intent: TcpIntent,
}
impl TcpActionResolution {
    pub fn group(self) -> NetworkGroupId {
        self.group
    }
    pub fn profile(self) -> ProfileId {
        self.profile
    }
    pub fn source(self) -> Option<RuleSource> {
        self.source
    }
    pub fn intent(self) -> TcpIntent {
        self.intent
    }
    pub fn on_connection_failure(mut self) -> Self {
        self.intent = self.intent.on_connection_failure();
        self
    }
}

/// Preserve DNS Profile/source without creating a gateway/network identity.
pub fn resolve_dns_action<A: Borrow<DnsAction>>(
    resolver: &ActionResolver,
    selection: &PolicySelection<'_, A>,
    readiness: GatewayReadiness,
) -> Result<DnsActionResolution, InvalidActionScope> {
    let PolicyScope::DnsEndpoint(endpoint) = selection.scope() else {
        return Err(InvalidActionScope);
    };
    let intent = match selection.action().borrow() {
        DnsAction::Resolve { upstream } => resolver.dns_origin(*upstream),
        DnsAction::Block {} => DnsIntent::Block,
        DnsAction::Gateway { outbound } => resolver.dns_gateway(*outbound, readiness),
    };
    Ok(DnsActionResolution {
        endpoint,
        profile: selection.profile(),
        source: selection.source(),
        intent,
    })
}

/// Only a separately selected gateway group can produce a TCP intent. A caller
/// must treat a scope error as refusal; no automatic fail-open/default exists.
pub fn resolve_tcp_action<A: Borrow<TrafficAction>>(
    resolver: &ActionResolver,
    selection: &PolicySelection<'_, A>,
    transport: RequestTransport,
) -> Result<TcpActionResolution, InvalidActionScope> {
    let PolicyScope::GatewayGroup(group) = selection.scope() else {
        return Err(InvalidActionScope);
    };
    let intent = match selection.action().borrow() {
        TrafficAction::Deny {} => TcpIntent::Deny(ActionRefusal::PolicyDenied),
        TrafficAction::Outbound { outbound } => resolver.tcp(*outbound, transport),
    };
    Ok(TcpActionResolution {
        group,
        profile: selection.profile(),
        source: selection.source(),
        intent,
    })
}
