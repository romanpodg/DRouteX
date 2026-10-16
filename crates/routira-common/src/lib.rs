//! Shared policy vocabulary. Resolver credentials and gateway identity are separate concerns.

/// A policy layer; user priority applies within a layer, never across layers.
///
/// Declaration order is not precedence. Use `routira_policy::RULE_LAYER_ORDER`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleLayer {
    /// Operator-authored rules.
    Custom,
    /// Rules for an explicitly selected service revision.
    Service,
    /// Rules from selected filter-list revisions.
    Filter,
    /// The fallback when no higher layer selects an action.
    Default,
}
