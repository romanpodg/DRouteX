use super::graph;
use crate::MAX_POLICY_ENTRIES;
use droutex_common::{FilterListRevisionId, ServiceGroupRevisionId};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CatalogRevision {
    Filter(FilterListRevisionId),
    Service(ServiceGroupRevisionId),
}

/// Expected hash from a pinned manifest, not from the catalog's own claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CatalogPin {
    pub revision: CatalogRevision,
    pub hash: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogLicense {
    Approved,
    Unapproved,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogProvenance {
    Recorded,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogValidationState {
    Valid,
    Invalid,
    Unavailable,
}

/// Explicit caller-trusted facts about actual verified data. P2-07 owns import,
/// byte hashing, source/notice retention and license approval evidence. These
/// fields cannot authenticate themselves; an existence-only registry is insufficient.
/// No dataset license is approved by this type or by dependency license policy.
pub struct CatalogRevisionFacts {
    pub revision: CatalogRevision,
    pub actual_hash: [u8; 32],
    pub license: CatalogLicense,
    pub provenance: CatalogProvenance,
    pub validation: CatalogValidationState,
    pub dependencies: Vec<CatalogPin>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogIssue {
    DuplicateRevision,
    Missing,
    HashMismatch,
    License,
    Provenance,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogValidationError {
    EntryLimit,
    Revision {
        issue: CatalogIssue,
        revision: CatalogRevision,
    },
    Cycle {
        from: CatalogRevision,
        to: CatalogRevision,
    },
}
impl fmt::Display for CatalogValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("catalog dependencies are not valid for publication")
    }
}
impl Error for CatalogValidationError {}

fn error(issue: CatalogIssue, revision: CatalogRevision) -> CatalogValidationError {
    CatalogValidationError::Revision { issue, revision }
}

/// Validate the selected transitive closure. Unselected unavailable licenses do
/// not authorize or disable a selected revision. All supplied facts/edges/pins
/// share the existing aggregate bound, including duplicates and disabled inputs.
pub fn validate_catalog_dependencies(
    pins: &[CatalogPin],
    facts: &[CatalogRevisionFacts],
) -> Result<(), CatalogValidationError> {
    let mut remaining = MAX_POLICY_ENTRIES;
    for count in [pins.len(), facts.len()]
        .into_iter()
        .chain(facts.iter().map(|f| f.dependencies.len()))
    {
        remaining = remaining
            .checked_sub(count)
            .ok_or(CatalogValidationError::EntryLimit)?;
    }
    let mut inventory = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for fact in facts {
        if inventory.insert(fact.revision, fact).is_some() {
            duplicates.insert(fact.revision);
        }
    }
    if let Some(&id) = duplicates.first() {
        return Err(error(CatalogIssue::DuplicateRevision, id));
    }
    let mut pending: BTreeSet<_> = pins.iter().copied().collect();
    let mut edges = BTreeMap::new();
    while let Some(pin) = pending.pop_first() {
        let fact = inventory
            .get(&pin.revision)
            .ok_or_else(|| error(CatalogIssue::Missing, pin.revision))?;
        let issue = if fact.actual_hash != pin.hash {
            Some(CatalogIssue::HashMismatch)
        } else if fact.license != CatalogLicense::Approved {
            Some(CatalogIssue::License)
        } else if fact.provenance != CatalogProvenance::Recorded {
            Some(CatalogIssue::Provenance)
        } else if fact.validation != CatalogValidationState::Valid {
            Some(CatalogIssue::Invalid)
        } else {
            None
        };
        if let Some(issue) = issue {
            return Err(error(issue, pin.revision));
        }
        // Check every expected hash, even when this identity was already visited.
        if let std::collections::btree_map::Entry::Vacant(entry) = edges.entry(pin.revision) {
            entry.insert(fact.dependencies.iter().map(|p| p.revision).collect());
            pending.extend(fact.dependencies.iter().copied());
        }
    }
    if let Some((first, second)) = graph::cycle(&edges) {
        return Err(CatalogValidationError::Cycle {
            from: first,
            to: second,
        });
    }
    Ok(())
}
