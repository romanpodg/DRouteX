//! Trusted control-plane revision metadata, separate from imported configuration.

use droutex_common::{ConfigurationRevisionId, CorrelationId, UserId};
use std::{error::Error, fmt, time::SystemTime};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidRevisionMetadata {
    SelfParent,
    Timestamp,
}

impl fmt::Display for InvalidRevisionMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SelfParent => "revision cannot be its own parent",
            Self::Timestamp => "revision timestamp is outside supported Unix milliseconds",
        })
    }
}

impl Error for InvalidRevisionMetadata {}

/// Trusted caller metadata, not an authenticated user or a signature.
/// Parent links determine edit order; wall-clock timestamps need not increase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DraftRevision {
    id: ConfigurationRevisionId,
    parent: Option<ConfigurationRevisionId>,
    author: UserId,
    unix_millis: i64,
    correlation: CorrelationId,
}

impl DraftRevision {
    pub fn new(
        id: ConfigurationRevisionId,
        parent: Option<ConfigurationRevisionId>,
        author: UserId,
        timestamp: SystemTime,
        correlation: CorrelationId,
    ) -> Result<Self, InvalidRevisionMetadata> {
        let millis = timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| InvalidRevisionMetadata::Timestamp)?
            .as_millis();
        let millis = i64::try_from(millis).map_err(|_| InvalidRevisionMetadata::Timestamp)?;
        Self::from_unix_millis(id, parent, author, millis, correlation)
    }

    pub fn from_unix_millis(
        id: ConfigurationRevisionId,
        parent: Option<ConfigurationRevisionId>,
        author: UserId,
        unix_millis: i64,
        correlation: CorrelationId,
    ) -> Result<Self, InvalidRevisionMetadata> {
        if parent == Some(id) {
            return Err(InvalidRevisionMetadata::SelfParent);
        }
        if unix_millis < 0 {
            return Err(InvalidRevisionMetadata::Timestamp);
        }
        Ok(Self {
            id,
            parent,
            author,
            unix_millis,
            correlation,
        })
    }

    pub fn id(&self) -> ConfigurationRevisionId {
        self.id
    }
    pub fn parent(&self) -> Option<ConfigurationRevisionId> {
        self.parent
    }
    pub fn author(&self) -> UserId {
        self.author
    }
    pub fn unix_millis(&self) -> i64 {
        self.unix_millis
    }
    pub fn correlation(&self) -> CorrelationId {
        self.correlation
    }
}

/// Old persisted records have no invented author, time, parent or correlation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevisionProvenance {
    LegacyUnknown,
    Recorded(DraftRevision),
}
