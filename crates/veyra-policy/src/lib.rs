//! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).

mod compile;
mod decide;
mod model;
mod name;
mod pattern;

pub use compile::{Issue, IssueCode, Policy, Rule, Severity};
pub use decide::{DecidedBy, Decision, Subject, SubjectError};
pub use model::*;
pub use name::{Name, NameError};
pub use pattern::{Pattern, PatternError, Specificity};
