//! Configuration validation against accepted policy contracts; no wire format yet.

use std::fmt;

use routira_common::RuleLayer;
use routira_policy::RULE_LAYER_ORDER;

/// The supplied layers do not match the accepted, fixed precedence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidLayerOrder;

impl fmt::Display for InvalidLayerOrder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("rule layers must be custom, service, filter, default")
    }
}

impl std::error::Error for InvalidLayerOrder {}

/// Reject reordered, missing, duplicate or extra layers instead of changing policy.
pub fn validate_rule_layers(layers: &[RuleLayer]) -> Result<(), InvalidLayerOrder> {
    if layers == RULE_LAYER_ORDER {
        Ok(())
    } else {
        Err(InvalidLayerOrder)
    }
}
