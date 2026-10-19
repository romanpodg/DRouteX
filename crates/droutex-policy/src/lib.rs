//! Pure policy contracts: no database, network, filesystem or OS operations.

use droutex_common::RuleLayer;

mod domain;
pub use domain::{
    CanonicalDomain, DomainError, DomainMatchMode, DomainMatcher, MAX_DOMAIN_INPUT_BYTES,
};

/// Accepted CR-0002 precedence, from highest priority to fallback.
///
/// Within-layer priority and conflict detection remain later P2 steps.
pub const RULE_LAYER_ORDER: [RuleLayer; 4] = [
    RuleLayer::Custom,
    RuleLayer::Service,
    RuleLayer::Filter,
    RuleLayer::Default,
];
