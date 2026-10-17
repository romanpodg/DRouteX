use crate::{EncryptedSecret, SecretBytes, SecretContext, SecretError, SecretErrorCode, envelope};
use aws_lc_rs::{
    aead::{AES_256_GCM_SIV, Aad, LessSafeKey, Nonce, UnboundKey},
    rand::fill,
};
use std::{collections::BTreeMap, fmt, path::Path};
use zeroize::Zeroizing;

pub const MAX_KEYS: usize = 32;
pub(crate) const KEYRING_HEADER: usize = 25;
pub(crate) const MAX_KEYRING_BYTES: usize = KEYRING_HEADER + MAX_KEYS * 48;
const MAGIC: &[u8; 8] = b"RTKEY\0\x00\x01";

/// Public opaque identifier, never the key itself. Never rebind it to another key.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct KeyId([u8; 16]);

impl KeyId {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, SecretError> {
        if bytes == [0; 16] {
            return Err(SecretError(SecretErrorCode::InvalidKeyring));
        }
        Ok(Self(bytes))
    }
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// One immutable provisioned ring. Replacing its owning instance is an explicit reload.
/// It neither writes nor generates key files. No plaintext serialization or Clone.
pub struct Keyring {
    active: KeyId,
    keys: BTreeMap<KeyId, Zeroizing<[u8; 32]>>,
}

impl Keyring {
    /// Takes ownership before parsing so errors also wipe the input buffer.
    pub fn from_owned_bytes(bytes: Vec<u8>) -> Result<Self, SecretError> {
        let bytes = Zeroizing::new(bytes);
        let invalid = || SecretError(SecretErrorCode::InvalidKeyring);
        if !(KEYRING_HEADER + 48..=MAX_KEYRING_BYTES).contains(&bytes.len()) || &bytes[..8] != MAGIC
        {
            return Err(invalid());
        }
        let count = usize::from(bytes[8]);
        if !(1..=MAX_KEYS).contains(&count) || bytes.len() != KEYRING_HEADER + count * 48 {
            return Err(invalid());
        }
        let active = KeyId::from_bytes(bytes[9..25].try_into().map_err(|_| invalid())?)?;
        let mut keys = BTreeMap::new();
        let mut previous = None;
        for entry in bytes[KEYRING_HEADER..].as_chunks::<48>().0 {
            let id = KeyId::from_bytes(entry[..16].try_into().map_err(|_| invalid())?)?;
            if previous.is_some_and(|previous| previous >= id) {
                return Err(invalid());
            }
            let mut material = Zeroizing::new([0; 32]);
            material.copy_from_slice(&entry[16..]);
            keys.insert(id, material);
            previous = Some(id);
        }
        if !keys.contains_key(&active) {
            return Err(invalid());
        }
        Ok(Self { active, keys })
    }

    /// Blocking Linux file loader; caller supplies existing trusted exclusion roots.
    /// Async consumers must move this work off their executor thread.
    pub fn load_file(
        path: &Path,
        database_root: &Path,
        backup_root: &Path,
    ) -> Result<Self, SecretError> {
        Self::from_owned_bytes(crate::paths::read(path, database_root, backup_root)?)
    }

    pub fn active_key_id(&self) -> KeyId {
        self.active
    }

    fn cipher(&self, id: KeyId) -> Result<LessSafeKey, SecretError> {
        let material = self
            .keys
            .get(&id)
            .ok_or(SecretError(SecretErrorCode::MissingKey))?;
        let key = UnboundKey::new(&AES_256_GCM_SIV, material.as_ref())
            .map_err(|_| SecretError(SecretErrorCode::InvalidKeyring))?;
        Ok(LessSafeKey::new(key))
    }

    pub fn seal(
        &self,
        context: SecretContext,
        secret: SecretBytes,
    ) -> Result<EncryptedSecret, SecretError> {
        let mut nonce = [0; 12];
        fill(&mut nonce).map_err(|_| SecretError(SecretErrorCode::RandomUnavailable))?;
        self.seal_with_nonce(context, secret, nonce)
    }

    // Fixed nonces are private, used only by golden tests. Public writes use OS randomness.
    pub(crate) fn seal_with_nonce(
        &self,
        context: SecretContext,
        secret: SecretBytes,
        nonce: [u8; 12],
    ) -> Result<EncryptedSecret, SecretError> {
        let SecretBytes(mut payload) = secret;
        let mut bytes = Vec::with_capacity(envelope::HEADER_BYTES + payload.len() + 16);
        bytes.extend_from_slice(envelope::MAGIC);
        bytes.push(1);
        bytes.extend_from_slice(self.active.as_bytes());
        bytes.extend_from_slice(context.reference.as_bytes());
        bytes.push(context.purpose as u8);
        bytes.extend_from_slice(&nonce);
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        let aad = [envelope::AAD_DOMAIN, bytes.as_slice()].concat();
        // Separate tag avoids reallocating/moving the owned plaintext buffer.
        let tag = self
            .cipher(self.active)?
            .seal_in_place_separate_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(&aad),
                payload.as_mut_slice(),
            )
            .map_err(|_| SecretError(SecretErrorCode::Authentication))?;
        bytes.extend_from_slice(&payload);
        bytes.extend_from_slice(tag.as_ref());
        EncryptedSecret::from_owned_bytes(bytes)
    }

    pub fn open(
        &self,
        expected: SecretContext,
        record: &EncryptedSecret,
    ) -> Result<SecretBytes, SecretError> {
        if record.context() != expected {
            return Err(SecretError(SecretErrorCode::ContextMismatch));
        }
        let key = self.cipher(record.key_id())?;
        let aad = [envelope::AAD_DOMAIN, record.header()].concat();
        let mut payload = Zeroizing::new(record.ciphertext().to_vec());
        let length = key
            .open_in_place(
                Nonce::assume_unique_for_key(record.nonce()?),
                Aad::from(&aad),
                payload.as_mut_slice(),
            )
            .map_err(|_| SecretError(SecretErrorCode::Authentication))?
            .len();
        payload.truncate(length);
        Ok(SecretBytes(payload))
    }

    pub fn reencrypt(
        &self,
        expected: SecretContext,
        record: &EncryptedSecret,
    ) -> Result<EncryptedSecret, SecretError> {
        self.seal(expected, self.open(expected, record)?)
    }
}

impl fmt::Debug for Keyring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Keyring([REDACTED])")
    }
}
