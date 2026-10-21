//! D41 local import. Verified objects cannot be deserialized from claimed facts.
mod codec;
mod scan;
mod store;
use crate::snapshot::ContentHash;
use droutex_common::MAX_LOCAL_INPUT_BYTES;
use droutex_policy::{
    BlocklistFormat, CatalogConditions, CatalogLicense, CatalogParseError, CatalogPin,
    CatalogProvenance, CatalogRevision, CatalogRevisionFacts, CatalogValidationError,
    CatalogValidationState, DomainMatchMode,
};
use serde::Serialize;
use std::{error::Error, fmt};
pub use store::CatalogStore;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogFormat {
    Domains(DomainMatchMode),
    BlockingHosts(DomainMatchMode),
    ServiceJson,
}
impl CatalogFormat {
    fn list(self) -> Option<(BlocklistFormat, DomainMatchMode)> {
        match self {
            Self::Domains(mode) => Some((BlocklistFormat::Domains, mode)),
            Self::BlockingHosts(mode) => Some((BlocklistFormat::BlockingHosts, mode)),
            Self::ServiceJson => None,
        }
    }
}

/// Trusted operator-supplied provenance, not an authenticity claim read from a feed.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogMetadata {
    pub source: String,
    pub owner: String,
    pub license: String,
    pub notice: String,
}
impl CatalogMetadata {
    fn bytes(&self) -> Option<usize> {
        [&self.source, &self.owner, &self.license, &self.notice]
            .iter()
            .try_fold(0usize, |n, s| n.checked_add(s.len()))
    }
    fn valid(&self) -> bool {
        [&self.source, &self.owner, &self.license, &self.notice]
            .iter()
            .all(|s| {
                !s.trim().is_empty()
                    && !s
                        .chars()
                        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            })
    }
}

/// No identity/approval is inferred from bytes. Metadata is supplied by a trusted
/// operator boundary; actual auth/RBAC remains P6, not possession of public IDs.
pub struct CatalogInput<'a> {
    pub revision: CatalogRevision,
    pub format: CatalogFormat,
    pub metadata: CatalogMetadata,
    pub bytes: &'a [u8],
}

/// Explicit trusted approval bound to actual source hash and license identity.
/// This is not authentication; only an authorized operator boundary may mint it.
/// It deliberately has no Deserialize implementation or feed-controlled Boolean.
pub struct CatalogLicenseApproval {
    revision: CatalogRevision,
    source_hash: ContentHash,
    license: String,
    evidence: String,
}
impl CatalogLicenseApproval {
    pub fn new(
        revision: CatalogRevision,
        source_hash: ContentHash,
        license: String,
        evidence: String,
    ) -> Result<Self, CatalogImportError> {
        if [&license, &evidence].iter().any(|s| {
            s.len() > MAX_LOCAL_INPUT_BYTES
                || s.trim().is_empty()
                || s.chars().any(char::is_control)
        }) {
            return Err(CatalogImportError::License);
        }
        Ok(Self {
            revision,
            source_hash,
            license,
            evidence,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum CatalogImportError {
    Size,
    Empty,
    Shape,
    /// Zero-based entry position; domain text and parser details are omitted.
    ServiceEntry {
        index: usize,
    },
    /// Zero-based dependency position; only the position is exposed.
    ServiceDependency {
        index: usize,
    },
    License,
    DuplicateApproval(CatalogRevision),
    UnknownApproval(CatalogRevision),
    DuplicateRevision(CatalogRevision),
    ImmutableRevision(CatalogRevision),
    Document {
        line: usize,
        column: usize,
    },
    Input(CatalogParseError),
    Dependencies(CatalogValidationError),
    Encoding,
}
impl fmt::Display for CatalogImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("local catalog update refused")
    }
}
impl Error for CatalogImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Input(e) => Some(e),
            Self::Dependencies(e) => Some(e),
            _ => None,
        }
    }
}

/// Complete immutable revision, exposed only after whole-set validation/commit.
/// No public constructor, deserializer or setter can manufacture verified facts.
/// ```compile_fail
/// use droutex_config::catalog::VerifiedCatalogRevision;
/// let revision: VerifiedCatalogRevision = serde_json::from_str("{}").unwrap();
/// ```
pub struct VerifiedCatalogRevision(PreparedRevision);
struct PreparedRevision {
    revision: CatalogRevision,
    format: CatalogFormat,
    metadata: CatalogMetadata,
    source: Vec<u8>,
    source_hash: ContentHash,
    conditions: CatalogConditions,
    dependencies: Vec<CatalogPin>,
    resource: Vec<u8>,
    hash: ContentHash,
    approval_evidence: String,
}
impl PreparedRevision {
    fn pin(&self) -> CatalogPin {
        CatalogPin {
            revision: self.revision,
            hash: *self.hash.as_bytes(),
        }
    }
    // Copy bounded pin values for validation without consuming retained data.
    fn facts(&self) -> CatalogRevisionFacts {
        CatalogRevisionFacts {
            revision: self.revision,
            actual_hash: *self.hash.as_bytes(),
            license: CatalogLicense::Approved,
            provenance: CatalogProvenance::Recorded,
            validation: CatalogValidationState::Valid,
            dependencies: self.dependencies.clone(),
        }
    }
}
impl VerifiedCatalogRevision {
    pub fn revision(&self) -> CatalogRevision {
        self.0.revision
    }
    pub fn format(&self) -> CatalogFormat {
        self.0.format
    }
    pub fn metadata(&self) -> &CatalogMetadata {
        &self.0.metadata
    }
    pub fn source_bytes(&self) -> &[u8] {
        &self.0.source
    }
    pub fn source_hash(&self) -> ContentHash {
        self.0.source_hash
    }
    pub fn conditions(&self) -> &CatalogConditions {
        &self.0.conditions
    }
    pub fn dependencies(&self) -> &[CatalogPin] {
        &self.0.dependencies
    }
    pub fn resource_bytes(&self) -> &[u8] {
        &self.0.resource
    }
    pub fn pin(&self) -> CatalogPin {
        self.0.pin()
    }
    pub fn approval_evidence(&self) -> &str {
        &self.0.approval_evidence
    }
    /// Owned D39 validation input; later compilation must bind its actual data
    /// and complete pins/config/resources to the same generation.
    pub fn facts(&self) -> CatalogRevisionFacts {
        self.0.facts()
    }
}
impl fmt::Debug for VerifiedCatalogRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerifiedCatalogRevision")
            .field("revision", &self.0.revision)
            .field("conditions", &self.0.conditions.entries().len())
            .field("dependencies", &self.0.dependencies.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogImportSummary {
    pub supplied_revisions: usize,
    /// Raw conditions plus dependency pins, before condition deduplication.
    pub raw_entries: usize,
    /// Canonical domain conditions; pins are separately available on revisions.
    pub canonical_entries: usize,
}
