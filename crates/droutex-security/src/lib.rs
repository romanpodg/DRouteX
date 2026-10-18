//! Bounded encrypted records and externally provisioned keys. No caller authentication,
//! token verification, anti-rollback authority or plaintext export interface.
mod envelope;
mod keyring;
mod paths;

pub use envelope::{EncryptedSecret, SecretContext, SecretPurpose};
pub use keyring::{KeyId, Keyring, MAX_KEYS};

use std::{error::Error, fmt};
use zeroize::Zeroizing;

/// Accepted maximum plaintext bytes per record.
pub const MAX_SECRET_BYTES: usize = 65_536;
/// Header58 + payload + tag16.
pub const MAX_ENVELOPE_BYTES: usize = 58 + MAX_SECRET_BYTES + 16;

/// Safe boundary reasons. Never include plaintext, key bytes or raw I/O diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretErrorCode {
    InvalidMaterial,
    InvalidKeyring,
    InvalidEnvelope,
    ContextMismatch,
    MissingKey,
    Authentication,
    RandomUnavailable,
    UnsafePath,
    Filesystem,
    UnsupportedPlatform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SecretError(SecretErrorCode);

impl SecretError {
    pub const fn code(self) -> SecretErrorCode {
        self.0
    }
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "secret {:?}", self.0)
    }
}
impl Error for SecretError {}

/// Owned confidential bytes. Explicit access is for trusted consumers only.
/// Caller copies, crypto internals and OS memory are outside its erasure promise.
///
/// ```compile_fail
/// use droutex_security::SecretBytes;
/// fn copy(value: SecretBytes) { let _ = value.clone(); }
/// ```
/// ```compile_fail
/// use droutex_security::SecretBytes;
/// fn log(value: SecretBytes) { println!("{value}"); }
/// ```
pub struct SecretBytes(Zeroizing<Vec<u8>>);

impl SecretBytes {
    pub fn from_owned_bytes(bytes: Vec<u8>) -> Result<Self, SecretError> {
        let bytes = Zeroizing::new(bytes);
        if bytes.is_empty() || bytes.len() > MAX_SECRET_BYTES {
            return Err(SecretError(SecretErrorCode::InvalidMaterial));
        }
        Ok(Self(bytes))
    }

    pub fn expose_secret(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretBytes([REDACTED])")
    }
}

#[cfg(test)]
mod tests;
