//! Private D43 decoder: only the complete CP/verified-DP compilation path calls it.
//! Provenance hashes are CP attestations here, not a rehash of absent raw source.
use super::{CatalogImportError as Error, CatalogMetadata, codec, scan};
use crate::schema::BoundedValue;
use droutex_common::MAX_LOCAL_INPUT_BYTES;
use droutex_policy::{
    CanonicalDomain, CatalogConditions, CatalogPin, CatalogRevision, DomainMatchMode,
    DomainMatcher, normalize_catalog_conditions,
};
use serde::{Deserialize, Serialize};

pub(crate) struct DecodedCatalog {
    pub(crate) conditions: CatalogConditions,
    pub(crate) dependencies: Vec<CatalogPin>,
}

/// Scan ALL resources before any buffered decoder/normalization is invoked.
pub(crate) fn preflight(bytes: &[u8], remaining: &mut usize) -> Result<(), Error> {
    scan::json_entries(bytes, remaining)
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Resource {
    schema_version: u32,
    kind: String,
    revision: String,
    format: String,
    mode: Option<String>,
    metadata: CatalogMetadata,
    source_hash: String,
    approval_evidence: String,
    entries: Vec<Condition>,
    dependencies: Vec<Pin>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Condition {
    domain: String,
    mode: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    kind: String,
    revision: String,
    hash: String,
}

fn identity(kind: &str, text: &str) -> Result<CatalogRevision, Error> {
    let (value, canonical) = match kind {
        "filter" => {
            let id: droutex_common::FilterListRevisionId =
                text.parse().map_err(|_| Error::Shape)?;
            (CatalogRevision::Filter(id), id.to_string())
        }
        "service" => {
            let id: droutex_common::ServiceGroupRevisionId =
                text.parse().map_err(|_| Error::Shape)?;
            (CatalogRevision::Service(id), id.to_string())
        }
        _ => return Err(Error::Shape),
    };
    if canonical != text {
        return Err(Error::Shape);
    }
    Ok(value)
}
fn mode(value: &str) -> Result<DomainMatchMode, Error> {
    match value {
        "exact" => Ok(DomainMatchMode::Exact),
        "domain_and_subdomains" => Ok(DomainMatchMode::DomainAndSubdomains),
        "subdomains_only" => Ok(DomainMatchMode::SubdomainsOnly),
        _ => Err(Error::Shape),
    }
}
fn hash(value: &str) -> Result<[u8; 32], Error> {
    if value
        .bytes()
        .any(|b| !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b))
    {
        return Err(Error::Shape);
    }
    codec::decode_hash(value.as_bytes())
}

pub(crate) fn decode(bytes: &[u8], expected: CatalogRevision) -> Result<DecodedCatalog, Error> {
    // Caller already bounded/scanned the complete byte set and checked actual pins.
    let value: BoundedValue = serde_json::from_slice(bytes).map_err(|_| Error::Shape)?;
    let raw: Resource = serde_path_to_error::deserialize(value).map_err(|_| Error::Shape)?;
    if raw.schema_version != 1 || identity(&raw.kind, &raw.revision)? != expected {
        return Err(Error::Shape);
    }
    let explicit = match (expected, raw.format.as_str(), raw.mode.as_deref()) {
        (CatalogRevision::Filter(_), "domains" | "blocking_hosts", Some(m)) => Some(mode(m)?),
        (CatalogRevision::Service(_), "service_json", None) => None,
        _ => return Err(Error::Shape),
    };
    let metadata_bytes = raw
        .metadata
        .bytes()
        .and_then(|n| n.checked_add(raw.metadata.license.len()))
        .and_then(|n| n.checked_add(raw.approval_evidence.len()))
        .ok_or(Error::Size)?;
    if metadata_bytes > MAX_LOCAL_INPUT_BYTES
        || !raw.metadata.valid()
        // D41 approval is stricter than multiline descriptive metadata: license
        // identities can never contain LF/CR/TAB or any other control character.
        || raw.metadata.license.chars().any(char::is_control)
        || raw.approval_evidence.trim().is_empty()
        || raw.approval_evidence.chars().any(char::is_control)
    {
        return Err(Error::Shape);
    }
    hash(&raw.source_hash)?;
    let entries = raw
        .entries
        .iter()
        .map(|entry| {
            let domain: CanonicalDomain = entry.domain.parse().map_err(|_| Error::Shape)?;
            let entry_mode = mode(&entry.mode)?;
            if domain.as_str() != entry.domain || explicit.is_some_and(|m| m != entry_mode) {
                return Err(Error::Shape);
            }
            Ok(DomainMatcher::new(domain, entry_mode))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let conditions = normalize_catalog_conditions(entries).map_err(Error::Input)?;
    if conditions.entries().len() != raw.entries.len()
        || conditions
            .entries()
            .iter()
            .zip(&raw.entries)
            .any(|(a, b)| a.domain().as_str() != b.domain || mode(&b.mode) != Ok(a.mode()))
    {
        return Err(Error::Shape);
    }
    let dependencies = raw
        .dependencies
        .iter()
        .map(|pin| {
            Ok(CatalogPin {
                revision: identity(&pin.kind, &pin.revision)?,
                hash: hash(&pin.hash)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    if dependencies.windows(2).any(|p| p[0] > p[1])
        || (matches!(expected, CatalogRevision::Filter(_)) && !dependencies.is_empty())
        || codec::canonical_json(&raw, bytes.len())? != bytes
    {
        return Err(Error::Shape);
    }
    Ok(DecodedCatalog {
        conditions,
        dependencies,
    })
}
