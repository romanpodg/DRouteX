//! D11 authenticated immutable candidates. No publication, durable replay or runtime admission.
mod canonical;
mod content;
mod envelope;
mod resources;

pub use content::{CredentialLink, SnapshotContent};
pub use envelope::{SnapshotMetadata, VerifiedSnapshot, encode_snapshot, verify_snapshot};
pub use resources::{ContentHash, ResourcePin, SnapshotResource, TrustedResources};

use droutex_common::NodeScopeId;
use std::{error::Error, fmt, num::NonZeroU64};

pub const MAX_SNAPSHOT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_RESOURCE_PINS: usize = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotErrorCode {
    Size,
    Format,
    Schema,
    Configuration,
    Resource,
    CredentialLink,
    Canonical,
    Authentication,
    Scope,
    Replay,
    Sequence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotError(SnapshotErrorCode);
impl SnapshotError {
    pub const fn code(self) -> SnapshotErrorCode {
        self.0
    }
}
impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "snapshot {:?}", self.0)
    }
}
impl Error for SnapshotError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SnapshotSequence(NonZeroU64);
impl SnapshotSequence {
    pub fn new(sequence: u64) -> Result<Self, SnapshotError> {
        NonZeroU64::new(sequence)
            .map(Self)
            .ok_or(SnapshotError(SnapshotErrorCode::Sequence))
    }
    pub const fn get(self) -> u64 {
        self.0.get()
    }
    pub fn checked_next(self) -> Result<Self, SnapshotError> {
        self.get()
            .checked_add(1)
            .ok_or(SnapshotError(SnapshotErrorCode::Sequence))
            .and_then(Self::new)
    }
}

/// Explicit caller-trusted replay floor. No Default or implicit recovery to zero.
/// P1-06 must durably recheck/commit against concurrent publication and crash windows.
#[derive(Debug)]
pub struct ReplayState {
    node: NodeScopeId,
    high_water: u64,
}
impl ReplayState {
    /// Only explicit initial operator provisioning may establish this state.
    pub fn first_provisioning(node: NodeScopeId) -> Self {
        Self {
            node,
            high_water: 0,
        }
    }
    /// Caller must authenticate durable state independently of a snapshot/old DB.
    pub fn from_trusted_high_water(node: NodeScopeId, sequence: SnapshotSequence) -> Self {
        Self {
            node,
            high_water: sequence.get(),
        }
    }
    fn check(&self, node: NodeScopeId, sequence: SnapshotSequence) -> Result<(), SnapshotError> {
        if node != self.node {
            return Err(SnapshotError(SnapshotErrorCode::Scope));
        }
        if sequence.get() <= self.high_water {
            return Err(SnapshotError(SnapshotErrorCode::Replay));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
