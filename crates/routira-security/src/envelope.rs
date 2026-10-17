use crate::{KeyId, MAX_ENVELOPE_BYTES, MAX_SECRET_BYTES, SecretError, SecretErrorCode};
use routira_common::SecretRefId;
use std::fmt;

pub(crate) const HEADER_BYTES: usize = 58;
pub(crate) const MAGIC: &[u8; 8] = b"RTSEC\0\x00\x01";
pub(crate) const AAD_DOMAIN: &[u8] = b"Routira secret record\0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SecretPurpose {
    ResolverVerifier = 1,
    ProxyOutboundCredential = 2,
}

impl TryFrom<u8> for SecretPurpose {
    type Error = SecretError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ResolverVerifier),
            2 => Ok(Self::ProxyOutboundCredential),
            _ => Err(SecretError(SecretErrorCode::InvalidEnvelope)),
        }
    }
}

/// The consumer obtains this context from trusted references, not from the envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SecretContext {
    pub reference: SecretRefId,
    pub purpose: SecretPurpose,
}

/// Structurally checked ciphertext. Parsing does not authenticate it.
/// Bytes may be persisted/exported as ciphertext; decryption is explicit on Keyring.
#[derive(Clone, Eq, PartialEq)]
pub struct EncryptedSecret {
    bytes: Vec<u8>,
    context: SecretContext,
    key: KeyId,
}

impl EncryptedSecret {
    pub fn from_owned_bytes(bytes: Vec<u8>) -> Result<Self, SecretError> {
        let invalid = || SecretError(SecretErrorCode::InvalidEnvelope);
        if !(HEADER_BYTES + 17..=MAX_ENVELOPE_BYTES).contains(&bytes.len())
            || &bytes[..8] != MAGIC
            || bytes[8] != 1
        {
            return Err(invalid());
        }
        let key = KeyId::from_bytes(bytes[9..25].try_into().map_err(|_| invalid())?)
            .map_err(|_| invalid())?;
        let reference = SecretRefId::from_bytes(bytes[25..41].try_into().map_err(|_| invalid())?)
            .map_err(|_| invalid())?;
        let purpose = SecretPurpose::try_from(bytes[41])?;
        let length = u32::from_be_bytes(bytes[54..58].try_into().map_err(|_| invalid())?) as usize;
        if !(1..=MAX_SECRET_BYTES).contains(&length) || bytes.len() != HEADER_BYTES + length + 16 {
            return Err(invalid());
        }
        Ok(Self {
            bytes,
            context: SecretContext { reference, purpose },
            key,
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn context(&self) -> SecretContext {
        self.context
    }
    pub fn key_id(&self) -> KeyId {
        self.key
    }

    pub(crate) fn header(&self) -> &[u8] {
        &self.bytes[..HEADER_BYTES]
    }
    pub(crate) fn ciphertext(&self) -> &[u8] {
        &self.bytes[HEADER_BYTES..]
    }
    pub(crate) fn nonce(&self) -> Result<[u8; 12], SecretError> {
        self.bytes[42..54]
            .try_into()
            .map_err(|_| SecretError(SecretErrorCode::InvalidEnvelope))
    }
}

impl fmt::Debug for EncryptedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptedSecret")
            .field("context", &self.context)
            .field("key_id", &self.key)
            .finish_non_exhaustive()
    }
}
