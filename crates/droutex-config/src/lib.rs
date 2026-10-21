//! Bounded configuration import and validation against accepted policy contracts.

mod actions;
pub mod catalog;
mod publication;
pub use actions::{
    ActionPreparationError, DnsActionResolution, InvalidActionScope, OutboundObservation,
    TcpActionResolution, resolve_dns_action, resolve_tcp_action,
};
mod revisions;
pub use publication::PublicationResourceError;
mod schema;
pub mod snapshot;
pub use revisions::{DraftRevision, InvalidRevisionMetadata, RevisionProvenance};
pub use schema::{
    CheckedConfiguration, ConfigError, ConfigErrorCode, Configuration, CredentialMetadata,
    CredentialState, CustomRule, DnsAccess, DnsAction, DnsEndpoint, DomainMatch, FilterSelection,
    GatewayGrant, InputFormat, LegacyGrant, Listener, ListenerTransport, MAX_DEPTH,
    MAX_INPUT_BYTES, MAX_RESOURCE_ENTRIES, MatchMode, NetworkGroup, OutboundKind, PortRange,
    PrivateRecord, Profile, ProxyOutbound, RecordData, ReferenceCatalog, RuleAction, ServiceRule,
    TrafficAction, parse_configuration,
};

use std::fmt;

use droutex_common::RuleLayer;
use droutex_policy::RULE_LAYER_ORDER;

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
