use super::{
    AdmissionCode, AdmissionError, AdmissionScope, AdmissionState, AuthorizedRequest, GrantLease,
    Result,
};
use crate::deployment::{Published, state::Generation};
use std::{
    fmt,
    sync::{Arc, Mutex, RwLock, Weak},
};

/// DNS-only live check for a bounded library I/O task. It owns no reservation,
/// generation pin or deployment, and cannot admit work or select a destination.
/// Every I/O resume must check it while the original AuthorizedRequest is held.
/// Dropping that owner permanently invalidates every derived check.
pub struct DnsIoCheck {
    lifetime: Weak<()>,
    published: Weak<RwLock<Published>>,
    admission: Weak<Mutex<AdmissionState>>,
    generation: Weak<Generation>,
    lease: GrantLease,
}
/// Gateway I/O liveness for one held admission. It cannot admit a new flow,
/// choose a destination, check an origin address, or outlive its owner.
pub struct GatewayIoCheck(DnsIoCheck);
impl fmt::Debug for GatewayIoCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GatewayIoCheck").finish_non_exhaustive()
    }
}
impl GatewayIoCheck {
    pub fn verify_current(&self) -> Result<()> {
        self.0.verify_current()
    }
}
impl fmt::Debug for DnsIoCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DnsIoCheck").finish_non_exhaustive()
    }
}
impl AuthorizedRequest<'_> {
    /// Derive a check for the same DNS reservation; never reacquire or refund quota.
    /// This does not authorize a gateway connection or bypass destination checks.
    pub fn dns_io_check(&self) -> Result<DnsIoCheck> {
        if !matches!(self.scope(), AdmissionScope::LegacyDns(_)) {
            return Err(AdmissionError(AdmissionCode::Scope));
        }
        Ok(self.io_check())
    }
    /// A weak current-generation check for the existing gateway reservation.
    /// Origin resolution and numeric connection still need their own checks.
    pub fn gateway_io_check(&self) -> Result<GatewayIoCheck> {
        if !matches!(self.scope(), AdmissionScope::Gateway(_)) {
            return Err(AdmissionError(AdmissionCode::Scope));
        }
        Ok(GatewayIoCheck(self.io_check()))
    }
    fn io_check(&self) -> DnsIoCheck {
        let reservation = &self.attempt.reservation;
        let owner = reservation.decision._owner;
        let check = DnsIoCheck {
            lifetime: Arc::downgrade(&self.lifetime),
            published: Arc::downgrade(&owner.published),
            admission: Arc::downgrade(&owner.admission),
            generation: Arc::downgrade(&reservation.decision.generation),
            lease: reservation.lease,
        };
        // This only constructs a checker. A caller must run its live check at
        // I/O consumption, including the existing bounded checkpoint wait.
        check
    }
}
impl DnsIoCheck {
    pub fn verify_current(&self) -> Result<()> {
        self.owner_alive()?;
        let published = self
            .published
            .upgrade()
            .ok_or(AdmissionError(AdmissionCode::Retired))?;
        let admission = self
            .admission
            .upgrade()
            .ok_or(AdmissionError(AdmissionCode::Retired))?;
        let generation = self
            .generation
            .upgrade()
            .ok_or(AdmissionError(AdmissionCode::Retired))?;
        check_current(&published, &admission, &generation, self.lease)?;
        self.owner_alive()
    }
    fn owner_alive(&self) -> Result<()> {
        // Never upgrade this Weak: concurrent checks must not keep one another
        // authorized after the sole original owner drops its strong token.
        if self.lifetime.strong_count() == 0 {
            Err(AdmissionError(AdmissionCode::Retired))
        } else {
            Ok(())
        }
    }
}

pub(super) fn check_current(
    published: &RwLock<Published>,
    admission: &Mutex<AdmissionState>,
    generation: &Arc<Generation>,
    lease: GrantLease,
) -> Result<()> {
    check_current_leases(published, admission, generation, &[lease])
}

pub(super) fn check_current_leases(
    published: &RwLock<Published>,
    admission: &Mutex<AdmissionState>,
    generation: &Arc<Generation>,
    leases: &[GrantLease],
) -> Result<()> {
    // Same linearization as original consumption: hold the current publication
    // read lock through the lease/clock check. Neither lock spans filesystem I/O.
    let published = published
        .read()
        .map_err(|_| AdmissionError(AdmissionCode::State))?;
    if !published.admitted
        || published
            .active
            .as_ref()
            .is_none_or(|active| !Arc::ptr_eq(active, generation))
    {
        return Err(AdmissionError(AdmissionCode::Retired));
    }
    admission
        .lock()
        .map_err(|_| AdmissionError(AdmissionCode::State))?
        .interval_many(leases)?;
    Ok(())
}
