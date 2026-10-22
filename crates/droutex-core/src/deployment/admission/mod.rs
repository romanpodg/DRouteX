//! D46 current admission. No socket, origin check or connector is supplied here.
mod clock;
mod quota;
use super::{Cancellation, Decision, Deployment, DeploymentError, ErrorCode, Journal};
use droutex_common::{DnsProfileBinding, GatewayNetworkIdentity, ListenerId, NetworkGroupId};
use droutex_config::{
    DnsAccess, GatewayCandidate, GrantLease, ListenerTransport, OutboundObservation,
};
use std::{
    collections::BTreeSet, error::Error, fmt, net::IpAddr, num::NonZeroU16, sync::Arc,
    time::Duration,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionCode {
    Unavailable,
    UnknownSource,
    Scope,
    Lease,
    Clock,
    RestartWait,
    CheckpointRequired,
    Quota,
    Retired,
    State,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionError(pub(super) AdmissionCode);
impl AdmissionError {
    pub fn code(self) -> AdmissionCode {
        self.0
    }
}
impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "admission {:?}", self.0)
    }
}
impl Error for AdmissionError {}
type Result<T> = std::result::Result<T, AdmissionError>;
#[derive(Clone, Copy)]
enum Plane {
    Dns,
    Gateway,
}
/// Scope evidence is not permission to resolve/connect or an effective route.
pub enum AdmissionScope {
    LegacyDns(DnsProfileBinding),
    Gateway(GatewayCandidate),
}
impl fmt::Debug for AdmissionScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LegacyDns(_) => "LegacyDns",
            Self::Gateway(_) => "Gateway",
        })
    }
}
/// Explicitly unavailable until a real P3-07 verified Secure DNS producer exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutomaticLearning {
    UnavailableUntilSecureDns,
}

pub(super) struct AdmissionState {
    clock: clock::Clock,
    floor: Option<u64>,
    checkpoint_failed: bool,
    publication_paused: bool,
    quotas: quota::Quotas,
}
impl AdmissionState {
    #[cfg(test)]
    pub fn fake_start(&mut self, wall: u64) {
        self.clock.fake_start(wall);
    }
    #[cfg(test)]
    pub fn fake_time(&mut self, wall: u64, elapsed: u64) {
        self.clock.fake = Some(clock::Reading {
            wall: Duration::from_secs(wall),
            elapsed: Duration::from_secs(elapsed),
        });
    }
    pub fn new(floor: Option<u64>) -> Self {
        Self {
            clock: clock::Clock::new(floor),
            floor,
            checkpoint_failed: false,
            publication_paused: false,
            quotas: quota::Quotas::new(),
        }
    }
    fn interval(&mut self, lease: GrantLease) -> Result<clock::Interval> {
        if self.checkpoint_failed {
            return Err(AdmissionError(AdmissionCode::Unavailable));
        }
        let interval = self.clock.sample()?;
        if interval.floor()?
            > self
                .floor
                .ok_or(AdmissionError(AdmissionCode::Unavailable))?
        {
            return Err(AdmissionError(AdmissionCode::CheckpointRequired));
        }
        if interval.lower < Duration::from_secs(lease.issued_at())
            || interval.upper >= Duration::from_secs(lease.expires_at())
        {
            return Err(AdmissionError(AdmissionCode::Lease));
        }
        Ok(interval)
    }
    pub fn preflight(&self, groups: &BTreeSet<NetworkGroupId>) -> Result<()> {
        self.quotas.preflight(groups)
    }
    pub fn publication_floor(&mut self, journal: &mut Journal, version: u32) -> Result<()> {
        if version == 2 {
            match self.clock.sample() {
                Ok(interval) => journal
                    .advance_clock_floor(interval.floor()?)
                    .map_err(|_| AdmissionError(AdmissionCode::State))?,
                Err(_) if journal.authorization_floor().is_some() => (), // A revocation can still publish while time is unavailable.
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
    pub fn publish(&mut self, journal: &Journal, groups: &BTreeSet<NetworkGroupId>) {
        self.floor = journal.authorization_floor();
        // A fresh config never clears clock failure or an unresolved checkpoint failure.
        let elapsed = self.clock.elapsed();
        self.quotas.reconcile(groups, elapsed);
    }
    pub fn fail_checkpoint(&mut self) {
        self.checkpoint_failed = true;
    }
}

struct Reservation<'store> {
    decision: Decision<'store>,
    group: NetworkGroupId,
    plane: Plane,
    lease: GrantLease,
}
/// One quota reservation from the actual current generation; not a connector permit.
/// Consumption rechecks the current generation and lease. Drop releases concurrency.
///
/// ```compile_fail
/// use droutex_core::deployment::AdmissionAttempt;
/// let forged: AdmissionAttempt<'_> = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// use droutex_core::deployment::{AuthorizedRequest,Decision};
/// fn stale<'store>(old: Decision<'store>) -> AuthorizedRequest<'store> {
///     old.prepare_legacy_dns("10000000-0000-4000-8000-000000000020".parse().unwrap(), "203.0.113.7".parse().unwrap()).unwrap().consume().unwrap()
/// }
/// ```
pub struct AdmissionAttempt<'store> {
    reservation: Reservation<'store>,
    scope: AdmissionScope,
}
impl fmt::Debug for AdmissionAttempt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdmissionAttempt")
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}
impl Reservation<'_> {
    fn check_current(&self) -> Result<()> {
        let owner = self.decision._owner;
        let published = owner
            .published
            .read()
            .map_err(|_| AdmissionError(AdmissionCode::State))?;
        if !published.admitted
            || published
                .active
                .as_ref()
                .is_none_or(|g| !Arc::ptr_eq(g, &self.decision.generation))
        {
            return Err(AdmissionError(AdmissionCode::Retired));
        }
        owner
            .admission
            .lock()
            .map_err(|_| AdmissionError(AdmissionCode::State))?
            .interval(self.lease)?;
        Ok(())
    }
}
impl<'store> AdmissionAttempt<'store> {
    /// Linearization point: current publication read lock plus current lease check.
    /// The result authorizes one bounded attempt, never an origin address or socket.
    pub fn consume(self) -> Result<AuthorizedRequest<'store>> {
        self.reservation.check_current()?;
        Ok(AuthorizedRequest { attempt: self })
    }
}
impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.decision._owner.admission.lock() {
            state.quotas.release(self.group, self.plane);
        }
    }
}
/// Non-Clone, non-deserializable single-request scope evidence. Actual P3/P4 callers
/// must use the current gate per request and again at final connect admission.
pub struct AuthorizedRequest<'store> {
    attempt: AdmissionAttempt<'store>,
}
impl AuthorizedRequest<'_> {
    pub fn scope(&self) -> &AdmissionScope {
        &self.attempt.scope
    }
    pub fn decision(&self) -> &Decision<'_> {
        &self.attempt.reservation.decision
    }
    pub fn verify_current(&self) -> Result<()> {
        self.attempt.reservation.check_current()
    }
    /// Recheck the exact canonical target of this attempt, not the wider grant.
    /// Origin address validation and checked-IP pinning remain P4-02 obligations.
    pub fn verify_gateway_destination(&self, domain: &str, port: NonZeroU16) -> Result<()> {
        match self.scope() {
            AdmissionScope::Gateway(candidate) if candidate.matches_destination(domain, port) => {
                self.verify_current()
            }
            _ => Err(AdmissionError(AdmissionCode::Scope)),
        }
    }
}
impl fmt::Debug for AuthorizedRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthorizedRequest")
            .field("scope", &self.attempt.scope)
            .finish_non_exhaustive()
    }
}

impl Deployment {
    /// Freeze acquisitions between inventory preflight and durable publication.
    /// No lock is held across filesystem I/O; old reservations may drain normally.
    pub(super) fn pause_admission(
        &self,
        journal: &mut Journal,
        version: u32,
        groups: &BTreeSet<NetworkGroupId>,
    ) -> super::Result<PublicationPause<'_>> {
        let mut state = self
            .admission
            .lock()
            .map_err(|_| DeploymentError(ErrorCode::State))?;
        state.preflight(groups).map_err(admission_error)?;
        state
            .publication_floor(journal, version)
            .map_err(admission_error)?;
        state.publication_paused = true;
        Ok(PublicationPause(self))
    }
    pub fn automatic_learning(&self) -> AutomaticLearning {
        AutomaticLearning::UnavailableUntilSecureDns
    }
    /// Trusted in-process caller must supply the actual transport peer, never headers.
    /// IPv4-mapped IPv6 is refused; transport normalization needs its own explicit gate.
    pub fn prepare_legacy_dns(
        &self,
        listener: ListenerId,
        peer: IpAddr,
    ) -> Result<AdmissionAttempt<'_>> {
        let (reservation, binding) = self.reserve(peer, Plane::Dns, |generation| {
            let document = generation.content().configuration().document();
            let mut grants = document
                .legacy_grants
                .iter()
                .filter(|g| g.source == peer && g.listener == listener);
            let grant = grants
                .next()
                .ok_or(AdmissionError(AdmissionCode::UnknownSource))?;
            if grants.next().is_some() {
                return Err(AdmissionError(AdmissionCode::Scope));
            }
            let socket = document
                .listeners
                .iter()
                .find(|l| l.id == listener && l.enabled)
                .ok_or(AdmissionError(AdmissionCode::Scope))?;
            let endpoint = document
                .endpoints
                .iter()
                .find(|e| e.id == grant.endpoint && e.enabled)
                .ok_or(AdmissionError(AdmissionCode::Scope))?;
            let linked = endpoint.access.iter().any(|access| match access {
                DnsAccess::Udp {
                    listener: id,
                    grants,
                } if socket.transport == ListenerTransport::DnsUdp => {
                    *id == listener && grants.contains(&grant.id)
                }
                DnsAccess::Tcp {
                    listener: id,
                    grants,
                } if socket.transport == ListenerTransport::DnsTcp => {
                    *id == listener && grants.contains(&grant.id)
                }
                _ => false,
            });
            if !linked {
                return Err(AdmissionError(AdmissionCode::Scope));
            }
            Ok((
                grant.group,
                grant.lease.ok_or(AdmissionError(AdmissionCode::Lease))?,
                DnsProfileBinding {
                    endpoint: endpoint.id,
                    profile: endpoint.profile,
                },
            ))
        })?;
        Ok(AdmissionAttempt {
            reservation,
            scope: AdmissionScope::LegacyDns(binding),
        })
    }
    /// Separately select the gateway group's current policy and approved outbound.
    /// Availability observations are supplied by the trusted runtime, not the client.
    pub fn prepare_gateway(
        &self,
        listener: ListenerId,
        peer: IpAddr,
        domain: &str,
        port: NonZeroU16,
        observations: Vec<OutboundObservation>,
    ) -> Result<AdmissionAttempt<'_>> {
        let (reservation, identity) = self.reserve(peer, Plane::Gateway, |generation| {
            let document = generation.content().configuration().document();
            let mut grants = document
                .gateway_grants
                .iter()
                .filter(|g| g.source == peer && g.listener == listener);
            let grant = grants
                .next()
                .ok_or(AdmissionError(AdmissionCode::UnknownSource))?;
            if grants.next().is_some() {
                return Err(AdmissionError(AdmissionCode::Scope));
            }
            let identity = GatewayNetworkIdentity {
                grant: grant.id,
                group: grant.group,
                listener,
                source: peer,
            };
            Ok((
                grant.group,
                grant.lease.ok_or(AdmissionError(AdmissionCode::Lease))?,
                identity,
            ))
        })?;
        // Reservation precedes expensive target/policy evaluation. Invalid attempts
        // spend rate credits; failure drops concurrency without refunding those credits.
        let generation = &reservation.decision.generation;
        let candidate = generation
            .content()
            .gateway_candidate(&generation.compiled, identity, domain, port, observations)
            .map_err(|_| AdmissionError(AdmissionCode::Scope))?;
        Ok(AdmissionAttempt {
            reservation,
            scope: AdmissionScope::Gateway(candidate),
        })
    }
    fn reserve<S>(
        &self,
        peer: IpAddr,
        plane: Plane,
        resolve: impl FnOnce(&super::Generation) -> Result<(NetworkGroupId, GrantLease, S)>,
    ) -> Result<(Reservation<'_>, S)> {
        if matches!(peer,IpAddr::V6(ip) if ip.to_ipv4_mapped().is_some()) {
            return Err(AdmissionError(AdmissionCode::Scope));
        }
        let published = self
            .published
            .read()
            .map_err(|_| AdmissionError(AdmissionCode::State))?;
        let generation = published
            .active
            .as_ref()
            .filter(|_| published.admitted)
            .ok_or(AdmissionError(AdmissionCode::Unavailable))?;
        if generation
            .content()
            .configuration()
            .document()
            .schema_version
            != 2
        {
            return Err(AdmissionError(AdmissionCode::Unavailable));
        }
        let (group, lease, scope) = resolve(generation)?;
        let mut state = self
            .admission
            .lock()
            .map_err(|_| AdmissionError(AdmissionCode::State))?;
        if state.publication_paused {
            return Err(AdmissionError(AdmissionCode::Unavailable));
        }
        let interval = state.interval(lease)?;
        state.quotas.acquire(group, plane, interval.elapsed)?;
        Ok((
            Reservation {
                decision: Decision {
                    generation: Arc::clone(generation),
                    _owner: self,
                },
                group,
                lease,
                plane,
            },
            scope,
        ))
    }

    /// Blocking local maintenance, independent of CP/DB. Async callers must use a
    /// bounded blocking context. Failure denies new admission; never resets a floor.
    pub fn refresh_authorization_clock(&self, cancel: &Cancellation) -> super::Result<()> {
        self.maintain_clock(cancel, false)
    }
    /// Explicit trusted local repair after host UTC correction. No public reset API.
    pub fn reanchor_authorization_clock(&self, cancel: &Cancellation) -> super::Result<()> {
        self.maintain_clock(cancel, true)
    }
    fn maintain_clock(&self, cancel: &Cancellation, reanchor: bool) -> super::Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| DeploymentError(ErrorCode::State))?;
        if writer.recovery_required {
            return Err(DeploymentError(ErrorCode::RecoveryRequired));
        }
        let floor = writer
            .journal
            .authorization_floor()
            .ok_or(DeploymentError(ErrorCode::Unavailable))?;
        let mut next = writer.journal.clone();
        {
            let mut state = self
                .admission
                .lock()
                .map_err(|_| DeploymentError(ErrorCode::State))?;
            state.checkpoint_failed = true;
            if reanchor {
                state.clock.reanchor(floor).map_err(admission_error)?;
            }
            next.advance_clock_floor(
                state
                    .clock
                    .sample()
                    .map_err(admission_error)?
                    .floor()
                    .map_err(admission_error)?,
            )?;
        }
        if let Err(error) = writer.store.commit(&next, cancel) {
            if error.code() == ErrorCode::UncertainCommit {
                writer.recovery_required = true;
            }
            return Err(error);
        }
        let mut state = self
            .admission
            .lock()
            .map_err(|_| DeploymentError(ErrorCode::State))?;
        state.floor = next.authorization_floor();
        state.checkpoint_failed = false;
        writer.journal = next;
        Ok(())
    }
}
pub(super) struct PublicationPause<'deployment>(&'deployment Deployment);
impl Drop for PublicationPause<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.admission.lock() {
            state.publication_paused = false;
        }
    }
}
pub(super) fn admission_error(error: AdmissionError) -> DeploymentError {
    DeploymentError(ErrorCode::Admission(error.code()))
}

#[cfg(test)]
mod tests;
