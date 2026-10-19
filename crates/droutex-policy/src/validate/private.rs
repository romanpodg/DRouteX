use super::graph;
use crate::{CanonicalDomain, MAX_POLICY_ENTRIES};
use droutex_common::{PrivateDnsRecordId, ProfileId};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    net::{Ipv4Addr, Ipv6Addr},
};

#[derive(Eq, PartialEq)]
pub enum PrivateRecordValue {
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    Cname(CanonicalDomain),
}

/// Parsed validation input, not a DNS response or proxy permission. Current
/// Profile references are checked at the configuration boundary, separately.
pub struct PreparedPrivateRecord {
    pub id: PrivateDnsRecordId,
    pub profile: ProfileId,
    pub owner: CanonicalDomain,
    pub ttl: u32,
    pub value: PrivateRecordValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateRecordIssue {
    DuplicateId,
    CnameData,
    CnameTarget,
    Ttl,
    Cycle,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateRecordValidationError {
    EntryLimit,
    Record {
        issue: PrivateRecordIssue,
        profile: ProfileId,
        first: PrivateDnsRecordId,
        second: PrivateDnsRecordId,
    },
}
impl fmt::Display for PrivateRecordValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("private DNS records are not valid for publication")
    }
}
impl Error for PrivateRecordValidationError {}

fn error(
    issue: PrivateRecordIssue,
    a: &PreparedPrivateRecord,
    b: &PreparedPrivateRecord,
) -> PrivateRecordValidationError {
    PrivateRecordValidationError::Record {
        issue,
        profile: a.profile,
        first: a.id,
        second: b.id,
    }
}

#[derive(Default)]
struct Owner<'a> {
    first: Option<&'a PreparedPrivateRecord>,
    alias: Option<&'a PreparedPrivateRecord>,
    ipv4: Option<&'a PreparedPrivateRecord>,
    ipv6: Option<&'a PreparedPrivateRecord>,
}

/// Canonical Profile-local RRsets and alias graph; no external resolution or
/// cross-Profile lookup. TTL zero is valid. Distinct A/AAAA values are allowed.
pub fn validate_private_records(
    records: &[PreparedPrivateRecord],
) -> Result<(), PrivateRecordValidationError> {
    if records.len() > MAX_POLICY_ENTRIES {
        return Err(PrivateRecordValidationError::EntryLimit);
    }
    let mut ordered: Vec<_> = records.iter().collect();
    ordered.sort_by_key(|r| (r.profile, &r.owner, r.id));
    let mut ids = BTreeSet::new();
    let mut owners: BTreeMap<_, Owner<'_>> = BTreeMap::new();
    for record in ordered {
        if !ids.insert(record.id) {
            return Err(error(PrivateRecordIssue::DuplicateId, record, record));
        }
        let owner = owners.entry((record.profile, &record.owner)).or_default();
        if let Some(first) = owner.first
            && matches!(first.value, PrivateRecordValue::Cname(_))
                != matches!(record.value, PrivateRecordValue::Cname(_))
        {
            return Err(error(PrivateRecordIssue::CnameData, first, record));
        }
        owner.first.get_or_insert(record);
        let previous = match &record.value {
            PrivateRecordValue::Cname(_) => &mut owner.alias,
            PrivateRecordValue::Ipv4(_) => &mut owner.ipv4,
            PrivateRecordValue::Ipv6(_) => &mut owner.ipv6,
        };
        if let Some(first) = previous {
            if matches!(record.value, PrivateRecordValue::Cname(_)) && first.value != record.value {
                return Err(error(PrivateRecordIssue::CnameTarget, first, record));
            }
            if first.ttl != record.ttl {
                return Err(error(PrivateRecordIssue::Ttl, first, record));
            }
        } else {
            *previous = Some(record);
        }
    }
    let mut edges = BTreeMap::new();
    for (&key, owner) in &owners {
        let mut next = BTreeSet::new();
        if let Some(record) = owner.alias
            && let PrivateRecordValue::Cname(target) = &record.value
            && owners.contains_key(&(record.profile, target))
        {
            next.insert((record.profile, target));
        }
        edges.insert(key, next);
    }
    if let Some((a, b)) = graph::cycle(&edges)
        && let Some(first) = owners[&a].first
        && let Some(second) = owners[&b].first
    {
        return Err(error(PrivateRecordIssue::Cycle, first, second));
    }
    Ok(())
}
