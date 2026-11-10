//! Gateway safety components, a fail-closed default listener and an opt-in preview.
pub mod listener;
pub mod origin;

/// Experimental parsing; traffic additionally requires the explicit preview API.
#[cfg(feature = "experimental-tls")]
pub mod tls_metadata;
