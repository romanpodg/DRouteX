//! Charge the COMPLETE expansion before copying a domain/action into any table.
use super::{
    CompilationError as Error, Result,
    resources::{self, Catalog},
};
use crate::{Configuration, RuleAction};
use droutex_policy::{CatalogRevision, MAX_POLICY_ENTRIES};
use std::collections::BTreeMap;

fn charge(remaining: &mut usize, count: usize) -> Result<()> {
    *remaining = remaining.checked_sub(count).ok_or(Error::Size)?;
    Ok(())
}
pub(super) fn check(
    document: &Configuration,
    catalogs: &BTreeMap<CatalogRevision, Catalog>,
) -> Result<()> {
    let mut remaining = MAX_POLICY_ENTRIES;
    for count in [
        document.profiles.len(),
        document.endpoints.len(),
        document.network_groups.len(),
        document.private_records.len(),
        document.legacy_grants.len(),
        document.gateway_grants.len(),
    ] {
        charge(&mut remaining, count)?;
    }
    for profile in &document.profiles {
        for rule in &profile.custom_rules {
            let count = match rule.action {
                RuleAction::Dns(_) => 1,
                RuleAction::Traffic(_) => rule.ports.len().max(1),
            };
            charge(&mut remaining, count)?;
        }
        for rule in &profile.service_rules {
            charge(
                &mut remaining,
                resources::get(catalogs, CatalogRevision::Service(rule.revision))?
                    .data
                    .conditions
                    .entries()
                    .len(),
            )?;
        }
        for list in &profile.filter_lists {
            let conditions = resources::get(catalogs, CatalogRevision::Filter(list.revision))?
                .data
                .conditions
                .entries()
                .len();
            charge(
                &mut remaining,
                conditions.checked_mul(2).ok_or(Error::Size)?,
            )?;
        }
    }
    for grant in &document.gateway_grants {
        for count in [
            grant.targets.len(),
            grant.ports.len(),
            grant.outbounds.len(),
        ] {
            charge(&mut remaining, count)?;
        }
        for revision in &grant.services {
            charge(
                &mut remaining,
                resources::get(catalogs, CatalogRevision::Service(*revision))?
                    .data
                    .conditions
                    .entries()
                    .len(),
            )?;
        }
    }
    Ok(())
}
