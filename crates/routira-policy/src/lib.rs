//! Pure policy contracts: no database, network, filesystem or OS operations.

use routira_common::RuleLayer;

/// Accepted CR-0002 precedence, from highest priority to fallback.
///
/// Rule matching, within-layer priority and conflict detection belong to P2.
pub const RULE_LAYER_ORDER: [RuleLayer; 4] = [
    RuleLayer::Custom,
    RuleLayer::Service,
    RuleLayer::Filter,
    RuleLayer::Default,
];
