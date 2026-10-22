use serde::{Deserialize, Deserializer, Serialize};
use std::{error::Error, fmt};

/// Largest accepted Unix-second timestamp (9999-12-31T23:59:59Z).
pub const MAX_GRANT_TIMESTAMP: u64 = 253_402_300_799;

/// Validated signed intent, never proof of current source or live permission.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "WireLease", into = "WireLease")]
pub struct GrantLease {
    issued_at: u64,
    expires_at: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGrantLease;
impl fmt::Display for InvalidGrantLease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid bounded manual grant lease")
    }
}
impl Error for InvalidGrantLease {}

impl GrantLease {
    pub fn manual(issued_at: u64, expires_at: u64) -> Result<Self, InvalidGrantLease> {
        let lifetime = expires_at.checked_sub(issued_at).ok_or(InvalidGrantLease)?;
        if expires_at > MAX_GRANT_TIMESTAMP || !(60..=86_400).contains(&lifetime) {
            return Err(InvalidGrantLease);
        }
        Ok(Self {
            issued_at,
            expires_at,
        })
    }
    pub const fn issued_at(self) -> u64 {
        self.issued_at
    }
    pub const fn expires_at(self) -> u64 {
        self.expires_at
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Cause {
    Manual,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireLease {
    issued_at: u64,
    expires_at: u64,
    cause: Cause,
}
impl TryFrom<WireLease> for GrantLease {
    type Error = InvalidGrantLease;
    fn try_from(value: WireLease) -> Result<Self, Self::Error> {
        Self::manual(value.issued_at, value.expires_at)
    }
}
impl From<GrantLease> for WireLease {
    fn from(value: GrantLease) -> Self {
        Self {
            issued_at: value.issued_at,
            expires_at: value.expires_at,
            cause: Cause::Manual,
        }
    }
}

/// Absence retains v1 bytes; explicit null is not a lease and is rejected.
pub(super) fn optional_lease<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<GrantLease>, D::Error> {
    GrantLease::deserialize(d).map(Some)
}
