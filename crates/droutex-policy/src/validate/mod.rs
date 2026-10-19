//! D39 offline validation components. P2-05 must bind all checks to the same
//! compiled generation; none is signing, catalog import or runtime authority.
mod catalog;
mod conflicts;
mod graph;
mod private;

use crate::{EvaluationError, PolicySet, RuleSource};
pub use catalog::*;
use droutex_common::ProfileId;
pub use private::*;
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyPlane {
    Dns,
    Tcp,
}

/// Stable source identities only; no domain, effect or confidential input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicyValidationError {
    pub profile: ProfileId,
    pub plane: PolicyPlane,
    pub first: RuleSource,
    pub second: RuleSource,
}

impl fmt::Display for PolicyValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("reachable contradictory policy effects prevent publication")
    }
}
impl Error for PolicyValidationError {}

impl<D: Eq, T: Eq> PolicySet<D, T> {
    /// Symbolically check every reachable DNS name/TCP port in the prepared set.
    /// Entirely shadowed conflicts are permitted; IDs never resolve a conflict.
    /// The future compiler must also validate current config references, private
    /// records and independently verified catalog dependencies for this generation.
    pub fn validate_conflicts(&self) -> Result<(), PolicyValidationError> {
        for (id, profile) in &self.profiles {
            for (plane, conflict) in [
                (PolicyPlane::Dns, conflicts::dns(&profile.dns_rules)),
                (PolicyPlane::Tcp, conflicts::tcp(&profile.tcp_rules)),
            ] {
                if let Some(EvaluationError::Ambiguous { first, second }) = conflict {
                    return Err(PolicyValidationError {
                        profile: *id,
                        plane,
                        first,
                        second,
                    });
                }
            }
        }
        Ok(())
    }
}
