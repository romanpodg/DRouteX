use routira_config::ConfigError;
use std::{error::Error, fmt};

/// Stable storage boundary reasons; raw database/path/SQL diagnostics are discarded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageErrorCode {
    InvalidConfiguration,
    InvalidCredentialLink,
    InvalidProvenance,
    InvalidSecret,
    QueueFull,
    DeadlineOutcomeUnknown,
    Busy,
    DiskFull,
    Conflict,
    CorruptData,
    Migration,
    UnsafePath,
    Filesystem,
    UnsupportedPlatform,
    RuntimeUnavailable,
    Closed,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageError {
    code: StorageErrorCode,
    configuration: Option<ConfigError>,
}

impl StorageError {
    pub(crate) const fn new(code: StorageErrorCode) -> Self {
        Self {
            code,
            configuration: None,
        }
    }

    pub(crate) fn configuration(error: ConfigError) -> Self {
        Self {
            code: StorageErrorCode::InvalidConfiguration,
            configuration: Some(error),
        }
    }

    pub fn code(&self) -> StorageErrorCode {
        self.code
    }
    pub fn configuration_error(&self) -> Option<&ConfigError> {
        self.configuration.as_ref()
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "storage {:?}", self.code)?;
        if let Some(error) = &self.configuration {
            write!(f, ": {error}")?;
        }
        Ok(())
    }
}

impl Error for StorageError {}

pub(crate) fn database(error: sqlx::Error) -> StorageError {
    let code = error
        .as_database_error()
        .and_then(|e| e.code())
        .and_then(|code| code.parse::<u32>().ok());
    StorageError::new(match code.map(|value| value & 0xff) {
        Some(5 | 6) => StorageErrorCode::Busy,
        Some(13) => StorageErrorCode::DiskFull,
        Some(19) => StorageErrorCode::Conflict,
        Some(11 | 26) => StorageErrorCode::CorruptData,
        _ => StorageErrorCode::Unavailable,
    })
}
