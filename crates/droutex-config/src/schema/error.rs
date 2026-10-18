use std::{error::Error, fmt};

/// Stable boundary reason, independent of a parser's diagnostic text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigErrorCode {
    InputTooLarge,
    InvalidUtf8,
    InvalidDocument,
    InvalidShape,
    UnsupportedVersion,
    ResourceLimit,
    DuplicateId,
    UnknownReference,
    InvalidValue,
    InvalidScope,
    AmbiguousGrant,
    BindingConflict,
}

/// Sanitized input failure: no original input, parser error or source chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigError {
    code: ConfigErrorCode,
    field: String,
    location: Option<(usize, usize)>,
}

impl ConfigError {
    pub(crate) fn at(code: ConfigErrorCode, field: String) -> Self {
        Self {
            code,
            field,
            location: None,
        }
    }

    pub(crate) fn document(location: Option<(usize, usize)>) -> Self {
        Self {
            code: ConfigErrorCode::InvalidDocument,
            field: "$".to_owned(),
            location,
        }
    }

    pub fn code(&self) -> ConfigErrorCode {
        self.code
    }
    pub fn field(&self) -> &str {
        &self.field
    }
    /// Parser-supplied source location when available; column units follow the parser.
    pub fn location(&self) -> Option<(usize, usize)> {
        self.location
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {}", self.code, self.field)?;
        if let Some((line, column)) = self.location {
            write!(f, " ({line}:{column})")?;
        }
        Ok(())
    }
}

impl Error for ConfigError {}

pub(crate) fn safe_path(path: &serde_path_to_error::Path) -> String {
    let mut result = String::from("$");
    for segment in path {
        match segment {
            serde_path_to_error::Segment::Seq { index } => result.push_str(&format!("[{index}]")),
            serde_path_to_error::Segment::Map { key } if FIELDS.contains(&key.as_str()) => {
                result.push('.');
                result.push_str(key);
            }
            _ => break,
        }
    }
    result
}

const FIELDS: &[&str] = &[
    "schemaVersion",
    "profiles",
    "endpoints",
    "resolverCredentials",
    "listeners",
    "networkGroups",
    "legacyGrants",
    "gatewayGrants",
    "outbounds",
    "privateRecords",
    "id",
    "name",
    "enabled",
    "profile",
    "device",
    "access",
    "transport",
    "listener",
    "credentials",
    "grants",
    "endpoint",
    "state",
    "bind",
    "dualStack",
    "trafficProfile",
    "source",
    "group",
    "targets",
    "services",
    "ports",
    "outbound",
    "outbounds",
    "kind",
    "host",
    "port",
    "tlsServerName",
    "secret",
    "defaultDnsAction",
    "defaultTrafficAction",
    "customRules",
    "serviceRules",
    "filterLists",
    "priority",
    "matcher",
    "mode",
    "value",
    "action",
    "plane",
    "upstream",
    "revision",
    "from",
    "to",
    "record",
    "address",
    "target",
    "ttl",
];
