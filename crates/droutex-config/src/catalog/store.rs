use super::{
    CatalogFormat, CatalogImportError as Error, CatalogImportSummary, CatalogInput,
    CatalogLicenseApproval, PreparedRevision, VerifiedCatalogRevision, codec, scan,
};
use crate::snapshot::ContentHash;
use droutex_common::{MAX_GENERATION_RESOURCE_BYTES, MAX_LOCAL_INPUT_BYTES};
use droutex_policy::{
    CatalogRevision, MAX_POLICY_ENTRIES, blocklist_entry_count, parse_blocklist,
    validate_catalog_dependencies,
};
use std::{collections::BTreeMap, fmt};

/// Bounded immutable-revision registry. No automatic latest/active selection,
/// deletion/rebinding or Profile authority. Import prepares a complete union
/// before exclusive commit; every returned failure preserves all prior data.
#[derive(Default)]
pub struct CatalogStore {
    revisions: BTreeMap<CatalogRevision, VerifiedCatalogRevision>,
}
impl fmt::Debug for CatalogStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CatalogStore")
            .field("revisions", &self.revisions.len())
            .finish_non_exhaustive()
    }
}
impl CatalogStore {
    pub fn get(&self, revision: CatalogRevision) -> Option<&VerifiedCatalogRevision> {
        self.revisions.get(&revision)
    }
    pub fn revisions(&self) -> impl Iterator<Item = &VerifiedCatalogRevision> {
        self.revisions.values()
    }

    pub fn import_batch(
        &mut self,
        mut inputs: Vec<CatalogInput<'_>>,
        mut approvals: Vec<CatalogLicenseApproval>,
    ) -> Result<CatalogImportSummary, Error> {
        if inputs.is_empty() {
            return Err(Error::Empty);
        }
        if inputs.len() > MAX_POLICY_ENTRIES || approvals.len() > MAX_POLICY_ENTRIES {
            return Err(Error::Size);
        }
        inputs.sort_unstable_by_key(|i| i.revision);
        approvals.sort_unstable_by_key(|a| a.revision);
        let supplied_revisions = inputs.len();
        let raw_entries = preflight(&inputs, &approvals)?;
        let (candidates, canonical_entries) = self.prepare_candidates(inputs, approvals)?;
        self.validate_candidates(&candidates)?;
        // No fallible semantic work follows mutation; &mut excludes concurrent use.
        self.revisions.extend(
            candidates
                .into_iter()
                .map(|(id, revision)| (id, VerifiedCatalogRevision(revision))),
        );
        Ok(CatalogImportSummary {
            supplied_revisions,
            raw_entries,
            canonical_entries,
        })
    }

    fn prepare_candidates(
        &self,
        inputs: Vec<CatalogInput<'_>>,
        approvals: Vec<CatalogLicenseApproval>,
    ) -> Result<(BTreeMap<CatalogRevision, PreparedRevision>, usize), Error> {
        let mut candidates = BTreeMap::new();
        let mut output_remaining = MAX_GENERATION_RESOURCE_BYTES;
        let mut canonical_entries = 0;
        for (input, approved) in inputs.into_iter().zip(approvals) {
            let source_hash = ContentHash::of(input.bytes);
            if approved.source_hash != source_hash || approved.license != input.metadata.license {
                return Err(Error::License);
            }
            let (conditions, dependencies) = match input.format.list() {
                Some((format, mode)) => (
                    parse_blocklist(input.bytes, format, mode).map_err(Error::Input)?,
                    vec![],
                ),
                None => codec::service(input.bytes)?,
            };
            canonical_entries += conditions.entries().len();
            let mut candidate = PreparedRevision {
                revision: input.revision,
                format: input.format,
                metadata: input.metadata,
                source: input.bytes.to_vec(),
                source_hash,
                conditions,
                dependencies,
                resource: vec![],
                hash: ContentHash::of(&[]),
                approval_evidence: approved.evidence,
            };
            candidate.resource = codec::resource(&candidate, output_remaining)?;
            output_remaining = output_remaining
                .checked_sub(candidate.resource.len())
                .ok_or(Error::Size)?;
            candidate.hash = ContentHash::of(&candidate.resource);
            if let Some(old) = self.revisions.get(&candidate.revision) {
                if old.0.hash != candidate.hash {
                    return Err(Error::ImmutableRevision(candidate.revision));
                }
            } else {
                candidates.insert(candidate.revision, candidate);
            }
        }
        Ok((candidates, canonical_entries))
    }

    fn validate_candidates(
        &self,
        candidates: &BTreeMap<CatalogRevision, PreparedRevision>,
    ) -> Result<(), Error> {
        // Bound the complete retained set (including history needed for immutable
        // identity) and validate every dependency before exposing verified facts.
        let union = self
            .revisions
            .values()
            .map(|revision| &revision.0)
            .chain(candidates.values());
        let mut entries = MAX_POLICY_ENTRIES;
        let mut bytes = MAX_GENERATION_RESOURCE_BYTES;
        let mut resources = MAX_GENERATION_RESOURCE_BYTES;
        for revision in union.clone() {
            for count in [
                1,
                revision.conditions.entries().len(),
                revision.dependencies.len(),
            ] {
                entries = entries.checked_sub(count).ok_or(Error::Size)?;
            }
            let stored = revision
                .source
                .len()
                .checked_add(revision.metadata.bytes().ok_or(Error::Size)?)
                .and_then(|n| n.checked_add(revision.approval_evidence.len()))
                .ok_or(Error::Size)?;
            bytes = bytes.checked_sub(stored).ok_or(Error::Size)?;
            resources = resources
                .checked_sub(revision.resource.len())
                .ok_or(Error::Size)?;
        }
        let facts: Vec<_> = union.clone().map(PreparedRevision::facts).collect();
        let pins: Vec<_> = union.map(PreparedRevision::pin).collect();
        validate_catalog_dependencies(&pins, &facts).map_err(Error::Dependencies)?;
        Ok(())
    }
}
// Validation phases remain immutable; only import_batch owns the commit point.
fn preflight(
    inputs: &[CatalogInput<'_>],
    approvals: &[CatalogLicenseApproval],
) -> Result<usize, Error> {
    if let Some(pair) = inputs.windows(2).find(|p| p[0].revision == p[1].revision) {
        return Err(Error::DuplicateRevision(pair[0].revision));
    }
    if let Some(pair) = approvals
        .windows(2)
        .find(|p| p[0].revision == p[1].revision)
    {
        return Err(Error::DuplicateApproval(pair[0].revision));
    }
    for approval in approvals {
        if inputs
            .binary_search_by_key(&approval.revision, |i| i.revision)
            .is_err()
        {
            return Err(Error::UnknownApproval(approval.revision));
        }
    }
    // Preflight ALL raw bytes and entries before normalized collections/maps.
    let mut raw_remaining = MAX_POLICY_ENTRIES - inputs.len();
    let mut source_remaining = MAX_GENERATION_RESOURCE_BYTES;
    for input in inputs {
        let approval = approval(approvals, input.revision)?;
        let size = input
            .metadata
            .bytes()
            .and_then(|n| n.checked_add(input.bytes.len()))
            .and_then(|n| n.checked_add(approval.license.len()))
            .and_then(|n| n.checked_add(approval.evidence.len()))
            .ok_or(Error::Size)?;
        if size > MAX_LOCAL_INPUT_BYTES {
            return Err(Error::Size);
        }
        source_remaining = source_remaining.checked_sub(size).ok_or(Error::Size)?;
        if !input.metadata.valid() {
            return Err(Error::Shape);
        }
        match (input.revision, input.format) {
            (CatalogRevision::Filter(_), format) if format.list().is_some() => {
                let (format, _) = format.list().ok_or(Error::Shape)?;
                let count = blocklist_entry_count(input.bytes, format).map_err(Error::Input)?;
                raw_remaining = raw_remaining.checked_sub(count).ok_or(Error::Size)?;
            }
            (CatalogRevision::Service(_), CatalogFormat::ServiceJson) => {
                scan::json_entries(codec::json_bytes(input.bytes), &mut raw_remaining)?
            }
            _ => return Err(Error::Shape),
        }
    }
    Ok(MAX_POLICY_ENTRIES - inputs.len() - raw_remaining)
}

fn approval(
    approvals: &[CatalogLicenseApproval],
    revision: CatalogRevision,
) -> Result<&CatalogLicenseApproval, Error> {
    approvals
        .binary_search_by_key(&revision, |a| a.revision)
        .ok()
        .map(|i| &approvals[i])
        .ok_or(Error::License)
}
