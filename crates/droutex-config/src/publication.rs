//! D39 config/resource checks. P2-05 must also validate its complete prepared
//! policy set and bind all proofs to one generation; this is not a compiler.
use crate::{CheckedConfiguration, ConfigError, RecordData, ReferenceCatalog};
use droutex_common::PrivateDnsRecordId;
use droutex_policy::{
    CatalogPin, CatalogRevision, CatalogRevisionFacts, CatalogValidationError,
    PreparedPrivateRecord, PrivateRecordValidationError, PrivateRecordValue,
    validate_catalog_dependencies, validate_private_records,
};
use std::{collections::BTreeSet, error::Error, fmt};

#[derive(Debug, Eq, PartialEq)]
pub enum PublicationResourceError {
    Configuration(ConfigError),
    MissingCatalogPin(CatalogRevision),
    Domain(PrivateDnsRecordId),
    Catalog(CatalogValidationError),
    PrivateRecord(PrivateRecordValidationError),
}
impl fmt::Display for PublicationResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("configuration resources are not valid for publication")
    }
}
impl Error for PublicationResourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Configuration(e) => Some(e),
            Self::Catalog(e) => Some(e),
            Self::PrivateRecord(e) => Some(e),
            Self::MissingCatalogPin(_) | Self::Domain(_) => None,
        }
    }
}

impl CheckedConfiguration {
    /// Recheck removed references/scope ambiguity against current inventory,
    /// then require pins and trusted license/hash/provenance facts for every
    /// selected revision, including disabled rules and gateway service grants.
    /// Canonical private records are checked without changing source/signed bytes.
    ///
    /// This does not derive service/filter conditions or validate prepared rules:
    /// P2-07 must supply verified actual data, and P2-05 must also call
    /// PolicySet::validate_conflicts on the COMPLETE same-generation compilation.
    /// An existence-only ReferenceCatalog or this Result is not publication authority.
    pub fn validate_publication_resources(
        &self,
        current: &ReferenceCatalog,
        pins: &[CatalogPin],
        facts: &[CatalogRevisionFacts],
    ) -> Result<(), PublicationResourceError> {
        self.revalidate(current)
            .map_err(PublicationResourceError::Configuration)?;
        // First enforce the aggregate pin/fact/edge budget before building a set.
        validate_catalog_dependencies(pins, facts).map_err(PublicationResourceError::Catalog)?;
        let available: BTreeSet<_> = pins.iter().map(|p| p.revision).collect();
        let document = self.document();
        let mut selected = BTreeSet::new();
        for profile in &document.profiles {
            selected.extend(
                profile
                    .service_rules
                    .iter()
                    .map(|r| CatalogRevision::Service(r.revision)),
            );
            selected.extend(
                profile
                    .filter_lists
                    .iter()
                    .map(|r| CatalogRevision::Filter(r.revision)),
            );
        }
        selected.extend(
            document
                .gateway_grants
                .iter()
                .flat_map(|g| &g.services)
                .copied()
                .map(CatalogRevision::Service),
        );
        if let Some(missing) = selected.difference(&available).next() {
            return Err(PublicationResourceError::MissingCatalogPin(*missing));
        }
        let records = document
            .private_records
            .iter()
            .map(|record| {
                let invalid = |_| PublicationResourceError::Domain(record.id);
                Ok(PreparedPrivateRecord {
                    id: record.id,
                    profile: record.profile,
                    owner: record.name.parse().map_err(invalid)?,
                    ttl: record.ttl,
                    value: match &record.record {
                        RecordData::A { address } => PrivateRecordValue::Ipv4(*address),
                        RecordData::Aaaa { address } => PrivateRecordValue::Ipv6(*address),
                        RecordData::Cname { target } => {
                            PrivateRecordValue::Cname(target.parse().map_err(invalid)?)
                        }
                    },
                })
            })
            .collect::<Result<Vec<_>, PublicationResourceError>>()?;
        validate_private_records(&records).map_err(PublicationResourceError::PrivateRecord)
    }
}
