//! Resource identity and lifecycle data, never runtime admission or cryptographic proof.

mod identity;
mod ids;
mod secret;
mod state;

pub use identity::*;
pub use ids::*;
pub use secret::{EmptyResolverCapability, ResolverCapabilityMaterial};
pub use state::*;
