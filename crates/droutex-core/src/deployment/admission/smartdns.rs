//! Current DNS route evidence, distinct from TCP admission and gateway readiness.
use super::{
    AdmissionAttempt, AdmissionCode, AdmissionError, AdmissionScope, AuthorizedRequest, Deployment,
    GrantLease, Result, io,
};
use droutex_common::{ListenerId, ProxyOutboundId};
use droutex_config::{OutboundObservation, SmartDnsRoute};
use std::{fmt, net::IpAddr, num::NonZeroU16};

impl Deployment {
    /// SmartDNS TCP ingress must use this gate instead of the general gateway
    /// gate. A DNS answer or earlier query is never used as authorization.
    pub fn prepare_smartdns_gateway(
        &self,
        listener: ListenerId,
        peer: IpAddr,
        domain: &str,
        port: NonZeroU16,
        observations: Vec<OutboundObservation>,
    ) -> Result<AdmissionAttempt<'_>> {
        let attempt = self.prepare_gateway(listener, peer, domain, port, observations)?;
        let AdmissionScope::Gateway(candidate) = &attempt.scope else {
            return Err(AdmissionError(AdmissionCode::Scope));
        };
        let doc = attempt
            .reservation
            .decision
            .gateway_configuration()
            .document();
        if !doc.smart_dns_routes.iter().any(|route| {
            route.grant == candidate.identity().grant
                && route.port == port.get()
                && route.outbound == candidate.outbound()
                && route.matches_domain(candidate.domain().as_str())
        }) {
            return Err(AdmissionError(AdmissionCode::Scope));
        }
        Ok(attempt)
    }
}

/// Borrowed route metadata and a live two-lease check. This is not gateway
/// readiness, TCP admission, or permission to connect or emit a DNS response.
/// A future DNS transport must check readiness and recheck this binding at every
/// output submission. The separately admitted gateway checks each connection.
pub struct SmartDnsBinding<'request, 'store> {
    owner: &'request AuthorizedRequest<'store>,
    route: &'request SmartDnsRoute,
    gateway_lease: GrantLease,
}
impl fmt::Debug for SmartDnsBinding<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmartDnsBinding").finish_non_exhaustive()
    }
}
impl SmartDnsBinding<'_, '_> {
    /// Static addresses are metadata, not independently authorized output.
    pub fn addresses(&self) -> &[IpAddr] {
        &self.route.addresses
    }
    pub fn ttl(&self) -> u32 {
        self.route.ttl
    }
    /// Recheck both leases under one clock sample and the same current generation.
    pub fn verify_current(&self) -> Result<()> {
        let reservation = &self.owner.attempt.reservation;
        let owner = reservation.decision._owner;
        io::check_current_leases(
            &owner.published,
            &owner.admission,
            &reservation.decision.generation,
            &[reservation.lease, self.gateway_lease],
        )
    }
}
impl<'store> AuthorizedRequest<'store> {
    /// Find an exact binding already checked during same-generation compilation.
    /// The DNS endpoint does not identify, select or authorize a TCP client.
    pub fn smart_dns_binding<'request>(
        &'request self,
        domain: &str,
        outbound: ProxyOutboundId,
    ) -> Result<SmartDnsBinding<'request, 'store>> {
        let AdmissionScope::LegacyDns(dns) = self.scope() else {
            return Err(AdmissionError(AdmissionCode::Scope));
        };
        let document = self.decision().dns_configuration().document();
        let route = document
            .smart_dns_routes
            .iter()
            .find(|route| {
                route.endpoint == dns.endpoint
                    && route.outbound == outbound
                    && route.matches_domain(domain)
            })
            .ok_or(AdmissionError(AdmissionCode::Scope))?;
        let lease = document
            .gateway_grants
            .iter()
            .find(|grant| grant.id == route.grant)
            .and_then(|grant| grant.lease)
            .ok_or(AdmissionError(AdmissionCode::Scope))?;
        let binding = SmartDnsBinding {
            owner: self,
            route,
            gateway_lease: lease,
        };
        binding.verify_current()?;
        Ok(binding)
    }
}
