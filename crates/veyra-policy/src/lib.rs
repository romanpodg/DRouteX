//! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).

mod model;
mod name;
mod pattern;

pub use model::*;
pub use name::{Name, NameError};
pub use pattern::{Pattern, PatternError, Specificity};
