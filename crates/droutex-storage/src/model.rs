use crate::{StorageError, StorageErrorCode};
use droutex_common::{
    ConfigurationRevisionId, FilterListRevisionId, ResolverCredentialId, SecretRefId,
    ServiceGroupRevisionId,
};
use droutex_config::{
    CheckedConfiguration, InputFormat, MAX_RESOURCE_ENTRIES, ReferenceCatalog, RevisionProvenance,
    parse_configuration,
};
use std::{collections::BTreeSet, fmt};

/// A claimed content identity only; dataset import/verification remains P2-07.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CatalogRevision {
    Filter(FilterListRevisionId),
    Service(ServiceGroupRevisionId),
}

impl CatalogRevision {
    pub(crate) fn parts(&self) -> (&'static str, &[u8; 16]) {
        match self {
            Self::Filter(id) => ("filter", id.as_bytes()),
            Self::Service(id) => ("service", id.as_bytes()),
        }
    }
}

/// Public metadata pointing to future protected storage, not verifier material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CredentialVerifierLink {
    pub credential: ResolverCredentialId,
    pub reference: SecretRefId,
}

/// Validated owned input for one immutable revision; no mutation/activation API.
pub struct ConfigurationWrite {
    pub(crate) id: ConfigurationRevisionId,
    pub(crate) format: InputFormat,
    pub(crate) input: Vec<u8>,
    pub(crate) checked: CheckedConfiguration,
    pub(crate) links: Vec<CredentialVerifierLink>,
}

impl ConfigurationWrite {
    /// Parse synchronously before queueing; large input never enters the DB writer.
    /// The catalog is caller-trusted metadata, never proof of actual secret validity.
    pub fn parse(
        id: ConfigurationRevisionId,
        input: Vec<u8>,
        format: InputFormat,
        catalog: &ReferenceCatalog,
        links: Vec<CredentialVerifierLink>,
    ) -> Result<Self, StorageError> {
        let checked =
            parse_configuration(&input, format, catalog).map_err(StorageError::configuration)?;
        validate_links(&checked, &links, catalog)?;
        Ok(Self {
            id,
            format,
            input,
            checked,
            links,
        })
    }
}

impl fmt::Debug for ConfigurationWrite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConfigurationWrite([REDACTED])")
    }
}

pub struct StoredConfiguration {
    pub(crate) id: ConfigurationRevisionId,
    pub(crate) provenance: RevisionProvenance,
    pub(crate) format: InputFormat,
    pub(crate) input: Vec<u8>,
    pub(crate) links: Vec<CredentialVerifierLink>,
}

impl StoredConfiguration {
    pub fn id(&self) -> ConfigurationRevisionId {
        self.id
    }

    pub fn provenance(&self) -> RevisionProvenance {
        self.provenance
    }

    /// Revalidate stored bytes against current trusted references before using them.
    /// This still does not compile policy or authenticate a runtime credential.
    pub fn check(&self, catalog: &ReferenceCatalog) -> Result<CheckedConfiguration, StorageError> {
        let checked = parse_configuration(&self.input, self.format, catalog)
            .map_err(StorageError::configuration)?;
        validate_links(&checked, &self.links, catalog)?;
        Ok(checked)
    }

    pub fn credential_links(&self) -> &[CredentialVerifierLink] {
        &self.links
    }
}

impl fmt::Debug for StoredConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StoredConfiguration([REDACTED])")
    }
}

fn validate_links(
    checked: &CheckedConfiguration,
    links: &[CredentialVerifierLink],
    catalog: &ReferenceCatalog,
) -> Result<(), StorageError> {
    let credentials: BTreeSet<_> = checked
        .document()
        .resolver_credentials
        .iter()
        .map(|c| c.id)
        .collect();
    let mut seen = BTreeSet::new();
    if links.len() > MAX_RESOURCE_ENTRIES
        || links.iter().any(|link| {
            !credentials.contains(&link.credential)
                || !catalog.secrets.contains(&link.reference)
                || !seen.insert(link.credential)
        })
    {
        return Err(StorageError::new(StorageErrorCode::InvalidCredentialLink));
    }
    Ok(())
}
