mod access;
mod binding;
mod error;
mod model;
mod parser;
mod rules;
mod validation;
mod value;

pub use error::{ConfigError, ConfigErrorCode};
pub use model::*;
pub use parser::{CheckedConfiguration, InputFormat, parse_configuration};

/// Owner-approved maximum bytes before any configuration parser is invoked.
pub const MAX_INPUT_BYTES: usize = 1_048_576;
/// Owner-approved maximum nested collections, including the root collection.
pub const MAX_DEPTH: usize = 32;
/// Owner-approved aggregate resource/rule count, excluding external datasets.
pub const MAX_RESOURCE_ENTRIES: usize = 10_000;
