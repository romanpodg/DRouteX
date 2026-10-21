//! D43 complete immutable compilation. No live identity, address or connector authority.
mod budget;
mod build;
mod resources;
mod scope;

use crate::{
    CheckedConfiguration, DnsAction, PublicationResourceError, TrafficAction,
    catalog::{CatalogImportError, VerifiedCatalogRevision},
    snapshot::{
        self, CredentialLink, SnapshotContent, SnapshotError, SnapshotMetadata, SnapshotResource,
    },
};
use droutex_common::{GatewayNetworkGrantId, LegacyDnsAssignment, LegacyNetworkGrantId};
use droutex_policy::{
    CatalogPin, CatalogRevision, PolicyBuildError, PolicySet, PolicyValidationError,
    PreparedPrivateRecord,
};
use droutex_security::SigningKey;
pub use scope::GatewayScope;
use std::{collections::BTreeMap, error::Error, fmt};

/// Borrowed actual input, with no trust or license assertion conferred by construction.
#[derive(Clone, Copy)]
pub struct ResourceData<'a> {
    pub resource: SnapshotResource,
    pub bytes: &'a [u8],
}

#[derive(Debug, Eq, PartialEq)]
pub enum CompilationError {
    Size,
    Resource,
    CatalogApproval(CatalogRevision),
    Catalog(CatalogImportError),
    Snapshot(SnapshotError),
    Publication(PublicationResourceError),
    Policy(PolicyBuildError),
    Conflict(PolicyValidationError),
    Domain,
}
impl fmt::Display for CompilationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("complete generation compilation refused")
    }
}
impl Error for CompilationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(e) => Some(e),
            Self::Snapshot(e) => Some(e),
            Self::Publication(e) => Some(e),
            Self::Policy(e) => Some(e),
            Self::Conflict(e) => Some(e),
            _ => None,
        }
    }
}
type Result<T> = std::result::Result<T, CompilationError>;

/// Owned derived tables, constructed only by complete CP or authenticated DP paths.
/// It is neither a live grant nor a serializable substitute for D11 source bytes.
/// ```compile_fail
/// use droutex_config::CompiledConfiguration;
/// let tables: CompiledConfiguration = serde_json::from_str("{}").unwrap();
/// ```
pub struct CompiledConfiguration {
    policy: PolicySet<DnsAction, TrafficAction>,
    private: Vec<PreparedPrivateRecord>,
    legacy: BTreeMap<LegacyNetworkGrantId, LegacyDnsAssignment>,
    gateway: BTreeMap<GatewayNetworkGrantId, GatewayScope>,
    catalogs: BTreeMap<CatalogRevision, CatalogManifest>,
}
struct CatalogManifest {
    pin: CatalogPin,
    dependencies: Vec<CatalogPin>,
}
impl CompiledConfiguration {
    pub fn policy(&self) -> &PolicySet<DnsAction, TrafficAction> {
        &self.policy
    }
    /// Canonical validated data only; P3 chooses response/precedence behavior.
    pub fn private_records(&self) -> &[PreparedPrivateRecord] {
        &self.private
    }
    /// Static desired scope only; actual authentication/revocation remains P3/P4.
    pub fn legacy_scope(&self, id: LegacyNetworkGrantId) -> Option<&LegacyDnsAssignment> {
        self.legacy.get(&id)
    }
    pub fn gateway_scope(&self, id: GatewayNetworkGrantId) -> Option<&GatewayScope> {
        self.gateway.get(&id)
    }
    pub fn catalog_pin(&self, id: CatalogRevision) -> Option<CatalogPin> {
        self.catalogs.get(&id).map(|c| c.pin)
    }
    pub fn catalog_dependencies(&self, id: CatalogRevision) -> Option<&[CatalogPin]> {
        self.catalogs.get(&id).map(|c| c.dependencies.as_slice())
    }
}
impl fmt::Debug for CompiledConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledConfiguration")
            .field("policy", &self.policy)
            .field("private_records", &self.private.len())
            .field("legacy_scopes", &self.legacy.len())
            .field("gateway_scopes", &self.gateway.len())
            .field("catalogs", &self.catalogs.len())
            .finish()
    }
}

/// Complete controller builder. A feed-supplied approval Boolean cannot enter here.
pub struct CompletePublication {
    content: SnapshotContent,
    compiled: CompiledConfiguration,
}
impl CompletePublication {
    pub fn new(
        configuration: CheckedConfiguration,
        links: Vec<CredentialLink>,
        resources: &[ResourceData<'_>],
        catalogs: &[&VerifiedCatalogRevision],
    ) -> Result<Self> {
        let actual = resources::actual(resources)?;
        resources::approved(resources, catalogs)?;
        let content = SnapshotContent::new(configuration, links, &actual)
            .map_err(CompilationError::Snapshot)?;
        let compiled = compile(&content, resources)?;
        Ok(Self { content, compiled })
    }
    pub fn content(&self) -> &SnapshotContent {
        &self.content
    }
    pub fn compiled(&self) -> &CompiledConfiguration {
        &self.compiled
    }
    /// Only this complete builder is the CP publication signing path. The low-level
    /// D11 codec remains available for cryptographic work, not publication authority.
    pub fn encode(&self, metadata: SnapshotMetadata, key: &SigningKey) -> Result<Vec<u8>> {
        snapshot::encode_snapshot(&self.content, metadata, key).map_err(CompilationError::Snapshot)
    }
}

impl snapshot::VerifiedSnapshot {
    /// Decode only after CP authentication and resource fingerprint verification.
    pub fn compile(&self, resources: &[ResourceData<'_>]) -> Result<CompiledConfiguration> {
        compile(self.content(), resources)
    }
}
impl snapshot::RestoredSnapshot {
    pub fn compile(&self, resources: &[ResourceData<'_>]) -> Result<CompiledConfiguration> {
        compile(self.content(), resources)
    }
}

fn compile(content: &SnapshotContent, bytes: &[ResourceData<'_>]) -> Result<CompiledConfiguration> {
    let actual = resources::actual(bytes)?;
    if actual.pins() != content.resource_pins() {
        return Err(CompilationError::Resource);
    }
    let catalogs = resources::decode(bytes)?;
    let document = content.configuration().document();
    budget::check(document, &catalogs)?;
    let pins: Vec<_> = catalogs.values().map(|c| c.pin).collect();
    let facts = resources::facts(&catalogs)?;
    content
        .configuration()
        .validate_publication_resources(&actual.catalog(), &pins, &facts)
        .map_err(CompilationError::Publication)?;
    let policy = build::policy(document, &catalogs)?;
    policy
        .validate_conflicts()
        .map_err(CompilationError::Conflict)?;
    let private = build::private(document)?;
    let (legacy, gateway) = scope::build(document, &catalogs)?;
    Ok(CompiledConfiguration {
        policy,
        private,
        legacy,
        gateway,
        catalogs: catalogs
            .into_iter()
            .map(|(id, c)| {
                (
                    id,
                    CatalogManifest {
                        pin: c.pin,
                        dependencies: c.data.dependencies,
                    },
                )
            })
            .collect(),
    })
}
