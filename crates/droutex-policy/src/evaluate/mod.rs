//! D38 pure selection over prepared data; no catalog compiler or admission proof.
mod ordering;
mod scope;

use crate::DomainMatcher;
use droutex_common::{
    AdministrativeState, DnsEndpointAssignment, EndpointId, FilterListRevisionId, NetworkGroupId,
    ProfileId, RuleId, RuleLayer, ServiceGroupRevisionId,
};
use std::{collections::BTreeMap, error::Error, fmt};

/// Aggregate profiles, endpoint/group associations and prepared rule entries.
pub const MAX_POLICY_ENTRIES: usize = 10_000;

/// Provenance only: a revision ID is not verification of its data or license.
/// Source kind fixes the layer; a fallback cannot be disguised as an explicit rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleSource {
    Custom(RuleId),
    Service {
        rule: RuleId,
        revision: ServiceGroupRevisionId,
    },
    Filter(FilterListRevisionId),
}

impl RuleSource {
    pub fn layer(self) -> RuleLayer {
        match self {
            Self::Custom(_) => RuleLayer::Custom,
            Self::Service { .. } => RuleLayer::Service,
            Self::Filter(_) => RuleLayer::Filter,
        }
    }
}

/// Explicit administrative state and within-layer priority, not authentication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleMetadata {
    pub source: RuleSource,
    pub state: AdministrativeState,
    pub priority: i32,
}

/// One validated domain condition with an opaque, comparable selection effect.
/// Actions stay in their owning DNS/traffic type; this crate never executes them.
pub struct DomainRule<A> {
    source: RuleSource,
    state: AdministrativeState,
    matcher: DomainMatcher,
    action: A,
    rank: ordering::RuleRank,
}

impl<A> DomainRule<A> {
    pub fn new(metadata: RuleMetadata, matcher: DomainMatcher, action: A) -> Self {
        Self {
            rank: ordering::RuleRank::new(metadata, &matcher),
            source: metadata.source,
            state: metadata.state,
            matcher,
            action,
        }
    }
}

/// Inclusive nonzero TCP ports. No missing-port or protocol inference is performed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcpPortRange {
    first: u16,
    last: u16,
}

impl TcpPortRange {
    pub fn new(first: u16, last: u16) -> Result<Self, PolicyBuildError> {
        if first == 0 || first > last {
            return Err(PolicyBuildError::InvalidPortRange);
        }
        Ok(Self { first, last })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TcpPortScope {
    Any,
    Range(TcpPortRange),
}

/// One TCP eligibility interval per prepared entry. An OR of intervals uses
/// entries with the same source/effect, still subject to the aggregate entry cap.
/// Interval width never adds a hidden priority or expands individual ports.
pub struct TcpRule<A> {
    rule: DomainRule<A>,
    ports: TcpPortScope,
}

impl<A> TcpRule<A> {
    pub fn new(rule: DomainRule<A>, ports: TcpPortScope) -> Self {
        Self { rule, ports }
    }
}

/// Prepared inputs, not a full P2-05 compiled snapshot. Catalog imports retain
/// P2-07 verification/provenance gates. Disabled entries still count toward limits.
pub struct PreparedProfile<D, T> {
    pub id: ProfileId,
    pub dns_default: D,
    pub dns_rules: Vec<DomainRule<D>>,
    pub tcp_default: T,
    pub tcp_rules: Vec<TcpRule<T>>,
}

/// Independent gateway group->Profile association, never learned from DNS.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GatewayProfileAssignment {
    pub group: NetworkGroupId,
    pub profile: ProfileId,
}

/// Immutable, bounded policy associations. Inputs and public IDs alone cannot
/// authenticate a client or authorize a gateway destination; P3/P4 own admission.
pub struct PolicySet<D, T> {
    profiles: BTreeMap<ProfileId, PreparedProfile<D, T>>,
    endpoints: BTreeMap<EndpointId, DnsEndpointAssignment>,
    groups: BTreeMap<NetworkGroupId, ProfileId>,
}

impl<D, T> fmt::Debug for PolicySet<D, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PolicySet")
            .field("profiles", &self.profiles.len())
            .field("endpoints", &self.endpoints.len())
            .field("groups", &self.groups.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyScope {
    DnsEndpoint(EndpointId),
    GatewayGroup(NetworkGroupId),
}

/// A borrowed selection, not a live effect, capability check or access permit.
/// None source means the Profile fallback was selected after all explicit layers.
///
/// A selection cannot outlive its policy owner:
/// ```compile_fail
/// use droutex_policy::{CanonicalDomain, PolicySet, PolicySelection};
/// use droutex_common::DnsProfileBinding;
/// fn escape<'a>(policy: PolicySet<u8, u8>, binding: DnsProfileBinding,
///               query: &CanonicalDomain) -> PolicySelection<'a, u8> {
///     policy.evaluate_dns(&binding, query).unwrap()
/// }
/// ```
#[derive(Eq, PartialEq)]
pub struct PolicySelection<'a, A> {
    scope: PolicyScope,
    profile: ProfileId,
    source: Option<RuleSource>,
    action: &'a A,
}

impl<A> PolicySelection<'_, A> {
    pub fn action(&self) -> &A {
        self.action
    }
    pub fn source(&self) -> Option<RuleSource> {
        self.source
    }
    pub fn profile(&self) -> ProfileId {
        self.profile
    }
    pub fn scope(&self) -> PolicyScope {
        self.scope
    }
}

impl<A> fmt::Debug for PolicySelection<'_, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // An opaque effect may hold confidential caller data. Never format A.
        f.debug_struct("PolicySelection")
            .field("scope", &self.scope)
            .field("profile", &self.profile)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyBuildError {
    EntryLimit,
    DuplicateProfile(ProfileId),
    DuplicateEndpoint(EndpointId),
    DuplicateGroup(NetworkGroupId),
    UnknownProfile(ProfileId),
    InvalidPortRange,
}

impl fmt::Display for PolicyBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EntryLimit => "policy entry limit exceeded",
            Self::DuplicateProfile(_) => "duplicate policy Profile",
            Self::DuplicateEndpoint(_) => "duplicate DNS Endpoint association",
            Self::DuplicateGroup(_) => "duplicate gateway group association",
            Self::UnknownProfile(_) => "unknown policy Profile reference",
            Self::InvalidPortRange => "invalid TCP port range",
        })
    }
}
impl Error for PolicyBuildError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationError {
    UnknownEndpoint,
    UnavailableEndpoint,
    EndpointBindingMismatch,
    UnknownGatewayGroup,
    UnknownProfile,
    Ambiguous {
        first: RuleSource,
        second: RuleSource,
    },
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownEndpoint => "unknown DNS policy scope",
            Self::UnavailableEndpoint => "DNS policy scope unavailable",
            Self::EndpointBindingMismatch => "DNS policy association mismatch",
            Self::UnknownGatewayGroup => "unknown gateway policy scope",
            Self::UnknownProfile => "policy Profile unavailable",
            Self::Ambiguous { .. } => "ambiguous highest-priority policy effects",
        })
    }
}
impl Error for EvaluationError {}
