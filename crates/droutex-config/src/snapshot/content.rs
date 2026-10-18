use super::{
    ContentHash, MAX_SNAPSHOT_BYTES, SnapshotError, SnapshotErrorCode as Code, TrustedResources,
    canonical, resources::WirePin,
};
use crate::{
    CheckedConfiguration, InputFormat, MAX_INPUT_BYTES, MAX_RESOURCE_ENTRIES, OutboundKind,
    parse_configuration,
};
use droutex_common::{ResolverCredentialId, SecretRefId};
use droutex_security::SecretPurpose;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CredentialLink {
    pub credential: ResolverCredentialId,
    pub reference: SecretRefId,
}

/// Immutable canonical checked input, still not compiled policy or runtime authority.
pub struct SnapshotContent {
    bytes: Vec<u8>,
    checked: CheckedConfiguration,
    links: Vec<CredentialLink>,
    hash: ContentHash,
    pins: Vec<super::ResourcePin>,
}
impl SnapshotContent {
    pub fn new(
        checked: CheckedConfiguration,
        mut links: Vec<CredentialLink>,
        resources: &TrustedResources,
    ) -> Result<Self, SnapshotError> {
        let configuration = canonical::configuration(&checked)?;
        let configuration_bytes =
            serde_json::to_vec(&configuration).map_err(|_| SnapshotError(Code::Canonical))?;
        // Recheck against these exact trusted pins, not the catalog used for an old import.
        let checked = parse_configuration(
            &configuration_bytes,
            InputFormat::Json,
            &resources.catalog(),
        )
        .map_err(|_| SnapshotError(Code::Configuration))?;
        validate_links(&checked, &links, resources)?;
        links.sort();
        let payload = WirePayload {
            configuration,
            credential_links: links
                .iter()
                .map(|link| WireLink {
                    credential: link.credential.to_string(),
                    reference: link.reference.to_string(),
                })
                .collect(),
            resources: resources.wire(),
        };
        // Value maps use sorted keys in the pinned profile; no preserve_order feature.
        let value = serde_json::to_value(payload).map_err(|_| SnapshotError(Code::Canonical))?;
        let bytes = serde_json::to_vec(&value).map_err(|_| SnapshotError(Code::Canonical))?;
        if bytes.len() > MAX_SNAPSHOT_BYTES - super::envelope::OVERHEAD {
            return Err(SnapshotError(Code::Size));
        }
        let hash = ContentHash::of(&bytes);
        Ok(Self {
            bytes,
            checked,
            links,
            hash,
            pins: resources.pins(),
        })
    }

    pub const fn hash(&self) -> ContentHash {
        self.hash
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn configuration(&self) -> &CheckedConfiguration {
        &self.checked
    }
    pub fn credential_links(&self) -> &[CredentialLink] {
        &self.links
    }
    pub fn resource_pins(&self) -> &[super::ResourcePin] {
        &self.pins
    }

    pub(super) fn decode(bytes: &[u8], trusted: &TrustedResources) -> Result<Self, SnapshotError> {
        let value: Value =
            serde_json::from_slice(bytes).map_err(|_| SnapshotError(Code::Format))?;
        canonical::check_tree(&value, 1)?;
        let payload: WirePayload =
            serde_json::from_value(value).map_err(|_| SnapshotError(Code::Format))?;
        if payload.credential_links.len() > MAX_RESOURCE_ENTRIES {
            return Err(SnapshotError(Code::Size));
        }
        let resources = trusted.select(&payload.resources)?;
        let config_bytes =
            serde_json::to_vec(&payload.configuration).map_err(|_| SnapshotError(Code::Format))?;
        if config_bytes.len() > MAX_INPUT_BYTES {
            return Err(SnapshotError(Code::Size));
        }
        let checked = parse_configuration(&config_bytes, InputFormat::Json, &resources.catalog())
            .map_err(|_| SnapshotError(Code::Configuration))?;
        let links = payload
            .credential_links
            .into_iter()
            .map(|link| {
                Ok(CredentialLink {
                    credential: link
                        .credential
                        .parse()
                        .map_err(|_| SnapshotError(Code::CredentialLink))?,
                    reference: link
                        .reference
                        .parse()
                        .map_err(|_| SnapshotError(Code::CredentialLink))?,
                })
            })
            .collect::<Result<Vec<_>, SnapshotError>>()?;
        let content = Self::new(checked, links, &resources)?;
        // Also refuses duplicate keys, noncanonical order/default/number/escape spellings.
        if content.bytes != bytes {
            return Err(SnapshotError(Code::Canonical));
        }
        Ok(content)
    }
}

impl fmt::Debug for SnapshotContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnapshotContent")
            .field("hash", &self.hash)
            .finish_non_exhaustive()
    }
}

fn validate_links(
    checked: &CheckedConfiguration,
    links: &[CredentialLink],
    resources: &TrustedResources,
) -> Result<(), SnapshotError> {
    let credentials: BTreeSet<_> = checked
        .document()
        .resolver_credentials
        .iter()
        .map(|c| c.id)
        .collect();
    if links.len() != credentials.len() {
        return Err(SnapshotError(Code::CredentialLink));
    }
    let mut seen = BTreeSet::new();
    for link in links {
        if !credentials.contains(&link.credential)
            || !seen.insert(link.credential)
            || !resources.secret(link.reference, SecretPurpose::ResolverVerifier)
        {
            return Err(SnapshotError(Code::CredentialLink));
        }
    }
    for outbound in &checked.document().outbounds {
        if let OutboundKind::HttpConnectTls {
            secret: Some(reference),
            ..
        } = outbound.transport
            && !resources.secret(reference, SecretPurpose::ProxyOutboundCredential)
        {
            return Err(SnapshotError(Code::Resource));
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WirePayload {
    configuration: Value,
    credential_links: Vec<WireLink>,
    resources: Vec<WirePin>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireLink {
    credential: String,
    reference: String,
}
