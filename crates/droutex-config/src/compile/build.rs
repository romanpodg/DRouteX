use super::{
    CompilationError as Error, Result,
    resources::{self, Catalog},
};
use crate::{Configuration, DnsAction, RecordData, RuleAction, TrafficAction};
use droutex_common::{AdministrativeState, DnsEndpointAssignment};
use droutex_policy::{
    CatalogRevision, DomainMatcher, DomainRule, GatewayProfileAssignment, PolicySet,
    PreparedPrivateRecord, PreparedProfile, PrivateRecordValue, RuleMetadata, RuleSource,
    TcpPortRange, TcpPortScope, TcpRule,
};
use std::collections::BTreeMap;

fn state(enabled: bool) -> AdministrativeState {
    if enabled {
        AdministrativeState::Enabled
    } else {
        AdministrativeState::Disabled
    }
}
fn add(
    profile: &mut PreparedProfile<DnsAction, TrafficAction>,
    metadata: RuleMetadata,
    matcher: DomainMatcher,
    action: &RuleAction,
    ports: &[crate::PortRange],
) -> Result<()> {
    match action {
        RuleAction::Dns(action) => profile
            .dns_rules
            .push(DomainRule::new(metadata, matcher, *action)),
        RuleAction::Traffic(action) => {
            if ports.is_empty() {
                profile.tcp_rules.push(TcpRule::new(
                    DomainRule::new(metadata, matcher, *action),
                    TcpPortScope::Any,
                ));
            } else {
                for port in ports {
                    // Each bounded copy represents a distinct OR interval, precharged by budget.
                    profile.tcp_rules.push(TcpRule::new(
                        DomainRule::new(metadata, matcher.clone(), *action),
                        TcpPortScope::Range(
                            TcpPortRange::new(port.from, port.to).map_err(Error::Policy)?,
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}
pub(super) fn policy(
    document: &Configuration,
    catalogs: &BTreeMap<CatalogRevision, Catalog>,
) -> Result<PolicySet<DnsAction, TrafficAction>> {
    let mut profiles = Vec::with_capacity(document.profiles.len());
    for input in &document.profiles {
        let mut profile = PreparedProfile {
            id: input.id,
            dns_default: input.default_dns_action,
            dns_rules: vec![],
            tcp_default: input.default_traffic_action,
            tcp_rules: vec![],
        };
        for rule in &input.custom_rules {
            add(
                &mut profile,
                RuleMetadata {
                    source: RuleSource::Custom(rule.id),
                    state: state(rule.enabled),
                    priority: rule.priority,
                },
                rule.matcher.normalized().map_err(|_| Error::Domain)?,
                &rule.action,
                &rule.ports,
            )?;
        }
        for rule in &input.service_rules {
            let catalog = resources::get(catalogs, CatalogRevision::Service(rule.revision))?;
            // Dependencies validate availability/hash/license only; never inherit domains/actions.
            for condition in catalog.data.conditions.entries() {
                add(
                    &mut profile,
                    RuleMetadata {
                        source: RuleSource::Service {
                            rule: rule.id,
                            revision: rule.revision,
                        },
                        state: state(rule.enabled),
                        priority: rule.priority,
                    },
                    condition.clone(),
                    &rule.action,
                    &[],
                )?;
            }
        }
        for list in &input.filter_lists {
            let catalog = resources::get(catalogs, CatalogRevision::Filter(list.revision))?;
            for condition in catalog.data.conditions.entries() {
                let metadata = RuleMetadata {
                    source: RuleSource::Filter(list.revision),
                    state: state(list.enabled),
                    priority: list.priority,
                };
                // Independent Profile planes; DNS selection never establishes a gateway identity.
                profile.dns_rules.push(DomainRule::new(
                    metadata,
                    condition.clone(),
                    DnsAction::Block {},
                ));
                profile.tcp_rules.push(TcpRule::new(
                    DomainRule::new(metadata, condition.clone(), TrafficAction::Deny {}),
                    TcpPortScope::Any,
                ));
            }
        }
        profiles.push(profile);
    }
    let endpoints = document
        .endpoints
        .iter()
        .map(|e| DnsEndpointAssignment {
            endpoint: e.id,
            profile: e.profile,
            state: state(e.enabled),
            device: e.device,
        })
        .collect();
    let groups = document
        .network_groups
        .iter()
        .map(|g| GatewayProfileAssignment {
            group: g.id,
            profile: g.traffic_profile,
        })
        .collect();
    PolicySet::new(profiles, endpoints, groups).map_err(Error::Policy)
}
pub(super) fn private(document: &Configuration) -> Result<Vec<PreparedPrivateRecord>> {
    let mut result = document
        .private_records
        .iter()
        .map(|record| {
            Ok(PreparedPrivateRecord {
                id: record.id,
                profile: record.profile,
                owner: record.name.parse().map_err(|_| Error::Domain)?,
                ttl: record.ttl,
                value: match &record.record {
                    RecordData::A { address } => PrivateRecordValue::Ipv4(*address),
                    RecordData::Aaaa { address } => PrivateRecordValue::Ipv6(*address),
                    RecordData::Cname { target } => {
                        PrivateRecordValue::Cname(target.parse().map_err(|_| Error::Domain)?)
                    }
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;
    result.sort_by(|a, b| (a.profile, &a.owner, a.id).cmp(&(b.profile, &b.owner, b.id)));
    Ok(result)
}
