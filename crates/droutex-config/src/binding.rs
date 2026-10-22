use crate::{
    CompiledConfiguration, OutboundObservation, TcpActionResolution, TrafficAction,
    snapshot::{SnapshotContent, TrustedResources},
};
use droutex_common::{GatewayNetworkIdentity, ProxyOutboundId};
use droutex_policy::{CanonicalDomain, RequestTransport, TcpIntent};
use std::{error::Error, fmt, num::NonZeroU16};

/// Static scope and policy intent only. Core separately checks current lease/time.
pub struct GatewayCandidate {
    identity: GatewayNetworkIdentity,
    outbound: ProxyOutboundId,
    action: TcpActionResolution,
    domain: CanonicalDomain,
    port: NonZeroU16,
}
impl fmt::Debug for GatewayCandidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GatewayCandidate").finish_non_exhaustive()
    }
}
impl GatewayCandidate {
    pub fn identity(&self) -> GatewayNetworkIdentity {
        self.identity
    }
    pub fn outbound(&self) -> ProxyOutboundId {
        self.outbound
    }
    pub fn action(&self) -> TcpActionResolution {
        self.action
    }
    pub fn domain(&self) -> &CanonicalDomain {
        &self.domain
    }
    pub fn port(&self) -> NonZeroU16 {
        self.port
    }
    pub fn matches_destination(&self, domain: &str, port: NonZeroU16) -> bool {
        port == self.port
            && domain
                .parse::<CanonicalDomain>()
                .is_ok_and(|d| d == self.domain)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGatewayBinding;
impl fmt::Display for InvalidGatewayBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("gateway scope or action unavailable")
    }
}
impl Error for InvalidGatewayBinding {}

impl SnapshotContent {
    /// Pure binding candidate, not source authentication or permission to connect.
    /// Core must pair its own content/tables and current runtime authorization.
    pub fn gateway_candidate(
        &self,
        compiled: &CompiledConfiguration,
        identity: GatewayNetworkIdentity,
        domain: &str,
        port: NonZeroU16,
        observations: Vec<OutboundObservation>,
    ) -> Result<GatewayCandidate, InvalidGatewayBinding> {
        let query = domain.parse().map_err(|_| InvalidGatewayBinding)?;
        let scope = compiled
            .gateway_scope(identity.grant)
            .ok_or(InvalidGatewayBinding)?;
        if !scope.matches_request_scope(&identity, &query, port) {
            return Err(InvalidGatewayBinding);
        }
        let selection = compiled
            .policy()
            .evaluate_tcp(&identity, &query, port)
            .map_err(|_| InvalidGatewayBinding)?;
        let TrafficAction::Outbound { outbound } = *selection.action() else {
            return Err(InvalidGatewayBinding);
        };
        if !scope.matches_static_scope(&identity, &query, port, outbound) {
            return Err(InvalidGatewayBinding);
        }
        let refs = TrustedResources::new(self.resource_pins().to_vec())
            .map_err(|_| InvalidGatewayBinding)?
            .catalog();
        let resolver = self
            .configuration()
            .prepare_action_resolver(&refs, observations)
            .map_err(|_| InvalidGatewayBinding)?;
        let action = crate::resolve_tcp_action(&resolver, &selection, RequestTransport::Tcp)
            .map_err(|_| InvalidGatewayBinding)?;
        if matches!(action.intent(), TcpIntent::Deny(_)) {
            return Err(InvalidGatewayBinding);
        }
        Ok(GatewayCandidate {
            identity,
            outbound,
            action,
            domain: query,
            port,
        })
    }
}
