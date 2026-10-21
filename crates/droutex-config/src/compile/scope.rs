use super::{
    CompilationError as Error, Result,
    resources::{self, Catalog},
};
use crate::Configuration;
use droutex_common::{
    GatewayNetworkGrantId, GatewayNetworkIdentity, LegacyDnsAssignment, LegacyNetworkGrantId,
    ListenerId, NetworkGroupId, ProxyOutboundId,
};
use droutex_policy::{CanonicalDomain, CatalogRevision, DomainMatcher};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    net::IpAddr,
    num::NonZeroU16,
};

/// Immutable desired target scope, not a live grant or checked destination address.
/// P4 must independently authenticate source/expiry/revocation and pin safe addresses.
pub struct GatewayScope {
    id: GatewayNetworkGrantId,
    group: NetworkGroupId,
    listener: ListenerId,
    source: IpAddr,
    enabled_listener: bool,
    targets: Vec<DomainMatcher>,
    ports: Vec<(u16, u16)>,
    outbounds: BTreeSet<ProxyOutboundId>,
}
impl GatewayScope {
    /// Pure static comparison only; true does not admit a connection.
    pub fn matches_static_scope(
        &self,
        identity: &GatewayNetworkIdentity,
        name: &CanonicalDomain,
        port: NonZeroU16,
        outbound: ProxyOutboundId,
    ) -> bool {
        self.matches_request_scope(identity, name, port) && self.outbounds.contains(&outbound)
    }
    /// Internal diagnostic request scope, without inferring a selected outbound.
    pub(crate) fn matches_request_scope(
        &self,
        identity: &GatewayNetworkIdentity,
        name: &CanonicalDomain,
        port: NonZeroU16,
    ) -> bool {
        self.enabled_listener
            && self.id == identity.grant
            && self.group == identity.group
            && self.listener == identity.listener
            && self.source == identity.source
            && self.targets.iter().any(|m| m.matches(name))
            && self
                .ports
                .iter()
                .any(|(first, last)| (*first..=*last).contains(&port.get()))
    }
}
impl fmt::Debug for GatewayScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GatewayScope")
            .field("targets", &self.targets.len())
            .field("ports", &self.ports.len())
            .field("outbounds", &self.outbounds.len())
            .finish_non_exhaustive()
    }
}
pub(super) fn build(
    document: &Configuration,
    catalogs: &BTreeMap<CatalogRevision, Catalog>,
) -> Result<(
    BTreeMap<LegacyNetworkGrantId, LegacyDnsAssignment>,
    BTreeMap<GatewayNetworkGrantId, GatewayScope>,
)> {
    let legacy = document
        .legacy_grants
        .iter()
        .map(|g| {
            (
                g.id,
                LegacyDnsAssignment {
                    grant: g.id,
                    group: g.group,
                    listener: g.listener,
                    source: g.source,
                    endpoint: g.endpoint,
                },
            )
        })
        .collect();
    let mut gateway = BTreeMap::new();
    for grant in &document.gateway_grants {
        let mut targets = grant
            .targets
            .iter()
            .map(|m| m.normalized().map_err(|_| Error::Domain))
            .collect::<Result<Vec<_>>>()?;
        for revision in &grant.services {
            let catalog = resources::get(catalogs, CatalogRevision::Service(*revision))?;
            // Precharged owned copies of only this revision's explicitly listed conditions.
            targets.extend(catalog.data.conditions.entries().iter().cloned());
        }
        gateway.insert(
            grant.id,
            GatewayScope {
                id: grant.id,
                group: grant.group,
                listener: grant.listener,
                source: grant.source,
                enabled_listener: document
                    .listeners
                    .iter()
                    .any(|l| l.id == grant.listener && l.enabled),
                targets,
                ports: grant.ports.iter().map(|p| (p.from, p.to)).collect(),
                outbounds: grant.outbounds.iter().copied().collect(),
            },
        );
    }
    Ok((legacy, gateway))
}
