use std::{error::Error, fmt};

/// Opaque confidential material, deliberately absent from public identity records.
///
/// This is an ownership/redaction boundary, not a token parser, verifier, random
/// generator or encrypted store. P1-07/P3-07 must choose and validate wire format,
/// entropy, input bounds and lifecycle before processing external capabilities.
/// No Clone, Copy, Display or serialization implementation exposes it implicitly.
/// Memory erasure is not promised by this wrapper.
///
/// ```compile_fail
/// use droutex_common::ResolverCapabilityMaterial;
/// fn implicit_copy(secret: ResolverCapabilityMaterial) { let _ = secret.clone(); }
/// ```
///
/// ```compile_fail
/// use droutex_common::ResolverCapabilityMaterial;
/// fn accidental_log(secret: ResolverCapabilityMaterial) { println!("{secret}"); }
/// ```
pub struct ResolverCapabilityMaterial(Vec<u8>);

impl ResolverCapabilityMaterial {
    /// Take already allocated confidential material; reject the empty value.
    pub fn from_owned_bytes(bytes: Vec<u8>) -> Result<Self, EmptyResolverCapability> {
        if bytes.is_empty() {
            Err(EmptyResolverCapability)
        } else {
            Ok(Self(bytes))
        }
    }

    /// Explicit access for a future provisioning/verifier boundary; never log it.
    pub fn expose_secret(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for ResolverCapabilityMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ResolverCapabilityMaterial([REDACTED])")
    }
}

/// Empty input cannot carry resolver capability material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmptyResolverCapability;

impl fmt::Display for EmptyResolverCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("empty resolver capability material")
    }
}

impl Error for EmptyResolverCapability {}
