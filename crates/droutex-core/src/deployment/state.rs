use super::{Artifacts, DeploymentError as Error, ErrorCode, Result};
use droutex_common::{ConfigurationRevisionId, NodeScopeId, SnapshotId};
use droutex_config::snapshot::{
    self, ContentHash, ReplayState, RestoredSnapshot, RetainedSnapshotIdentity, SnapshotContent,
    SnapshotMetadata, SnapshotSequence, VerifiedSnapshot,
};
use droutex_security::{SigningKeyId, SnapshotTrust};
use serde::{Deserialize, Serialize};

enum Authenticated {
    Fresh(VerifiedSnapshot),
    Retained(RestoredSnapshot),
}
pub(super) struct Generation {
    authenticated: Authenticated,
    pub(super) compiled: droutex_config::CompiledConfiguration,
    pub(super) envelope: Vec<u8>,
    pub(super) artifacts: Artifacts,
}
impl Generation {
    pub(super) fn fresh(
        envelope: Vec<u8>,
        artifacts: Artifacts,
        trust: &SnapshotTrust,
        journal: &Journal,
    ) -> Result<Self> {
        let replay = match journal.active.as_ref() {
            Some(active) => ReplayState::from_trusted_high_water(
                journal.node()?,
                SnapshotSequence::new(active.sequence).map_err(Error::snapshot)?,
            ),
            None => ReplayState::first_provisioning(journal.node()?),
        };
        let value = snapshot::verify_snapshot(&envelope, trust, &replay, &artifacts.trusted()?)
            .map_err(Error::snapshot)?;
        artifacts.check_exact(value.content().resource_pins())?;
        let compiled = value
            .compile(&artifacts.data())
            .map_err(|_| Error(ErrorCode::Compilation))?;
        Ok(Self {
            authenticated: Authenticated::Fresh(value),
            compiled,
            envelope,
            artifacts,
        })
    }
    pub(super) fn restore(
        envelope: Vec<u8>,
        artifacts: Artifacts,
        trust: &SnapshotTrust,
        descriptor: &Descriptor,
    ) -> Result<Self> {
        let hash = ContentHash::of(&envelope);
        if hash.as_bytes() != &descriptor.hash {
            return Err(Error(ErrorCode::State));
        }
        let value = snapshot::restore_snapshot(
            &envelope,
            trust,
            RetainedSnapshotIdentity {
                metadata: descriptor.metadata()?,
                signing_key: descriptor.key()?,
                envelope_hash: hash,
            },
            &artifacts.trusted()?,
        )
        .map_err(Error::snapshot)?;
        artifacts.check_exact(value.content().resource_pins())?;
        let compiled = value
            .compile(&artifacts.data())
            .map_err(|_| Error(ErrorCode::Compilation))?;
        Ok(Self {
            authenticated: Authenticated::Retained(value),
            compiled,
            envelope,
            artifacts,
        })
    }
    pub(super) fn metadata(&self) -> SnapshotMetadata {
        match &self.authenticated {
            Authenticated::Fresh(value) => value.metadata(),
            Authenticated::Retained(value) => value.metadata(),
        }
    }
    pub(super) fn content(&self) -> &SnapshotContent {
        match &self.authenticated {
            Authenticated::Fresh(value) => value.content(),
            Authenticated::Retained(value) => value.content(),
        }
    }
    fn key(&self) -> SigningKeyId {
        match &self.authenticated {
            Authenticated::Fresh(value) => value.signing_key(),
            Authenticated::Retained(value) => value.signing_key(),
        }
    }
    pub(super) fn authenticate(&self, trust: &SnapshotTrust) -> Result<()> {
        snapshot::restore_snapshot(
            &self.envelope,
            trust,
            RetainedSnapshotIdentity {
                metadata: self.metadata(),
                signing_key: self.key(),
                envelope_hash: ContentHash::of(&self.envelope),
            },
            &self.artifacts.trusted()?,
        )
        .map(|_| ())
        .map_err(Error::snapshot)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Descriptor {
    pub(super) slot: u8,
    node: [u8; 16],
    snapshot: [u8; 16],
    source: [u8; 16],
    pub(super) sequence: u64,
    signing_key: [u8; 16],
    hash: [u8; 32],
}
impl Descriptor {
    pub(super) fn new(generation: &Generation, slot: u8) -> Self {
        let metadata = generation.metadata();
        Self {
            slot,
            node: *metadata.node.as_bytes(),
            snapshot: *metadata.snapshot.as_bytes(),
            source: *metadata.source.as_bytes(),
            sequence: metadata.sequence.get(),
            signing_key: *generation.key().as_bytes(),
            hash: *ContentHash::of(&generation.envelope).as_bytes(),
        }
    }
    fn key(&self) -> Result<SigningKeyId> {
        SigningKeyId::from_bytes(self.signing_key).map_err(|_| Error(ErrorCode::State))
    }
    fn metadata(&self) -> Result<SnapshotMetadata> {
        let invalid = |_| Error(ErrorCode::State);
        Ok(SnapshotMetadata {
            node: NodeScopeId::from_bytes(self.node).map_err(invalid)?,
            snapshot: SnapshotId::from_bytes(self.snapshot).map_err(invalid)?,
            source: ConfigurationRevisionId::from_bytes(self.source).map_err(invalid)?,
            sequence: SnapshotSequence::new(self.sequence).map_err(Error::snapshot)?,
        })
    }
    fn validate(&self, node: NodeScopeId) -> Result<()> {
        if self.slot > 2 || self.metadata()?.node != node {
            return Err(Error(ErrorCode::State));
        }
        self.key()?;
        Ok(())
    }
}

/// The trusted local manifest commits active identity and floor in one replacement.
/// Previous has retention value only; it is never a recovery/admission fallback.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Journal {
    version: u8,
    node: [u8; 16],
    floor: u64,
    pub(super) active: Option<Descriptor>,
    pub(super) previous: Option<Descriptor>,
}
impl Journal {
    pub(super) fn empty(node: NodeScopeId) -> Self {
        Self {
            version: 1,
            node: *node.as_bytes(),
            floor: 0,
            active: None,
            previous: None,
        }
    }
    pub(super) fn node(&self) -> Result<NodeScopeId> {
        NodeScopeId::from_bytes(self.node).map_err(|_| Error(ErrorCode::State))
    }
    pub(super) fn validate(&self, node: NodeScopeId) -> Result<()> {
        if self.version != 1 || self.node()? != node {
            return Err(Error(ErrorCode::State));
        }
        match &self.active {
            Some(active) => {
                active.validate(node)?;
                if self.floor != active.sequence {
                    return Err(Error(ErrorCode::State));
                }
                if let Some(previous) = &self.previous {
                    previous.validate(node)?;
                    if previous.slot == active.slot || previous.sequence >= self.floor {
                        return Err(Error(ErrorCode::State));
                    }
                }
            }
            None if self.floor == 0 && self.previous.is_none() => (),
            None => return Err(Error(ErrorCode::State)),
        }
        Ok(())
    }
    pub(super) fn free_slot(&self) -> u8 {
        let occupied = |slot| {
            self.active.as_ref().is_some_and(|d| d.slot == slot)
                || self.previous.as_ref().is_some_and(|d| d.slot == slot)
        };
        if !occupied(0) {
            0
        } else if !occupied(1) {
            1
        } else {
            2
        }
    }
    pub(super) fn next(&self, active: Descriptor) -> Self {
        Self {
            version: 1,
            node: self.node,
            floor: active.sequence,
            active: Some(active),
            previous: self.active.clone(),
        }
    }
}
