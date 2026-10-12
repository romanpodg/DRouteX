//! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).

mod compile;
mod model;
mod name;
mod pattern;

pub use compile::{Issue, IssueCode, Policy, Rule, Severity};
pub use model::*;
pub use name::{Name, NameError};
pub use pattern::{Pattern, PatternError, Specificity};
