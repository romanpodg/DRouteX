use super::{
    Cancellation, DeploymentError as Error, ErrorCode, Result,
    state::{Descriptor, Generation, Journal},
};
use droutex_common::NodeScopeId;
use droutex_security::SnapshotTrust;
use std::path::Path;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux;

pub(super) enum Store {
    Memory,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    Files(linux::FileStore),
}
impl Store {
    pub(super) fn provision(
        root: &Path,
        database: &Path,
        backup: &Path,
        journal: &Journal,
    ) -> Result<Self> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            linux::FileStore::provision(root, database, backup, journal).map(Self::Files)
        }
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        {
            let _ = (root, database, backup, journal);
            Err(Error(ErrorCode::UnsupportedPlatform))
        }
    }
    pub(super) fn open(root: &Path, node: NodeScopeId) -> Result<(Self, Journal)> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            let (store, journal) = linux::FileStore::open(root, node)?;
            Ok((Self::Files(store), journal))
        }
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        {
            let _ = (root, node);
            Err(Error(ErrorCode::UnsupportedPlatform))
        }
    }
    pub(super) fn stage(
        &self,
        candidate: Generation,
        descriptor: &Descriptor,
        trust: &SnapshotTrust,
        cancel: &Cancellation,
    ) -> Result<Generation> {
        match self {
            Self::Memory => {
                cancel.check()?;
                Generation::restore(candidate.envelope, candidate.artifacts, trust, descriptor)
            }
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::Files(store) => store.stage(candidate, descriptor, trust, cancel),
        }
    }
    pub(super) fn commit(&self, journal: &Journal) -> Result<()> {
        match self {
            Self::Memory => {
                let _ = journal;
                Ok(())
            }
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::Files(store) => store.commit(journal),
        }
    }
    pub(super) fn restore(
        &self,
        descriptor: &Descriptor,
        trust: &SnapshotTrust,
    ) -> Result<Generation> {
        match self {
            Self::Memory => {
                let _ = (descriptor, trust);
                Err(Error(ErrorCode::State))
            }
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::Files(store) => store.restore(descriptor, trust),
        }
    }
    pub(super) fn recover(&self, journal: &Journal) -> Result<()> {
        match self {
            Self::Memory => {
                let _ = journal;
                Ok(())
            }
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::Files(store) => store.recover(journal),
        }
    }
}
