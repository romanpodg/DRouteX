use super::{MAX_RESOURCE_PINS, SnapshotError, SnapshotErrorCode as Code};
use crate::ReferenceCatalog;
use droutex_common::{
    DeviceId, DnsUpstreamId, FilterListRevisionId, SecretRefId, ServiceGroupRevisionId,
};
use droutex_security::{EncryptedSecret, SecretPurpose, sha256};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ContentHash([u8; 32]);
impl ContentHash {
    pub fn of(bytes: &[u8]) -> Self {
        Self(sha256(bytes))
    }
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Typed external identity. A hash cannot establish provenance or admission rights.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotResource {
    Device(DeviceId),
    Upstream(DnsUpstreamId),
    Secret {
        reference: SecretRefId,
        purpose: SecretPurpose,
    },
    Filter(FilterListRevisionId),
    Service(ServiceGroupRevisionId),
}

/// Convert only actual catalog identities; devices/upstreams/secrets never
/// become catalog revisions through an ID spelling shared with another kind.
impl TryFrom<SnapshotResource> for droutex_policy::CatalogRevision {
    type Error = SnapshotError;
    fn try_from(value: SnapshotResource) -> Result<Self, Self::Error> {
        match value {
            SnapshotResource::Filter(id) => Ok(Self::Filter(id)),
            SnapshotResource::Service(id) => Ok(Self::Service(id)),
            _ => Err(SnapshotError(Code::Resource)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Device,
    Upstream,
    Secret,
    Filter,
    Service,
}

impl SnapshotResource {
    fn key(self) -> (Kind, [u8; 16]) {
        let (kind, bytes) = match self {
            Self::Device(id) => (Kind::Device, *id.as_bytes()),
            Self::Upstream(id) => (Kind::Upstream, *id.as_bytes()),
            Self::Secret { reference, .. } => (Kind::Secret, *reference.as_bytes()),
            Self::Filter(id) => (Kind::Filter, *id.as_bytes()),
            Self::Service(id) => (Kind::Service, *id.as_bytes()),
        };
        (kind, bytes)
    }
    fn id(self) -> String {
        match self {
            Self::Device(id) => id.to_string(),
            Self::Upstream(id) => id.to_string(),
            Self::Secret { reference, .. } => reference.to_string(),
            Self::Filter(id) => id.to_string(),
            Self::Service(id) => id.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourcePin {
    resource: SnapshotResource,
    hash: ContentHash,
}
impl ResourcePin {
    /// Caller supplies independently verified actual resource bytes, not claimed DB hashes.
    /// Secret bytes must be a bounded encrypted envelope with the matching context.
    pub fn from_bytes(resource: SnapshotResource, bytes: &[u8]) -> Result<Self, SnapshotError> {
        if let SnapshotResource::Secret { reference, purpose } = resource {
            if bytes.len() > droutex_security::MAX_ENVELOPE_BYTES {
                return Err(SnapshotError(Code::Size));
            }
            let encrypted = EncryptedSecret::from_owned_bytes(bytes.to_vec())
                .map_err(|_| SnapshotError(Code::Resource))?;
            if encrypted.context().reference != reference || encrypted.context().purpose != purpose
            {
                return Err(SnapshotError(Code::Resource));
            }
        }
        Ok(Self {
            resource,
            hash: ContentHash::of(bytes),
        })
    }
    pub const fn resource(&self) -> SnapshotResource {
        self.resource
    }
    pub const fn hash(&self) -> ContentHash {
        self.hash
    }
    fn wire(self) -> WirePin {
        WirePin {
            kind: self.resource.key().0,
            id: self.resource.id(),
            purpose: match self.resource {
                SnapshotResource::Secret {
                    purpose: SecretPurpose::ResolverVerifier,
                    ..
                } => Some("resolver_verifier".into()),
                SnapshotResource::Secret {
                    purpose: SecretPurpose::ProxyOutboundCredential,
                    ..
                } => Some("outbound_credential".into()),
                _ => None,
            },
            hash: self.hash.to_string(),
        }
    }
}

/// Actual trusted dependency fingerprints. No bytes, plaintext or implicit DB trust.
#[derive(Debug)]
pub struct TrustedResources(BTreeMap<(Kind, [u8; 16]), ResourcePin>);
impl TrustedResources {
    pub fn new(pins: Vec<ResourcePin>) -> Result<Self, SnapshotError> {
        if pins.len() > MAX_RESOURCE_PINS {
            return Err(SnapshotError(Code::Size));
        }
        let mut resources = BTreeMap::new();
        for pin in pins {
            if resources.insert(pin.resource.key(), pin).is_some() {
                return Err(SnapshotError(Code::Resource));
            }
        }
        Ok(Self(resources))
    }
    pub(crate) fn catalog(&self) -> ReferenceCatalog {
        let mut catalog = ReferenceCatalog::default();
        for pin in self.0.values() {
            match pin.resource {
                SnapshotResource::Device(id) => {
                    catalog.devices.insert(id);
                }
                SnapshotResource::Upstream(id) => {
                    catalog.upstreams.insert(id);
                }
                SnapshotResource::Secret { reference, .. } => {
                    catalog.secrets.insert(reference);
                }
                SnapshotResource::Filter(id) => {
                    catalog.filter_revisions.insert(id);
                }
                SnapshotResource::Service(id) => {
                    catalog.service_revisions.insert(id);
                }
            }
        }
        catalog
    }
    pub(super) fn secret(&self, id: SecretRefId, purpose: SecretPurpose) -> bool {
        self.0.get(&(Kind::Secret, *id.as_bytes())).is_some_and(|pin|
            matches!(pin.resource, SnapshotResource::Secret { purpose: expected, .. } if purpose == expected))
    }
    pub(super) fn wire(&self) -> Vec<WirePin> {
        self.0.values().map(|pin| pin.wire()).collect()
    }
    pub(crate) fn pins(&self) -> Vec<ResourcePin> {
        self.0.values().copied().collect()
    }
    pub(super) fn select(&self, pins: &[WirePin]) -> Result<Self, SnapshotError> {
        if pins.len() > MAX_RESOURCE_PINS {
            return Err(SnapshotError(Code::Size));
        }
        let mut selected = Vec::with_capacity(pins.len());
        for wire in pins {
            // Compare against canonical independent fingerprints, never elevate a wire hash.
            let invalid = |_| SnapshotError(Code::Resource);
            let bytes = match wire.kind {
                Kind::Device => *wire.id.parse::<DeviceId>().map_err(invalid)?.as_bytes(),
                Kind::Upstream => *wire
                    .id
                    .parse::<DnsUpstreamId>()
                    .map_err(invalid)?
                    .as_bytes(),
                Kind::Secret => *wire.id.parse::<SecretRefId>().map_err(invalid)?.as_bytes(),
                Kind::Filter => *wire
                    .id
                    .parse::<FilterListRevisionId>()
                    .map_err(invalid)?
                    .as_bytes(),
                Kind::Service => *wire
                    .id
                    .parse::<ServiceGroupRevisionId>()
                    .map_err(invalid)?
                    .as_bytes(),
            };
            let pin = self
                .0
                .get(&(wire.kind, bytes))
                .ok_or(SnapshotError(Code::Resource))?;
            if pin.wire() != *wire {
                return Err(SnapshotError(Code::Resource));
            }
            selected.push(*pin);
        }
        Self::new(selected)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct WirePin {
    kind: Kind,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    purpose: Option<String>,
    hash: String,
}
