use super::{ConfigError, ConfigErrorCode as Code, model::*, validation::*};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_access(
    c: &Configuration,
    tables: &Tables<'_>,
    catalog: &ReferenceCatalog,
) -> Result<(), ConfigError> {
    for (i, group) in c.network_groups.iter().enumerate() {
        named(&group.name, &field("networkGroups", i, "name"))?;
        require(
            tables.profiles.contains_key(&group.traffic_profile),
            Code::UnknownReference,
            &field("networkGroups", i, "trafficProfile"),
        )?;
    }
    for (i, credential) in c.resolver_credentials.iter().enumerate() {
        require(
            tables.endpoints.contains_key(&credential.endpoint),
            Code::UnknownReference,
            &field("resolverCredentials", i, "endpoint"),
        )?;
    }
    for (i, endpoint) in c.endpoints.iter().enumerate() {
        validate_endpoint(endpoint, i, tables, catalog)?;
    }
    legacy_grants(&c.legacy_grants, tables)?;
    gateway_grants(&c.gateway_grants, tables, catalog)
}

fn validate_endpoint(
    endpoint: &DnsEndpoint,
    i: usize,
    tables: &Tables<'_>,
    catalog: &ReferenceCatalog,
) -> Result<(), ConfigError> {
    named(&endpoint.name, &field("endpoints", i, "name"))?;
    require(
        tables.profiles.contains_key(&endpoint.profile),
        Code::UnknownReference,
        &field("endpoints", i, "profile"),
    )?;
    if let Some(device) = endpoint.device {
        require(
            catalog.devices.contains(&device),
            Code::UnknownReference,
            &field("endpoints", i, "device"),
        )?;
    }
    require(
        !endpoint.enabled || !endpoint.access.is_empty(),
        Code::InvalidScope,
        &field("endpoints", i, "access"),
    )?;
    let mut bindings = BTreeSet::new();
    for (j, access) in endpoint.access.iter().enumerate() {
        let path = format!("$.endpoints[{i}].access[{j}]");
        let (id, transport) = access_listener(access);
        let listener = tables
            .listeners
            .get(&id)
            .ok_or_else(|| error(Code::UnknownReference, &format!("{path}.listener")))?;
        require(
            listener.transport == transport,
            Code::InvalidScope,
            &format!("{path}.listener"),
        )?;
        require(bindings.insert((id, transport)), Code::DuplicateId, &path)?;
        match access {
            DnsAccess::Doh { credentials, .. } | DnsAccess::Dot { credentials, .. } => {
                secure(endpoint, credentials, tables, &path)?
            }
            DnsAccess::Udp { grants, .. } | DnsAccess::Tcp { grants, .. } => {
                legacy(endpoint, id, grants, tables, &path)?
            }
        }
    }
    Ok(())
}

fn access_listener(access: &DnsAccess) -> (droutex_common::ListenerId, ListenerTransport) {
    match access {
        DnsAccess::Doh { listener, .. } => (*listener, ListenerTransport::Doh),
        DnsAccess::Dot { listener, .. } => (*listener, ListenerTransport::Dot),
        DnsAccess::Udp { listener, .. } => (*listener, ListenerTransport::DnsUdp),
        DnsAccess::Tcp { listener, .. } => (*listener, ListenerTransport::DnsTcp),
    }
}

fn secure(
    endpoint: &DnsEndpoint,
    ids: &[droutex_common::ResolverCredentialId],
    tables: &Tables<'_>,
    path: &str,
) -> Result<(), ConfigError> {
    let path = format!("{path}.credentials");
    require(!ids.is_empty(), Code::InvalidScope, &path)?;
    let mut seen = BTreeSet::new();
    let mut active = false;
    for (i, id) in ids.iter().enumerate() {
        let field = format!("{path}[{i}]");
        require(seen.insert(id), Code::DuplicateId, &field)?;
        let credential = tables
            .credentials
            .get(id)
            .ok_or_else(|| error(Code::UnknownReference, &field))?;
        require(
            credential.endpoint == endpoint.id,
            Code::InvalidScope,
            &field,
        )?;
        active |= credential.state == CredentialState::Active;
    }
    require(!endpoint.enabled || active, Code::InvalidScope, &path)
}

fn legacy(
    endpoint: &DnsEndpoint,
    listener: droutex_common::ListenerId,
    ids: &[droutex_common::LegacyNetworkGrantId],
    tables: &Tables<'_>,
    path: &str,
) -> Result<(), ConfigError> {
    let path = format!("{path}.grants");
    require(!ids.is_empty(), Code::InvalidScope, &path)?;
    let mut seen = BTreeSet::new();
    for (i, id) in ids.iter().enumerate() {
        let field = format!("{path}[{i}]");
        require(seen.insert(id), Code::DuplicateId, &field)?;
        let grant = tables
            .legacy
            .get(id)
            .ok_or_else(|| error(Code::UnknownReference, &field))?;
        require(
            grant.endpoint == endpoint.id && grant.listener == listener,
            Code::InvalidScope,
            &field,
        )?;
    }
    Ok(())
}

fn legacy_grants(grants: &[LegacyGrant], tables: &Tables<'_>) -> Result<(), ConfigError> {
    let mut assignments = BTreeSet::new();
    for (i, grant) in grants.iter().enumerate() {
        let path = format!("$.legacyGrants[{i}]");
        let listener = tables
            .listeners
            .get(&grant.listener)
            .ok_or_else(|| error(Code::UnknownReference, &format!("{path}.listener")))?;
        require(
            matches!(
                listener.transport,
                ListenerTransport::DnsUdp | ListenerTransport::DnsTcp
            ),
            Code::InvalidScope,
            &format!("{path}.listener"),
        )?;
        require(
            tables.endpoints.contains_key(&grant.endpoint),
            Code::UnknownReference,
            &format!("{path}.endpoint"),
        )?;
        require(
            tables.groups.contains_key(&grant.group),
            Code::UnknownReference,
            &format!("{path}.group"),
        )?;
        require(
            assignments.insert((grant.source, grant.listener)),
            Code::AmbiguousGrant,
            &path,
        )?;
    }
    Ok(())
}

fn gateway_grants(
    grants: &[GatewayGrant],
    tables: &Tables<'_>,
    catalog: &ReferenceCatalog,
) -> Result<(), ConfigError> {
    let mut assignments = BTreeMap::new();
    for (i, grant) in grants.iter().enumerate() {
        let path = format!("$.gatewayGrants[{i}]");
        let listener = tables
            .listeners
            .get(&grant.listener)
            .ok_or_else(|| error(Code::UnknownReference, &format!("{path}.listener")))?;
        require(
            listener.transport == ListenerTransport::GatewayTcp,
            Code::InvalidScope,
            &format!("{path}.listener"),
        )?;
        require(
            tables.groups.contains_key(&grant.group),
            Code::UnknownReference,
            &format!("{path}.group"),
        )?;
        let previous = assignments.insert((grant.source, grant.listener), grant.group);
        require(
            previous.is_none_or(|group| group == grant.group),
            Code::AmbiguousGrant,
            &path,
        )?;
        gateway_scope(grant, tables, catalog, &path)?;
    }
    Ok(())
}

fn gateway_scope(
    grant: &GatewayGrant,
    tables: &Tables<'_>,
    catalog: &ReferenceCatalog,
    path: &str,
) -> Result<(), ConfigError> {
    require(
        !grant.targets.is_empty() || !grant.services.is_empty(),
        Code::InvalidScope,
        &format!("{path}.targets"),
    )?;
    require(
        !grant.ports.is_empty(),
        Code::InvalidScope,
        &format!("{path}.ports"),
    )?;
    ports(&grant.ports, &format!("{path}.ports"))?;
    require(
        !grant.outbounds.is_empty(),
        Code::InvalidScope,
        &format!("{path}.outbounds"),
    )?;
    for (i, target) in grant.targets.iter().enumerate() {
        domain(target, &format!("{path}.targets[{i}]"))?;
    }
    for (i, service) in grant.services.iter().enumerate() {
        require(
            catalog.service_revisions.contains(service),
            Code::UnknownReference,
            &format!("{path}.services[{i}]"),
        )?;
    }
    for (i, outbound) in grant.outbounds.iter().enumerate() {
        require(
            tables.outbounds.contains_key(outbound),
            Code::UnknownReference,
            &format!("{path}.outbounds[{i}]"),
        )?;
    }
    Ok(())
}
