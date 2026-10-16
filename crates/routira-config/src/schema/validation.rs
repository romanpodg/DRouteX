use super::{ConfigError, ConfigErrorCode as Code, Configuration, ReferenceCatalog, model::*};
use routira_common::{
    EndpointId, LegacyNetworkGrantId, ListenerId, NetworkGroupId, ProfileId, ProxyOutboundId,
    ResolverCredentialId, RuleId,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Tables<'a> {
    pub profiles: BTreeMap<ProfileId, &'a Profile>,
    pub endpoints: BTreeMap<EndpointId, &'a DnsEndpoint>,
    pub credentials: BTreeMap<ResolverCredentialId, &'a CredentialMetadata>,
    pub listeners: BTreeMap<ListenerId, &'a Listener>,
    pub groups: BTreeMap<NetworkGroupId, &'a NetworkGroup>,
    pub legacy: BTreeMap<LegacyNetworkGrantId, &'a LegacyGrant>,
    pub outbounds: BTreeMap<ProxyOutboundId, &'a ProxyOutbound>,
}

impl<'a> Tables<'a> {
    fn new(c: &'a Configuration) -> Result<Self, ConfigError> {
        Ok(Self {
            profiles: unique(&c.profiles, |p| p.id, "profiles")?,
            endpoints: unique(&c.endpoints, |p| p.id, "endpoints")?,
            credentials: unique(&c.resolver_credentials, |p| p.id, "resolverCredentials")?,
            listeners: unique(&c.listeners, |p| p.id, "listeners")?,
            groups: unique(&c.network_groups, |p| p.id, "networkGroups")?,
            legacy: unique(&c.legacy_grants, |p| p.id, "legacyGrants")?,
            outbounds: unique(&c.outbounds, |p| p.id, "outbounds")?,
        })
    }
}

pub(super) fn validate(c: &Configuration, catalog: &ReferenceCatalog) -> Result<(), ConfigError> {
    require(
        c.schema_version == 1,
        Code::UnsupportedVersion,
        "$.schemaVersion",
    )?;
    count_resources(c)?;
    let tables = Tables::new(c)?;
    unique(&c.gateway_grants, |g| g.id, "gatewayGrants")?;
    unique(&c.private_records, |r| r.id, "privateRecords")?;
    super::rules::validate_profiles(&c.profiles, &tables, catalog)?;
    super::access::validate_access(c, &tables, catalog)?;
    super::binding::validate_bindings(&c.listeners)?;
    validate_outbounds(&c.outbounds, catalog)?;
    validate_private_records(&c.private_records, &tables)?;
    Ok(())
}

fn count_resources(c: &Configuration) -> Result<(), ConfigError> {
    let counts = [
        c.profiles.len(),
        c.endpoints.len(),
        c.resolver_credentials.len(),
        c.listeners.len(),
        c.network_groups.len(),
        c.legacy_grants.len(),
        c.gateway_grants.len(),
        c.outbounds.len(),
        c.private_records.len(),
    ];
    let nested = c.profiles.iter().flat_map(|p| {
        [
            p.custom_rules.len(),
            p.service_rules.len(),
            p.filter_lists.len(),
        ]
    });
    let mut remaining = super::MAX_RESOURCE_ENTRIES;
    for count in counts.into_iter().chain(nested) {
        remaining = remaining
            .checked_sub(count)
            .ok_or_else(|| error(Code::ResourceLimit, "$"))?;
    }
    Ok(())
}

pub(super) fn unique<'a, T, K: Copy + Ord>(
    items: &'a [T],
    id: impl Fn(&T) -> K,
    collection: &'static str,
) -> Result<BTreeMap<K, &'a T>, ConfigError> {
    let mut entries = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        require(
            entries.insert(id(item), item).is_none(),
            Code::DuplicateId,
            &field(collection, index, "id"),
        )?;
    }
    Ok(entries)
}

pub(super) fn require(condition: bool, code: Code, field: &str) -> Result<(), ConfigError> {
    if condition {
        Ok(())
    } else {
        Err(error(code, field))
    }
}
pub(super) fn error(code: Code, field: &str) -> ConfigError {
    ConfigError::at(code, field.to_owned())
}
pub(super) fn field(collection: &'static str, index: usize, member: &'static str) -> String {
    format!("$.{collection}[{index}].{member}")
}
pub(super) fn named(name: &str, path: &str) -> Result<(), ConfigError> {
    require(!name.trim().is_empty(), Code::InvalidValue, path)
}
pub(super) fn ports(ports: &[PortRange], path: &str) -> Result<(), ConfigError> {
    for (index, range) in ports.iter().enumerate() {
        require(
            range.from != 0 && range.from <= range.to,
            Code::InvalidValue,
            &format!("{path}[{index}]"),
        )?;
    }
    Ok(())
}
pub(super) fn domain(pattern: &DomainMatch, path: &str) -> Result<(), ConfigError> {
    host_syntax(&pattern.value, &format!("{path}.value"))?;
    require(
        !pattern.value.contains('*'),
        Code::InvalidValue,
        &format!("{path}.value"),
    )
}

fn host_syntax(value: &str, path: &str) -> Result<(), ConfigError> {
    named(value, path)?;
    if value.parse::<std::net::IpAddr>().is_ok() {
        return Ok(());
    }
    require(
        !value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || ":/?#@\\[]".contains(c))
            && !value.contains("..")
            && !value.starts_with('.')
            && !value.trim_end_matches('.').is_empty(),
        Code::InvalidValue,
        path,
    )
}

fn validate_outbounds(
    outbounds: &[ProxyOutbound],
    catalog: &ReferenceCatalog,
) -> Result<(), ConfigError> {
    for (i, outbound) in outbounds.iter().enumerate() {
        named(&outbound.name, &field("outbounds", i, "name"))?;
        if let OutboundKind::HttpConnectTls {
            host,
            port,
            tls_server_name,
            secret,
        } = &outbound.transport
        {
            host_syntax(host, &field("outbounds", i, "transport.host"))?;
            host_syntax(
                tls_server_name,
                &field("outbounds", i, "transport.tlsServerName"),
            )?;
            require(
                *port != 0,
                Code::InvalidValue,
                &field("outbounds", i, "transport.port"),
            )?;
            if let Some(secret) = secret {
                require(
                    catalog.secrets.contains(secret),
                    Code::UnknownReference,
                    &field("outbounds", i, "transport.secret"),
                )?;
            }
        }
    }
    Ok(())
}

fn validate_private_records(
    records: &[PrivateRecord],
    tables: &Tables<'_>,
) -> Result<(), ConfigError> {
    for (i, record) in records.iter().enumerate() {
        require(
            tables.profiles.contains_key(&record.profile),
            Code::UnknownReference,
            &field("privateRecords", i, "profile"),
        )?;
        host_syntax(&record.name, &field("privateRecords", i, "name"))?;
        if let RecordData::Cname { target } = &record.record {
            host_syntax(target, &field("privateRecords", i, "record.target"))?;
        }
    }
    Ok(())
}

pub(super) fn rule_id(
    id: RuleId,
    seen: &mut BTreeSet<RuleId>,
    path: &str,
) -> Result<(), ConfigError> {
    require(seen.insert(id), Code::DuplicateId, path)
}
