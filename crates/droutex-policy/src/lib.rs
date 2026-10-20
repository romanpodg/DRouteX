//! Pure policy contracts: no database, network, filesystem or OS operations.

use droutex_common::RuleLayer;

mod decision;
mod domain;
pub use decision::{
    ActionInventoryError, ActionRefusal, ActionResolver, DecisionPoint, DnsIntent,
    GatewayReadiness, LocalAnswerIntent, OutboundFacts, RequestTransport, TcpCapability, TcpIntent,
    local_answer_intent,
};
mod evaluate;
mod validate;
pub use domain::{
    CanonicalDomain, DomainError, DomainMatchMode, DomainMatcher, MAX_DOMAIN_INPUT_BYTES,
};
pub use evaluate::{
    DomainRule, EvaluationError, GatewayProfileAssignment, MAX_POLICY_ENTRIES, PolicyBuildError,
    PolicyScope, PolicySelection, PolicySet, PreparedProfile, RuleMetadata, RuleSource,
    TcpPortRange, TcpPortScope, TcpRule,
};
pub use validate::{
    CatalogIssue, CatalogLicense, CatalogPin, CatalogProvenance, CatalogRevision,
    CatalogRevisionFacts, CatalogValidationError, CatalogValidationState, PolicyPlane,
    PolicyValidationError, PreparedPrivateRecord, PrivateRecordIssue, PrivateRecordValidationError,
    PrivateRecordValue, validate_catalog_dependencies, validate_private_records,
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
