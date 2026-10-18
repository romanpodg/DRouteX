//! Shared resource identities and policy vocabulary, without network-admission authority.

mod domain;

pub use domain::{
    AdministrativeState, AmbiguousLegacyAssignment, ConfigurationRevisionId, CorrelationId,
    DeploymentId, DeviceId, DnsBindingError, DnsEndpointAssignment, DnsProfileBinding,
    DnsUpstreamId, EmptyResolverCapability, EndpointId, FilterListId, FilterListRevisionId,
    GatewayNetworkGrantId, GatewayNetworkIdentity, InvalidResourceId, LegacyDnsAssignment,
    LegacyNetworkGrantId, ListenerId, NetworkGroupId, NodeScopeId, OutboundAvailability,
    PrivateDnsRecordId, ProfileId, ProxyOutboundId, ProxyOutboundKind, ResolverCapabilityMaterial,
    ResolverCredentialId, ResolverCredentialMetadata, ResolverCredentialState, RevisionState,
    RuleId, SecretRefId, ServiceGroupId, ServiceGroupRevisionId, SnapshotId, SnapshotState, UserId,
    unique_legacy_assignment,
};

/// A policy layer; user priority applies within a layer, never across layers.
///
/// Declaration order is not precedence. Use `droutex_policy::RULE_LAYER_ORDER`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleLayer {
    /// Operator-authored rules.
    Custom,
    /// Rules for an explicitly selected service revision.
    Service,
    /// Rules from selected filter-list revisions.
    Filter,
    /// The fallback when no higher layer selects an action.
    Default,
}
