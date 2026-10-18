//! Owner-approved D11 snapshot-only signature boundary, separate from AES keys.
use aws_lc_rs::{
    digest::{SHA256, digest},
    signature::{Ed25519KeyPair, KeyPair},
};
use ed25519_dalek::{Signature, VerifyingKey};
use std::{collections::BTreeMap, error::Error, fmt, path::Path};
use zeroize::Zeroizing;

const DOMAIN: &[u8] = b"DRouteX snapshot v1\0";
const MAX_SIGNED_BYTES: usize = 8 * 1024 * 1024;
const MAX_PKCS8_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SigningErrorCode {
    Key,
    Trust,
    Signature,
    Size,
    ProtectedFile(crate::SecretErrorCode),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SigningError(SigningErrorCode);

impl SigningError {
    pub const fn code(self) -> SigningErrorCode {
        self.0
    }
}
impl fmt::Display for SigningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "snapshot signing {:?}", self.0)
    }
}
impl Error for SigningError {}

/// Opaque signing-key identity. It is distinct from an encryption KeyId.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SigningKeyId([u8; 16]);

impl SigningKeyId {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, SigningError> {
        if bytes == [0; 16] {
            return Err(SigningError(SigningErrorCode::Key));
        }
        Ok(Self(bytes))
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// No implicit export, serialization or copying of the signing private key.
///
/// ```compile_fail
/// use droutex_security::SigningKey;
/// fn accidental_copy(key: SigningKey) { let _ = key.clone(); }
/// ```
pub struct SigningKey {
    id: SigningKeyId,
    pair: Ed25519KeyPair,
}

impl SigningKey {
    /// Caller owns secure provisioning. Expected public key is independently trusted.
    /// Zeroizes this owned DER buffer, not caller copies or library/OS internals.
    pub fn from_pkcs8(
        id: SigningKeyId,
        bytes: Vec<u8>,
        expected_public: &[u8; 32],
    ) -> Result<Self, SigningError> {
        let bytes = Zeroizing::new(bytes);
        if bytes.is_empty() || bytes.len() > MAX_PKCS8_BYTES {
            return Err(SigningError(SigningErrorCode::Key));
        }
        let pair =
            Ed25519KeyPair::from_pkcs8(&bytes).map_err(|_| SigningError(SigningErrorCode::Key))?;
        if pair.public_key().as_ref() != expected_public {
            return Err(SigningError(SigningErrorCode::Key));
        }
        Ok(Self { id, pair })
    }

    /// Blocking Linux x86_64 file boundary; async callers must use blocking work.
    pub fn load_file(
        path: &Path,
        database: &Path,
        backup: &Path,
        id: SigningKeyId,
        expected_public: &[u8; 32],
    ) -> Result<Self, SigningError> {
        let bytes = crate::paths::read_bounded(path, database, backup, MAX_PKCS8_BYTES)
            .map_err(|e| SigningError(SigningErrorCode::ProtectedFile(e.code())))?;
        Self::from_pkcs8(id, bytes, expected_public)
    }

    pub const fn id(&self) -> SigningKeyId {
        self.id
    }

    /// Signs only the domain-separated snapshot protocol. Not a general signer.
    pub fn sign_snapshot(&self, bytes: &[u8]) -> Result<[u8; 64], SigningError> {
        let signature = self
            .pair
            .try_sign(&message(bytes)?)
            .map_err(|_| SigningError(SigningErrorCode::Signature))?;
        signature
            .as_ref()
            .try_into()
            .map_err(|_| SigningError(SigningErrorCode::Signature))
    }
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SigningKey([REDACTED])")
    }
}

/// Explicit operator-provisioned verification trust, never read from a snapshot.
/// Public keys must originate from the independently authenticated provisioner.
/// This type does not grant publication/admission permission or persist revocation.
#[derive(Debug)]
pub struct SnapshotTrust(BTreeMap<SigningKeyId, VerifyingKey>);

impl SnapshotTrust {
    pub fn new(keys: Vec<(SigningKeyId, [u8; 32])>) -> Result<Self, SigningError> {
        if keys.is_empty() || keys.len() > 32 {
            return Err(SigningError(SigningErrorCode::Trust));
        }
        let mut trusted = BTreeMap::new();
        for (id, public) in keys {
            let key = VerifyingKey::from_bytes(&public)
                .map_err(|_| SigningError(SigningErrorCode::Trust))?;
            // ZIP-215 import permits noncanonical encodings. Preserve exact key
            // identity and refuse weak keys using the qualified library APIs.
            if key.is_weak()
                || key.to_edwards().compress().to_bytes() != public
                || trusted.insert(id, key).is_some()
            {
                return Err(SigningError(SigningErrorCode::Trust));
            }
        }
        Ok(Self(trusted))
    }

    pub fn verify_snapshot(
        &self,
        id: SigningKeyId,
        bytes: &[u8],
        signature: &[u8; 64],
    ) -> Result<(), SigningError> {
        let public = self
            .0
            .get(&id)
            .ok_or(SigningError(SigningErrorCode::Trust))?;
        public
            .verify_strict(&message(bytes)?, &Signature::from_bytes(signature))
            .map_err(|_| SigningError(SigningErrorCode::Signature))
    }
}

fn message(bytes: &[u8]) -> Result<Vec<u8>, SigningError> {
    if bytes.len() > MAX_SIGNED_BYTES {
        return Err(SigningError(SigningErrorCode::Size));
    }
    let mut message = Vec::with_capacity(DOMAIN.len() + bytes.len());
    message.extend_from_slice(DOMAIN);
    message.extend_from_slice(bytes);
    Ok(message)
}

/// Content fingerprint, never a password/credential verifier or authorization proof.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hash = [0; 32];
    hash.copy_from_slice(digest(&SHA256, bytes).as_ref());
    hash
}

#[cfg(test)]
mod tests;
