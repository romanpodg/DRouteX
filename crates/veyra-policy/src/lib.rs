//! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).

mod admission;
mod compile;
mod decide;
mod dns_config;
mod model;
mod name;
mod pattern;

pub use admission::{Admitted, Cidr};
pub use compile::{Issue, IssueCode, Policy, Rule, Severity};
pub use decide::{DecidedBy, Decision, Subject, SubjectError};
pub use dns_config::{DEFAULT_UPSTREAM_GROUP, DnsConfig};
pub use model::*;
pub use name::{Name, NameError};
pub use pattern::{Pattern, PatternError, Specificity};
