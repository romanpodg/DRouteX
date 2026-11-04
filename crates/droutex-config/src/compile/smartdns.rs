//! Check explicit DNS routes against the complete same-generation policy.
use super::{CompilationError, GatewayScope, Result};
use crate::{Configuration, DnsAction, TrafficAction};
use droutex_common::{DnsProfileBinding, GatewayNetworkGrantId, GatewayNetworkIdentity};
use droutex_policy::{CanonicalDomain, PolicySet};
use std::{collections::BTreeMap, num::NonZeroU16};

pub(super) fn validate(
    document: &Configuration,
    policy: &PolicySet<DnsAction, TrafficAction>,
    scopes: &BTreeMap<GatewayNetworkGrantId, GatewayScope>,
) -> Result<()> {
    let endpoints: BTreeMap<_, _> = document.endpoints.iter().map(|e| (e.id, e)).collect();
    let grants: BTreeMap<_, _> = document.gateway_grants.iter().map(|g| (g.id, g)).collect();
    for route in &document.smart_dns_routes {
        let endpoint = endpoints
            .get(&route.endpoint)
            .ok_or(CompilationError::Domain)?;
        let grant = grants.get(&route.grant).ok_or(CompilationError::Domain)?;
        let domain = route
            .domain
            .parse::<CanonicalDomain>()
            .map_err(|_| CompilationError::Domain)?;
        let port = NonZeroU16::new(route.port).ok_or(CompilationError::Domain)?;
        let identity = GatewayNetworkIdentity {
            grant: grant.id,
            group: grant.group,
            listener: grant.listener,
            source: grant.source,
        };
        if !scopes.get(&grant.id).is_some_and(|scope| {
            scope.matches_static_scope(&identity, &domain, port, route.outbound)
        }) {
            return Err(CompilationError::Domain);
        }
        let dns = policy
            .evaluate_dns(
                &DnsProfileBinding {
                    endpoint: endpoint.id,
                    profile: endpoint.profile,
                },
                &domain,
            )
            .map_err(|_| CompilationError::Domain)?;
        let tcp = policy
            .evaluate_tcp(&identity, &domain, port)
            .map_err(|_| CompilationError::Domain)?;
        if !matches!(dns.action(), DnsAction::Gateway { outbound } if *outbound == route.outbound)
            || !matches!(tcp.action(), TrafficAction::Outbound { outbound } if *outbound == route.outbound)
        {
            return Err(CompilationError::Domain);
        }
    }
    Ok(())
}
