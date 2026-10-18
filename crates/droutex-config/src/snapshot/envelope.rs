use super::{
    ContentHash, MAX_SNAPSHOT_BYTES, ReplayState, SnapshotContent, SnapshotError,
    SnapshotErrorCode as Code, SnapshotSequence, TrustedResources,
};
use droutex_common::{ConfigurationRevisionId, NodeScopeId, SnapshotId};
use droutex_security::{SigningKey, SigningKeyId, SnapshotTrust};
use std::fmt;

const MAGIC: &[u8; 8] = b"DXSNAP\0\x01";
const HEADER: usize = 121;
pub(super) const OVERHEAD: usize = HEADER + 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotMetadata {
    pub node: NodeScopeId,
    pub snapshot: SnapshotId,
    pub source: ConfigurationRevisionId,
    pub sequence: SnapshotSequence,
}

/// Only verification constructs this candidate. It does not advance durable state.
///
/// ```compile_fail
/// use droutex_config::{CheckedConfiguration, snapshot::VerifiedSnapshot};
/// fn admit(_: VerifiedSnapshot) {}
/// fn unchecked(config: CheckedConfiguration) { admit(config); }
/// ```
pub struct VerifiedSnapshot {
    metadata: SnapshotMetadata,
    signing_key: SigningKeyId,
    content: SnapshotContent,
}
impl VerifiedSnapshot {
    pub const fn metadata(&self) -> SnapshotMetadata {
        self.metadata
    }
    pub const fn signing_key(&self) -> SigningKeyId {
        self.signing_key
    }
    pub fn content(&self) -> &SnapshotContent {
        &self.content
    }
}
impl fmt::Debug for VerifiedSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerifiedSnapshot")
            .field("metadata", &self.metadata)
            .field("hash", &self.content.hash())
            .finish_non_exhaustive()
    }
}

/// Trusted publisher builds a candidate; publication authorization/sequence allocation
/// is the caller's P1-06 responsibility, not conferred by possession of this function.
pub fn encode_snapshot(
    content: &SnapshotContent,
    metadata: SnapshotMetadata,
    key: &SigningKey,
) -> Result<Vec<u8>, SnapshotError> {
    let length = u32::try_from(content.as_bytes().len()).map_err(|_| SnapshotError(Code::Size))?;
    if content.as_bytes().len() > MAX_SNAPSHOT_BYTES - OVERHEAD {
        return Err(SnapshotError(Code::Size));
    }
    let mut bytes = Vec::with_capacity(OVERHEAD + content.as_bytes().len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes.push(1);
    bytes.extend_from_slice(key.id().as_bytes());
    bytes.extend_from_slice(metadata.node.as_bytes());
    bytes.extend_from_slice(metadata.snapshot.as_bytes());
    bytes.extend_from_slice(metadata.source.as_bytes());
    bytes.extend_from_slice(&metadata.sequence.get().to_be_bytes());
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(content.hash().as_bytes());
    bytes.extend_from_slice(content.as_bytes());
    let signature = key
        .sign_snapshot(&bytes)
        .map_err(|_| SnapshotError(Code::Authentication))?;
    bytes.extend_from_slice(&signature);
    Ok(bytes)
}

pub fn verify_snapshot(
    bytes: &[u8],
    trust: &SnapshotTrust,
    replay: &ReplayState,
    resources: &TrustedResources,
) -> Result<VerifiedSnapshot, SnapshotError> {
    authenticate_snapshot(bytes, trust, resources, Some(replay))
}

/// Exact identity obtained from a separately trusted durable activation manifest.
/// Possession grants no fresh-admission or sequence-reset authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetainedSnapshotIdentity {
    pub metadata: SnapshotMetadata,
    pub signing_key: SigningKeyId,
    pub envelope_hash: ContentHash,
}

/// Authenticated retained state, deliberately distinct from a fresh candidate.
///
/// ```compile_fail
/// use droutex_config::snapshot::{RestoredSnapshot, VerifiedSnapshot};
/// fn fresh(_: VerifiedSnapshot) {}
/// fn restore(value: RestoredSnapshot) { fresh(value); }
/// ```
pub struct RestoredSnapshot(VerifiedSnapshot);
impl RestoredSnapshot {
    pub const fn metadata(&self) -> SnapshotMetadata {
        self.0.metadata()
    }
    pub const fn signing_key(&self) -> SigningKeyId {
        self.0.signing_key()
    }
    pub fn content(&self) -> &SnapshotContent {
        self.0.content()
    }
}

pub fn restore_snapshot(
    bytes: &[u8],
    trust: &SnapshotTrust,
    expected: RetainedSnapshotIdentity,
    resources: &TrustedResources,
) -> Result<RestoredSnapshot, SnapshotError> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SnapshotError(Code::Size));
    }
    if ContentHash::of(bytes) != expected.envelope_hash {
        return Err(SnapshotError(Code::Authentication));
    }
    let verified = authenticate_snapshot(bytes, trust, resources, None)?;
    if verified.metadata != expected.metadata || verified.signing_key != expected.signing_key {
        return Err(SnapshotError(Code::Scope));
    }
    Ok(RestoredSnapshot(verified))
}

fn authenticate_snapshot(
    bytes: &[u8],
    trust: &SnapshotTrust,
    resources: &TrustedResources,
    replay: Option<&ReplayState>,
) -> Result<VerifiedSnapshot, SnapshotError> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SnapshotError(Code::Size));
    }
    if bytes.len() < OVERHEAD || &bytes[..8] != MAGIC {
        return Err(SnapshotError(Code::Format));
    }
    if bytes[8..12] != 1_u32.to_be_bytes() || bytes[12] != 1 {
        return Err(SnapshotError(Code::Schema));
    }
    let length = u32::from_be_bytes(part(bytes, 85)?) as usize;
    if length != bytes.len() - OVERHEAD {
        return Err(SnapshotError(Code::Format));
    }
    let signing_key =
        SigningKeyId::from_bytes(part(bytes, 13)?).map_err(|_| SnapshotError(Code::Format))?;
    let signed_length = bytes.len() - 64;
    let signature = bytes[signed_length..]
        .try_into()
        .map_err(|_| SnapshotError(Code::Format))?;
    trust
        .verify_snapshot(signing_key, &bytes[..signed_length], signature)
        .map_err(|_| SnapshotError(Code::Authentication))?;
    let metadata = SnapshotMetadata {
        node: NodeScopeId::from_bytes(part(bytes, 29)?).map_err(|_| SnapshotError(Code::Format))?,
        snapshot: SnapshotId::from_bytes(part(bytes, 45)?)
            .map_err(|_| SnapshotError(Code::Format))?,
        source: ConfigurationRevisionId::from_bytes(part(bytes, 61)?)
            .map_err(|_| SnapshotError(Code::Format))?,
        sequence: SnapshotSequence::new(u64::from_be_bytes(part(bytes, 77)?))?,
    };
    if let Some(replay) = replay {
        replay.check(metadata.node, metadata.sequence)?;
    }
    let payload = &bytes[HEADER..signed_length];
    if ContentHash::of(payload).as_bytes() != &bytes[89..121] {
        return Err(SnapshotError(Code::Authentication));
    }
    let content = SnapshotContent::decode(payload, resources)?;
    Ok(VerifiedSnapshot {
        metadata,
        signing_key,
        content,
    })
}

fn part<const N: usize>(bytes: &[u8], start: usize) -> Result<[u8; N], SnapshotError> {
    bytes
        .get(start..start + N)
        .ok_or(SnapshotError(Code::Format))?
        .try_into()
        .map_err(|_| SnapshotError(Code::Format))
}
