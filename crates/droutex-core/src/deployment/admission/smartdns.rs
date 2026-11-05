//! Current DNS route evidence, distinct from TCP admission and gateway readiness.
use super::{
    AdmissionAttempt, AdmissionCode, AdmissionError, AdmissionScope, AuthorizedRequest, Deployment,
    GrantLease, Reservation, Result, io,
};
use droutex_common::{GatewayNetworkIdentity, ListenerId, ProxyOutboundId};
use droutex_config::{Configuration, OutboundObservation, SmartDnsRoute};
use std::{fmt, net::IpAddr, num::NonZeroU16};

fn unique_route_port(
    document: &Configuration,
    listener: ListenerId,
    peer: IpAddr,
    domain: &str,
) -> Result<NonZeroU16> {
    let grant = document
        .gateway_grants
        .iter()
        .find(|grant| grant.listener == listener && grant.source == peer)
        .ok_or(AdmissionError(AdmissionCode::Scope))?;
    let mut selected = None;
    for route in document
        .smart_dns_routes
        .iter()
        .filter(|route| route.grant == grant.id && route.matches_domain(domain))
    {
        let port = NonZeroU16::new(route.port).ok_or(AdmissionError(AdmissionCode::Scope))?;
        if selected.is_some_and(|old| old != port) {
            return Err(AdmissionError(AdmissionCode::Scope));
        }
        selected = Some(port);
    }
    selected.ok_or(AdmissionError(AdmissionCode::Scope))
}

impl Deployment {
    /// Reserve the actual gateway peer and its quota before reading ClientHello.
    /// This does not authorize any destination or allow an origin connection.
    pub fn reserve_smartdns_gateway_ingress(
        &self,
        listener: ListenerId,
        peer: IpAddr,
    ) -> Result<SmartDnsIngress<'_>> {
        let (reservation, identity) = self.reserve_gateway(listener, peer)?;
        Ok(SmartDnsIngress {
            reservation,
            identity,
        })
    }
    /// Convenience for callers that have already bounded ClientHello processing.
    pub fn prepare_smartdns_gateway_for_name(
        &self,
        listener: ListenerId,
        peer: IpAddr,
        domain: &str,
        observations: Vec<OutboundObservation>,
    ) -> Result<AdmissionAttempt<'_>> {
        self.reserve_smartdns_gateway_ingress(listener, peer)?
            .prepare_for_name(domain, observations)
    }
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

/// One bounded ingress reservation, pinned to its original generation.
/// It cannot authorize or select a destination without `prepare_for_name`.
pub struct SmartDnsIngress<'store> {
    reservation: Reservation<'store>,
    identity: GatewayNetworkIdentity,
}
impl fmt::Debug for SmartDnsIngress<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmartDnsIngress").finish_non_exhaustive()
    }
}
impl<'store> SmartDnsIngress<'store> {
    /// Select a single signed port, then validate the candidate against the
    /// same reservation. Consuming the attempt rechecks current authority.
    pub fn prepare_for_name(
        self,
        domain: &str,
        observations: Vec<OutboundObservation>,
    ) -> Result<AdmissionAttempt<'store>> {
        let document = self.reservation.decision.gateway_configuration().document();
        let port = unique_route_port(
            document,
            self.identity.listener,
            self.identity.source,
            domain,
        )?;
        let attempt = Deployment::finish_gateway(
            self.reservation,
            self.identity,
            domain,
            port,
            observations,
        )?;
        let AdmissionScope::Gateway(candidate) = &attempt.scope else {
            return Err(AdmissionError(AdmissionCode::Scope));
        };
        let document = attempt
            .reservation
            .decision
            .gateway_configuration()
            .document();
        if !document.smart_dns_routes.iter().any(|route| {
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
    listener: ListenerId,
}
impl fmt::Debug for SmartDnsBinding<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmartDnsBinding").finish_non_exhaustive()
    }
}
impl SmartDnsBinding<'_, '_> {
    /// The exact listener required by this signed route.
    pub fn listener(&self) -> ListenerId {
        self.listener
    }
    pub fn outbound(&self) -> ProxyOutboundId {
        self.route.outbound
    }
    /// Port a client uses to connect to the advertised gateway address.
    pub fn port(&self) -> u16 {
        self.route.port
    }
    /// Checks identity, not merely equivalent configuration from another owner.
    pub fn belongs_to(&self, deployment: &Deployment) -> bool {
        std::ptr::eq(self.owner.attempt.reservation.decision._owner, deployment)
    }
    pub fn metadata(&self) -> droutex_config::snapshot::SnapshotMetadata {
        self.owner.decision().metadata()
    }
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
impl AuthorizedRequest<'_> {
    /// Recheck the exact SmartDNS route for an existing gateway admission.
    /// This does not validate a socket or authorize an unchecked origin address.
    pub fn verify_smartdns_gateway_destination(
        &self,
        domain: &str,
        port: NonZeroU16,
    ) -> Result<()> {
        let AdmissionScope::Gateway(candidate) = self.scope() else {
            return Err(AdmissionError(AdmissionCode::Scope));
        };
        let document = self.decision().gateway_configuration().document();
        if !document.smart_dns_routes.iter().any(|route| {
            route.grant == candidate.identity().grant
                && route.port == port.get()
                && route.outbound == candidate.outbound()
                && route.matches_domain(candidate.domain().as_str())
        }) {
            return Err(AdmissionError(AdmissionCode::Scope));
        }
        self.verify_gateway_destination(domain, port)
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
        let grant = document
            .gateway_grants
            .iter()
            .find(|grant| grant.id == route.grant)
            .ok_or(AdmissionError(AdmissionCode::Scope))?;
        let lease = grant.lease.ok_or(AdmissionError(AdmissionCode::Scope))?;
        let binding = SmartDnsBinding {
            owner: self,
            route,
            gateway_lease: lease,
            listener: grant.listener,
        };
        binding.verify_current()?;
        Ok(binding)
    }
}
