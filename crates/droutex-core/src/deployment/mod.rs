//! D36 bounded activation. Storage methods block: async callers must use a blocking context.
//! A decision pins both consumer views once; never pin separately for DNS and gateway.
mod admission;
mod artifacts;
pub use admission::{
    AdmissionAttempt, AdmissionCode, AdmissionConsumption, AdmissionError, AdmissionScope,
    AuthorizedRequest, AutomaticLearning,
};
mod explain;
pub use explain::SimulationReport;
mod state;
mod store;

pub use artifacts::Artifacts;
use droutex_common::NodeScopeId;
use droutex_config::{
    CheckedConfiguration,
    snapshot::{self, SnapshotMetadata, SnapshotResource},
};
use droutex_security::SnapshotTrust;
use state::{Descriptor, Generation, Journal};
use std::{
    error::Error,
    fmt,
    path::Path,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};
use store::Store;

pub const MAX_RESOURCE_BYTES: usize = droutex_common::MAX_GENERATION_RESOURCE_BYTES;
pub const MAX_MANIFEST_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    Snapshot(snapshot::SnapshotErrorCode),
    Compilation,
    Size,
    Resource,
    State,
    Filesystem,
    UnsafePath,
    Busy,
    Cancelled,
    UncertainCommit,
    RecoveryRequired,
    Unavailable,
    UnsupportedPlatform,
    Admission(AdmissionCode),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeploymentError(ErrorCode);
impl DeploymentError {
    pub const fn code(self) -> ErrorCode {
        self.0
    }
    fn snapshot(error: snapshot::SnapshotError) -> Self {
        Self(ErrorCode::Snapshot(error.code()))
    }
}
impl fmt::Display for DeploymentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "deployment {:?}", self.0)
    }
}
impl Error for DeploymentError {}
type Result<T> = std::result::Result<T, DeploymentError>;

/// Signal only; no data is published through this flag, so relaxed ordering suffices.
#[derive(Default)]
pub struct Cancellation(AtomicBool);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    fn check(&self) -> Result<()> {
        if self.0.load(Ordering::Relaxed) {
            Err(DeploymentError(ErrorCode::Cancelled))
        } else {
            Ok(())
        }
    }
}

/// Decision-scoped pin, not a TCP session handle. Both consumers share one generation.
/// The owner cannot close/reopen the store while a decision is still using its pin.
///
/// ```compile_fail
/// use droutex_core::deployment::Deployment;
/// fn close_while_pinned(store: Deployment) {
///     let decision = store.pin().unwrap();
///     drop(store);
///     decision.metadata();
/// }
/// ```
pub struct Decision<'store> {
    generation: Arc<Generation>,
    _owner: &'store Deployment,
}
impl Decision<'_> {
    /// Both DNS and gateway consumers borrow the tables of this one pinned
    /// generation. These pure tables still confer no live network admission.
    pub fn compiled_configuration(&self) -> &droutex_config::CompiledConfiguration {
        &self.generation.compiled
    }
    pub fn metadata(&self) -> SnapshotMetadata {
        self.generation.metadata()
    }
    pub fn dns_configuration(&self) -> &CheckedConfiguration {
        self.generation.content().configuration()
    }
    pub fn gateway_configuration(&self) -> &CheckedConfiguration {
        self.generation.content().configuration()
    }
    pub fn artifact(&self, resource: SnapshotResource) -> Option<&[u8]> {
        self.generation.artifacts.get(resource)
    }
}

struct Writer {
    journal: Journal,
    trust: SnapshotTrust,
    previous: Option<Arc<Generation>>,
    store: Store,
    recovery_required: bool,
}
struct Published {
    active: Option<Arc<Generation>>,
    admitted: bool,
}

/// One writer serializes durable transitions; the reader lock never covers filesystem I/O.
pub struct Deployment {
    writer: Mutex<Writer>,
    published: RwLock<Published>,
    admission: Mutex<admission::AdmissionState>,
}
impl Deployment {
    /// Explicit ephemeral model; never silently substituted for a durable store.
    pub fn in_memory(node: NodeScopeId, trust: SnapshotTrust) -> Self {
        Self::new(Journal::empty(node), trust, Store::Memory, None)
    }
    /// Explicit first provisioning. The directory must not exist. CP roots must exist
    /// during this placement check; they are not needed to reopen the DP store.
    pub fn provision(
        root: &Path,
        cp_database: &Path,
        cp_backups: &Path,
        node: NodeScopeId,
        trust: SnapshotTrust,
    ) -> Result<Self> {
        let journal = Journal::empty(node);
        let store = Store::provision(root, cp_database, cp_backups, &journal)?;
        Ok(Self::new(journal, trust, store, None))
    }
    /// Missing/corrupt initialized state fails closed; this never provisions a new floor.
    pub fn open(root: &Path, node: NodeScopeId, trust: SnapshotTrust) -> Result<Self> {
        let (store, journal) = Store::open(root, node)?;
        let active = journal
            .active
            .as_ref()
            .map(|descriptor| store.restore(descriptor, &trust).map(Arc::new))
            .transpose()?;
        if active
            .as_ref()
            .is_some_and(|g| g.content().configuration().document().schema_version == 2)
            && journal.authorization_floor().is_none()
        {
            return Err(DeploymentError(ErrorCode::State));
        }
        store.recover(&journal)?;
        Ok(Self::new(journal, trust, store, active))
    }
    fn new(
        journal: Journal,
        trust: SnapshotTrust,
        store: Store,
        active: Option<Arc<Generation>>,
    ) -> Self {
        let admitted = active.is_some();
        let mut admission = admission::AdmissionState::new(journal.authorization_floor());
        if let Some(generation) = &active {
            admission.publish(
                &journal,
                &generation
                    .content()
                    .configuration()
                    .document()
                    .network_groups
                    .iter()
                    .map(|g| g.id)
                    .collect(),
            );
        }
        Self {
            writer: Mutex::new(Writer {
                journal,
                trust,
                store,
                previous: None,
                recovery_required: false,
            }),
            published: RwLock::new(Published { active, admitted }),
            admission: Mutex::new(admission),
        }
    }
    pub fn pin(&self) -> Result<Decision<'_>> {
        let state = self
            .published
            .read()
            .map_err(|_| DeploymentError(ErrorCode::State))?;
        if !state.admitted {
            return Err(DeploymentError(ErrorCode::Unavailable));
        }
        state
            .active
            .as_ref()
            .map(|active| Decision {
                generation: Arc::clone(active),
                _owner: self,
            })
            .ok_or(DeploymentError(ErrorCode::Unavailable))
    }
    /// Rechecks the current floor under the serialized writer. Err never means applied;
    /// UncertainCommit requires reopen before another publication. Committed data is
    /// never rolled back implicitly, including cancellation arriving after commit.
    pub fn activate(
        &self,
        envelope: Vec<u8>,
        artifacts: Artifacts,
        cancel: &Cancellation,
    ) -> Result<SnapshotMetadata> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| DeploymentError(ErrorCode::State))?;
        if writer.recovery_required {
            return Err(DeploymentError(ErrorCode::RecoveryRequired));
        }
        cancel.check()?;
        if writer
            .previous
            .as_ref()
            .is_some_and(|old| Arc::strong_count(old) > 1)
        {
            return Err(DeploymentError(ErrorCode::Busy));
        }
        let candidate = Generation::fresh(envelope, artifacts, &writer.trust, &writer.journal)?;
        let descriptor = Descriptor::new(&candidate, writer.journal.free_slot());
        let candidate = writer
            .store
            .stage(candidate, &descriptor, &writer.trust, cancel)?;
        cancel.check()?;
        let groups = candidate
            .content()
            .configuration()
            .document()
            .network_groups
            .iter()
            .map(|g| g.id)
            .collect();
        let mut next = writer.journal.next(descriptor);
        let _pause = self.pause_admission(
            &mut next,
            candidate
                .content()
                .configuration()
                .document()
                .schema_version,
            &groups,
        )?;
        next.validate(writer.journal.node()?)?;
        if let Err(error) = writer.store.commit(&next, cancel) {
            if error.code() == ErrorCode::UncertainCommit {
                writer.recovery_required = true;
                if let Ok(mut admission) = self.admission.lock() {
                    admission.fail_checkpoint();
                }
            }
            return Err(error);
        }
        // Acquire the publication lock only after I/O. Poisoning here must freeze
        // further mutations: durable floor already moved, no false success/rollback.
        let mut published = match self.published.write() {
            Ok(guard) => guard,
            Err(_) => {
                writer.recovery_required = true;
                return Err(DeploymentError(ErrorCode::UncertainCommit));
            }
        };
        let metadata = candidate.metadata();
        let mut admission = match self.admission.lock() {
            Ok(guard) => guard,
            Err(_) => {
                writer.recovery_required = true;
                return Err(DeploymentError(ErrorCode::UncertainCommit));
            }
        };
        admission.publish(&next, &groups);
        let previous = published.active.replace(Arc::new(candidate));
        published.admitted = true;
        drop(published);
        writer.previous = previous;
        writer.journal = next;
        Ok(metadata)
    }
    /// Independently provisioned trust only. Applies immediately for new pins;
    /// existing decisions may finish. A false result means no current usable active
    /// generation, while the trust replacement itself has still taken effect.
    pub fn replace_trust(&self, trust: SnapshotTrust) -> Result<bool> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| DeploymentError(ErrorCode::State))?;
        let mut published = self
            .published
            .write()
            .map_err(|_| DeploymentError(ErrorCode::State))?;
        published.admitted = published
            .active
            .as_ref()
            .is_some_and(|active| active.authenticate(&trust).is_ok());
        writer.trust = trust;
        Ok(published.admitted)
    }
}

#[cfg(test)]
mod tests;
