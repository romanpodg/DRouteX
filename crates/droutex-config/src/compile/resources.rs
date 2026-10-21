use super::{CompilationError as Error, ResourceData, Result};
use crate::{
    catalog::{VerifiedCatalogRevision, decoded},
    snapshot::{ResourcePin, SnapshotResource, TrustedResources},
};
use droutex_common::MAX_GENERATION_RESOURCE_BYTES;
use droutex_policy::{
    CatalogLicense, CatalogPin, CatalogProvenance, CatalogRevision, CatalogRevisionFacts,
    CatalogValidationState, MAX_POLICY_ENTRIES,
};
use std::collections::BTreeMap;

pub(super) struct Catalog {
    pub(super) pin: CatalogPin,
    pub(super) data: decoded::DecodedCatalog,
}
pub(super) fn revision(resource: SnapshotResource) -> Option<CatalogRevision> {
    resource.try_into().ok()
}
pub(super) fn actual(data: &[ResourceData<'_>]) -> Result<TrustedResources> {
    if data.len() > snapshot_limit() {
        return Err(Error::Size);
    }
    let mut remaining = MAX_GENERATION_RESOURCE_BYTES;
    for input in data {
        remaining = remaining
            .checked_sub(input.bytes.len())
            .ok_or(Error::Size)?;
    }
    TrustedResources::new(
        data.iter()
            .map(|r| ResourcePin::from_bytes(r.resource, r.bytes))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::Snapshot)?,
    )
    .map_err(Error::Snapshot)
}
fn snapshot_limit() -> usize {
    crate::snapshot::MAX_RESOURCE_PINS
}

pub(super) fn approved(
    data: &[ResourceData<'_>],
    approved: &[&VerifiedCatalogRevision],
) -> Result<()> {
    if approved.len() > MAX_POLICY_ENTRIES {
        return Err(Error::Size);
    }
    let mut verified = BTreeMap::new();
    for item in approved {
        if verified.insert(item.revision(), *item).is_some() {
            return Err(Error::CatalogApproval(item.revision()));
        }
    }
    let mut used = 0;
    for input in data {
        if let Some(id) = revision(input.resource) {
            let item = verified.get(&id).ok_or(Error::CatalogApproval(id))?;
            if input.bytes != item.resource_bytes() {
                return Err(Error::CatalogApproval(id));
            }
            used += 1;
        }
    }
    if used != verified.len() {
        return Err(Error::Resource);
    }
    Ok(())
}
pub(super) fn decode(data: &[ResourceData<'_>]) -> Result<BTreeMap<CatalogRevision, Catalog>> {
    let mut remaining = MAX_POLICY_ENTRIES;
    for input in data {
        if revision(input.resource).is_some() {
            remaining = remaining.checked_sub(1).ok_or(Error::Size)?;
            decoded::preflight(input.bytes, &mut remaining).map_err(Error::Catalog)?;
        }
    }
    data.iter()
        .filter_map(|r| revision(r.resource).map(|id| (id, r)))
        .map(|(id, input)| {
            Ok((
                id,
                Catalog {
                    pin: CatalogPin {
                        revision: id,
                        hash: *crate::snapshot::ContentHash::of(input.bytes).as_bytes(),
                    },
                    data: decoded::decode(input.bytes, id).map_err(Error::Catalog)?,
                },
            ))
        })
        .collect()
}
pub(super) fn facts(
    data: &BTreeMap<CatalogRevision, Catalog>,
) -> Result<Vec<CatalogRevisionFacts>> {
    let mut remaining = MAX_POLICY_ENTRIES;
    for count in [data.len(), data.len()]
        .into_iter()
        .chain(data.values().map(|c| c.data.dependencies.len()))
    {
        remaining = remaining.checked_sub(count).ok_or(Error::Size)?;
    }
    // These facts are private derived values: CP required real approved objects;
    // DP entered only through verified/restored CP-signed content and actual hashes.
    Ok(data
        .iter()
        .map(|(id, c)| CatalogRevisionFacts {
            revision: *id,
            actual_hash: c.pin.hash,
            license: CatalogLicense::Approved,
            provenance: CatalogProvenance::Recorded,
            validation: CatalogValidationState::Valid,
            // Copy bounded manifest pins for the existing D39 validator, not source data.
            dependencies: c.data.dependencies.clone(),
        })
        .collect())
}
pub(super) fn get(
    data: &BTreeMap<CatalogRevision, Catalog>,
    id: CatalogRevision,
) -> Result<&Catalog> {
    data.get(&id).ok_or(Error::Resource)
}
