use super::{ConfigError, ConfigErrorCode as Code, model::*, validation::*};
use routira_common::RuleId;
use std::collections::BTreeSet;

pub(super) fn validate_profiles(
    profiles: &[Profile],
    tables: &Tables<'_>,
    catalog: &ReferenceCatalog,
) -> Result<(), ConfigError> {
    let mut ids: BTreeSet<RuleId> = BTreeSet::new();
    for (i, profile) in profiles.iter().enumerate() {
        let path = format!("$.profiles[{i}]");
        named(&profile.name, &format!("{path}.name"))?;
        dns_action(
            &profile.default_dns_action,
            tables,
            catalog,
            &format!("{path}.defaultDnsAction"),
        )?;
        traffic_action(
            &profile.default_traffic_action,
            tables,
            &format!("{path}.defaultTrafficAction"),
        )?;
        custom_rules(&profile.custom_rules, tables, catalog, &mut ids, &path)?;
        service_rules(&profile.service_rules, tables, catalog, &mut ids, &path)?;
        filters(&profile.filter_lists, catalog, &path)?;
    }
    Ok(())
}

fn custom_rules(
    rules: &[CustomRule],
    tables: &Tables<'_>,
    catalog: &ReferenceCatalog,
    seen: &mut BTreeSet<RuleId>,
    profile: &str,
) -> Result<(), ConfigError> {
    for (i, rule) in rules.iter().enumerate() {
        let path = format!("{profile}.customRules[{i}]");
        rule_id(rule.id, seen, &format!("{path}.id"))?;
        domain(&rule.matcher, &format!("{path}.matcher"))?;
        ports(&rule.ports, &format!("{path}.ports"))?;
        require(
            !matches!(rule.action, RuleAction::Dns(_)) || rule.ports.is_empty(),
            Code::InvalidScope,
            &format!("{path}.ports"),
        )?;
        rule_action(&rule.action, tables, catalog, &format!("{path}.action"))?;
    }
    Ok(())
}

fn service_rules(
    rules: &[ServiceRule],
    tables: &Tables<'_>,
    catalog: &ReferenceCatalog,
    seen: &mut BTreeSet<RuleId>,
    profile: &str,
) -> Result<(), ConfigError> {
    for (i, rule) in rules.iter().enumerate() {
        let path = format!("{profile}.serviceRules[{i}]");
        rule_id(rule.id, seen, &format!("{path}.id"))?;
        require(
            catalog.service_revisions.contains(&rule.revision),
            Code::UnknownReference,
            &format!("{path}.revision"),
        )?;
        rule_action(&rule.action, tables, catalog, &format!("{path}.action"))?;
    }
    Ok(())
}

fn filters(
    filters: &[FilterSelection],
    catalog: &ReferenceCatalog,
    profile: &str,
) -> Result<(), ConfigError> {
    let mut revisions = BTreeSet::new();
    for (i, filter) in filters.iter().enumerate() {
        let path = format!("{profile}.filterLists[{i}].revision");
        require(revisions.insert(filter.revision), Code::DuplicateId, &path)?;
        require(
            catalog.filter_revisions.contains(&filter.revision),
            Code::UnknownReference,
            &path,
        )?;
    }
    Ok(())
}

fn rule_action(
    action: &RuleAction,
    tables: &Tables<'_>,
    catalog: &ReferenceCatalog,
    path: &str,
) -> Result<(), ConfigError> {
    let path = format!("{path}.action");
    match action {
        RuleAction::Dns(action) => dns_action(action, tables, catalog, &path),
        RuleAction::Traffic(action) => traffic_action(action, tables, &path),
    }
}

fn dns_action(
    action: &DnsAction,
    tables: &Tables<'_>,
    catalog: &ReferenceCatalog,
    path: &str,
) -> Result<(), ConfigError> {
    match action {
        DnsAction::Resolve { upstream } => require(
            catalog.upstreams.contains(upstream),
            Code::UnknownReference,
            &format!("{path}.upstream"),
        ),
        DnsAction::Gateway { outbound } => require(
            tables.outbounds.contains_key(outbound),
            Code::UnknownReference,
            &format!("{path}.outbound"),
        ),
        DnsAction::Block {} => Ok(()),
    }
}

fn traffic_action(
    action: &TrafficAction,
    tables: &Tables<'_>,
    path: &str,
) -> Result<(), ConfigError> {
    match action {
        TrafficAction::Outbound { outbound } => require(
            tables.outbounds.contains_key(outbound),
            Code::UnknownReference,
            &format!("{path}.outbound"),
        ),
        TrafficAction::Deny {} => Ok(()),
    }
}
