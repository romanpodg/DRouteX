use std::{error::Error, fmt, str::FromStr};
use uuid::{Uuid, Variant, Version};

/// The input is not an RFC-variant UUIDv4 resource identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidResourceId;

impl fmt::Display for InvalidResourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("expected a UUIDv4 resource identifier")
    }
}

impl Error for InvalidResourceId {}

fn checked_id(id: Uuid) -> Result<Uuid, InvalidResourceId> {
    if id.get_variant() == Variant::RFC4122 && id.get_version() == Some(Version::Random) {
        Ok(id)
    } else {
        Err(InvalidResourceId)
    }
}

macro_rules! resource_ids {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {$ (
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(Uuid);

        impl $name {
            /// Accept already minted UUIDv4 bytes; this does not generate randomness.
            pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, InvalidResourceId> {
                checked_id(Uuid::from_bytes(bytes)).map(Self)
            }

            /// The UUID's standard byte order, without exposing a dependency type.
            pub fn as_bytes(&self) -> &[u8; 16] {
                self.0.as_bytes()
            }
        }

        impl FromStr for $name {
            type Err = InvalidResourceId;

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                checked_id(Uuid::parse_str(input).map_err(|_| InvalidResourceId)?).map(Self)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    )+};
}

resource_ids! {
    /// A reusable DNS policy Profile, not an access credential.
    ProfileId,
    /// Public resolver configuration identity; never a resolver bearer secret.
    EndpointId,
    /// Public credential-record identity, not the secret capability material.
    ResolverCredentialId,
    /// Optional device metadata; it proves no network identity.
    DeviceId,
    /// A filter-list resource.
    FilterListId,
    /// One immutable filter-list revision.
    FilterListRevisionId,
    /// A curated service-group resource.
    ServiceGroupId,
    /// One immutable service-group revision.
    ServiceGroupRevisionId,
    /// A Profile-owned private DNS record.
    PrivateDnsRecordId,
    /// A rule with stable identity independent of its priority.
    RuleId,
    /// A configured native direct or protected proxy outbound.
    ProxyOutboundId,
    /// A DNS or gateway listener resource.
    ListenerId,
    /// A shared network/group context; never an individual device.
    NetworkGroupId,
    /// Legacy DNS grant identity, distinct from a gateway grant.
    LegacyNetworkGrantId,
    /// Gateway grant identity; possession of its ID grants no access.
    GatewayNetworkGrantId,
    /// Immutable snapshot content identity, not its increasing activation sequence.
    SnapshotId,
    /// A snapshot publication/application attempt.
    DeploymentId,
}
