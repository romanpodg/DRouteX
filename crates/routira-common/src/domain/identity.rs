use super::{
    AdministrativeState, DeviceId, EndpointId, GatewayNetworkGrantId, LegacyNetworkGrantId,
    ListenerId, NetworkGroupId, ProfileId, ResolverCredentialId, ResolverCredentialState,
};
use std::{error::Error, fmt, net::IpAddr};

/// Public metadata only. Raw resolver secrets never belong in this record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolverCredentialMetadata {
    /// Public identity of the verifier/provisioning record.
    pub id: ResolverCredentialId,
    /// The only Endpoint this credential may select.
    pub endpoint: EndpointId,
    /// Lifecycle observation, not independent authentication proof.
    pub state: ResolverCredentialState,
}

/// One active Profile assignment per Endpoint, independent of the client's NAT IP.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DnsEndpointAssignment {
    /// Public Endpoint identity.
    pub endpoint: EndpointId,
    /// The single assigned Profile.
    pub profile: ProfileId,
    /// Configuration intent, not actual listener status.
    pub state: AdministrativeState,
    /// Optional label/metadata link; it never authenticates a TCP connection.
    pub device: Option<DeviceId>,
}

/// DNS policy association only; not a gateway grant or runtime admission permit.
///
/// ```compile_fail
/// use routira_common::{DnsProfileBinding, GatewayNetworkIdentity};
/// fn gateway(_: GatewayNetworkIdentity) {}
/// fn incorrect_upgrade(dns: DnsProfileBinding) { gateway(dns); }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DnsProfileBinding {
    /// Selected public Endpoint identity.
    pub endpoint: EndpointId,
    /// Its assigned Profile.
    pub profile: ProfileId,
}

impl DnsEndpointAssignment {
    /// Associate independently verified credential metadata with its Endpoint.
    ///
    /// The caller still owns capability verification, current expiry/revocation
    /// and generation checks. This pure association cannot authenticate a token.
    pub fn binding_for_credential(
        &self,
        credential: &ResolverCredentialMetadata,
    ) -> Result<DnsProfileBinding, DnsBindingError> {
        if self.endpoint != credential.endpoint {
            return Err(DnsBindingError::WrongEndpoint);
        }
        if self.state != AdministrativeState::Enabled
            || credential.state != ResolverCredentialState::Active
        {
            return Err(DnsBindingError::Unavailable);
        }
        Ok(DnsProfileBinding {
            endpoint: self.endpoint,
            profile: self.profile,
        })
    }
}

/// A legacy DNS identity assignment, not a live/unexpired authorization grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LegacyDnsAssignment {
    /// Legacy DNS grant identity, distinct from any gateway grant ID.
    pub grant: LegacyNetworkGrantId,
    /// Shared group context.
    pub group: NetworkGroupId,
    /// Listener scope prevents accidental cross-listener reuse.
    pub listener: ListenerId,
    /// Exact address; the type does not represent a prefix or automatic /64.
    pub source: IpAddr,
    /// Assigned DNS Endpoint; its Profile comes from current Endpoint assignment.
    pub endpoint: EndpointId,
}

/// Transparent TCP identity is a separately established network/group context.
///
/// No DeviceId, resolver capability or latest DNS Endpoint proves this identity.
/// Destination/port/outbound scope and current authorization are enforced in P4-01.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GatewayNetworkIdentity {
    /// Independent gateway grant identity.
    pub grant: GatewayNetworkGrantId,
    /// Shared group context, not one device behind NAT.
    pub group: NetworkGroupId,
    /// Actual gateway listener scope.
    pub listener: ListenerId,
    /// Exact observed source address.
    pub source: IpAddr,
}

/// Select an unambiguous legacy association without merging conflicting scopes.
///
/// This resolves data associations only. The runtime must independently enforce
/// grant expiry/revocation, allowed sources, current generations and request limits.
pub fn unique_legacy_assignment(
    assignments: &[LegacyDnsAssignment],
    listener: ListenerId,
    source: IpAddr,
) -> Result<Option<&LegacyDnsAssignment>, AmbiguousLegacyAssignment> {
    let mut matches = assignments
        .iter()
        .filter(|entry| entry.listener == listener && entry.source == source);
    let selected = matches.next();
    if matches.next().is_some() {
        Err(AmbiguousLegacyAssignment)
    } else {
        Ok(selected)
    }
}

/// Public IDs with the same bytes remain different resource types.
///
/// ```compile_fail
/// use routira_common::{EndpointId, ProfileId};
/// fn endpoint(_: EndpointId) {}
/// fn confused(profile: ProfileId) { endpoint(profile); }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DnsBindingError {
    /// The credential does not belong to the selected Endpoint.
    WrongEndpoint,
    /// The Endpoint or credential is disabled, expired or revoked.
    Unavailable,
}

impl fmt::Display for DnsBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongEndpoint => "credential Endpoint mismatch",
            Self::Unavailable => "DNS association unavailable",
        })
    }
}

impl Error for DnsBindingError {}

/// Multiple assignments match the same exact source/listener; no union is allowed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmbiguousLegacyAssignment;

impl fmt::Display for AmbiguousLegacyAssignment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ambiguous legacy DNS assignment")
    }
}

impl Error for AmbiguousLegacyAssignment {}
