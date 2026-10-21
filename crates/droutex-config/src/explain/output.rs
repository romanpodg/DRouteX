use super::SimulationError;
use crate::{CompiledConfiguration, DnsAction, TrafficAction};
use droutex_common::AdministrativeState;
use droutex_policy::{
    ActionRefusal, CanonicalDomain, CatalogRevision, DecisionPoint, DnsIntent, DomainMatchMode,
    PolicyScope, PolicyTrace, PrivateRecordValue, RequestTransport, RuleSource, TcpIntent,
    TraceDisposition,
};
use serde::Serialize;
use std::{fmt, net::IpAddr, num::NonZeroU16};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    Present,
    Absent,
    Unknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticContext {
    /// Supplied flow context only; a known target does not prove ECH recovery.
    pub ech: Evidence,
    /// Whether this simulated flow uses QUIC. Present contradicts TCP transport.
    pub quic: Evidence,
}
pub(super) struct TcpContext {
    pub(super) flow: DiagnosticContext,
    pub(super) source: IpAddr,
    pub(super) port: NonZeroU16,
    pub(super) transport: RequestTransport,
}
impl Default for DiagnosticContext {
    fn default() -> Self {
        Self {
            ech: Evidence::Unknown,
            quic: Evidence::Unknown,
        }
    }
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Source {
    Default,
    Custom {
        rule: String,
    },
    Service {
        rule: String,
        revision: String,
        resource_hash: String,
    },
    Filter {
        revision: String,
        resource_hash: String,
    },
}
#[derive(Serialize)]
struct Entry {
    source: Source,
    layer: &'static str,
    priority: i32,
    labels: usize,
    domain: String,
    mode: &'static str,
    enabled: bool,
    domain_matches: bool,
    port_matches: Option<bool>,
    port_range: Option<(u16, u16)>,
    disposition: &'static str,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PredictedAction {
    DnsBlock,
    DnsResolve { upstream: String },
    DnsGateway { outbound: String },
    TcpDeny,
    TcpOutbound { outbound: String },
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Intent {
    DnsBlock,
    Resolve {
        upstream: String,
    },
    Gateway {
        outbound: String,
    },
    GatewayDirect {
        outbound: String,
    },
    ProtectedProxy {
        outbound: String,
    },
    Refused {
        code: &'static str,
        reference: Option<String>,
    },
}
#[derive(Serialize)]
struct PrivateOwner {
    id: String,
    record_type: &'static str,
}
/// Allow-listed unbound selection diagnostic, not a complete generation report.
/// No constructor/Deserialize; it cannot become an admission or visibility token.
#[derive(Serialize)]
pub struct SelectionExplanation {
    plane: &'static str,
    scope: String,
    profile: String,
    canonical_domain: String,
    destination_port: Option<u16>,
    transport: &'static str,
    selected_source: Source,
    predicted_action: PredictedAction,
    resolved_intent: Intent,
    predicted_decision_point: Option<&'static str>,
    observation_basis: &'static str,
    context: DiagnosticContext,
    limitations: Vec<&'static str>,
    private_owners: Vec<PrivateOwner>,
    private_response_precedence: &'static str,
    inspected_entries: usize,
    emitted_entries: usize,
    trace: Vec<Entry>,
}
impl fmt::Debug for SelectionExplanation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectionExplanation")
            .field("entries", &self.trace.len())
            .finish_non_exhaustive()
    }
}
fn hash(
    compiled: &CompiledConfiguration,
    revision: CatalogRevision,
) -> Result<String, SimulationError> {
    let pin = compiled
        .catalog_pin(revision)
        .ok_or(SimulationError::Inconsistent)?;
    Ok(pin.hash.iter().map(|b| format!("{b:02x}")).collect())
}
fn source(
    compiled: &CompiledConfiguration,
    source: Option<RuleSource>,
) -> Result<Source, SimulationError> {
    Ok(match source {
        None => Source::Default,
        Some(RuleSource::Custom(rule)) => Source::Custom {
            rule: rule.to_string(),
        },
        Some(RuleSource::Service { rule, revision }) => Source::Service {
            rule: rule.to_string(),
            revision: revision.to_string(),
            resource_hash: hash(compiled, CatalogRevision::Service(revision))?,
        },
        Some(RuleSource::Filter(revision)) => Source::Filter {
            revision: revision.to_string(),
            resource_hash: hash(compiled, CatalogRevision::Filter(revision))?,
        },
    })
}
fn entries<A>(
    compiled: &CompiledConfiguration,
    trace: &PolicyTrace<'_, A>,
) -> Result<Vec<Entry>, SimulationError> {
    trace
        .entries()
        .iter()
        .map(|e| {
            Ok(Entry {
                source: source(compiled, Some(e.source()))?,
                layer: match e.source() {
                    RuleSource::Custom(_) => "custom",
                    RuleSource::Service { .. } => "service",
                    RuleSource::Filter(_) => "filter",
                },
                priority: e.priority(),
                labels: e.matcher().domain().as_str().split('.').count(),
                domain: e.matcher().domain().as_str().to_owned(),
                mode: match e.matcher().mode() {
                    DomainMatchMode::Exact => "exact",
                    DomainMatchMode::SubdomainsOnly => "subdomains_only",
                    DomainMatchMode::DomainAndSubdomains => "domain_and_subdomains",
                },
                enabled: e.state() == AdministrativeState::Enabled,
                domain_matches: e.domain_matches(),
                port_matches: e.port_matches(),
                port_range: e.port_range(),
                disposition: match e.disposition() {
                    TraceDisposition::Disabled => "disabled",
                    TraceDisposition::DomainMismatch => "domain_mismatch",
                    TraceDisposition::PortMismatch => "port_mismatch",
                    TraceDisposition::Selected => "selected",
                    TraceDisposition::EquivalentTie => "equivalent_tie",
                    TraceDisposition::LowerRank => "lower_rank",
                },
            })
        })
        .collect()
}
fn point(point: Option<DecisionPoint>) -> Option<&'static str> {
    point.map(|p| match p {
        DecisionPoint::ClientOrigin => "client_origin",
        DecisionPoint::GatewayVps => "gateway_vps",
        DecisionPoint::ProtectedProxyExit => "protected_proxy_exit",
    })
}
fn refused(reason: ActionRefusal) -> Intent {
    let (code, reference) = match reason {
        ActionRefusal::PolicyDenied => ("policy_denied", None),
        ActionRefusal::UnknownUpstream(id) => ("unknown_upstream", Some(id.to_string())),
        ActionRefusal::UnknownOutbound(id) => ("unknown_outbound", Some(id.to_string())),
        ActionRefusal::DisabledOutbound(id) => ("disabled_outbound", Some(id.to_string())),
        ActionRefusal::UnavailableOutbound(id) => ("unavailable_outbound", Some(id.to_string())),
        ActionRefusal::UnknownAvailability(id) => ("unknown_availability", Some(id.to_string())),
        ActionRefusal::UnsupportedCapability(id) => {
            ("unsupported_capability", Some(id.to_string()))
        }
        ActionRefusal::UnknownCapability(id) => ("unknown_capability", Some(id.to_string())),
        ActionRefusal::ConnectionFailed(id) => ("connection_failed", Some(id.to_string())),
        ActionRefusal::UnsupportedTransport => ("unsupported_transport", None),
        ActionRefusal::GatewayNotReady => ("gateway_not_ready", None),
        ActionRefusal::PrivateProfileMismatch => ("private_profile_mismatch", None),
        ActionRefusal::PrivateNameMismatch => ("private_name_mismatch", None),
    };
    Intent::Refused { code, reference }
}
fn base<A>(
    compiled: &CompiledConfiguration,
    trace: &PolicyTrace<'_, A>,
    query: &CanonicalDomain,
    context: DiagnosticContext,
    predicted_action: PredictedAction,
    resolved_intent: Intent,
    decision_point: Option<DecisionPoint>,
) -> Result<SelectionExplanation, SimulationError> {
    let selection = trace.selection();
    let (plane, scope) = match selection.scope() {
        PolicyScope::DnsEndpoint(id) => ("dns", id.to_string()),
        PolicyScope::GatewayGroup(id) => ("tcp", id.to_string()),
    };
    let entries = entries(compiled, trace)?;
    Ok(SelectionExplanation {
        plane,
        scope,
        profile: selection.profile().to_string(),
        canonical_domain: query.as_str().to_owned(),
        destination_port: None,
        transport: "not_applicable",
        selected_source: source(compiled, selection.source())?,
        predicted_action,
        resolved_intent,
        predicted_decision_point: point(decision_point),
        observation_basis: "caller_supplied_simulation_assumptions",
        context,
        limitations: vec![
            "no_network_test",
            "ingress_not_authenticated",
            "destination_not_checked_or_pinned",
            "individual_device_not_authenticated",
        ],
        private_owners: vec![],
        private_response_precedence: "not_applicable",
        inspected_entries: entries.len(),
        emitted_entries: entries.len(),
        trace: entries,
    })
}
pub(super) fn dns(
    compiled: &CompiledConfiguration,
    trace: PolicyTrace<'_, DnsAction>,
    query: &CanonicalDomain,
    intent: DnsIntent,
    context: DiagnosticContext,
) -> Result<SelectionExplanation, SimulationError> {
    let predicted = match trace.selection().action() {
        DnsAction::Block {} => PredictedAction::DnsBlock,
        DnsAction::Resolve { upstream } => PredictedAction::DnsResolve {
            upstream: upstream.to_string(),
        },
        DnsAction::Gateway { outbound } => PredictedAction::DnsGateway {
            outbound: outbound.to_string(),
        },
    };
    let resolved = match intent {
        DnsIntent::Block => Intent::DnsBlock,
        DnsIntent::Resolve(id) => Intent::Resolve {
            upstream: id.to_string(),
        },
        DnsIntent::Gateway(id) => Intent::Gateway {
            outbound: id.to_string(),
        },
        DnsIntent::Refuse(r) => refused(r),
    };
    let mut result = base(
        compiled,
        &trace,
        query,
        context,
        predicted,
        resolved,
        intent.decision_point(),
    )?;
    result.limitations.extend([
        "dns_only_ech_quic_bypass_route_not_guaranteed",
        "no_ip_family_authorization_assumed",
    ]);
    result.private_owners = compiled
        .private_records()
        .iter()
        .filter(|p| p.profile == trace.selection().profile() && p.owner == *query)
        .map(|p| PrivateOwner {
            id: p.id.to_string(),
            record_type: match p.value {
                PrivateRecordValue::Ipv4(_) => "a",
                PrivateRecordValue::Ipv6(_) => "aaaa",
                PrivateRecordValue::Cname(_) => "cname",
            },
        })
        .collect();
    if !result.private_owners.is_empty() {
        result.private_response_precedence = "pending_p3";
        result
            .limitations
            .push("private_owner_present_effective_answer_not_determined");
    }
    Ok(result)
}
pub(super) fn tcp(
    compiled: &CompiledConfiguration,
    trace: PolicyTrace<'_, TrafficAction>,
    query: &CanonicalDomain,
    intent: TcpIntent,
    context: TcpContext,
) -> Result<SelectionExplanation, SimulationError> {
    let TcpContext {
        flow,
        source,
        port,
        transport,
    } = context;
    let predicted = match trace.selection().action() {
        TrafficAction::Deny {} => PredictedAction::TcpDeny,
        TrafficAction::Outbound { outbound } => PredictedAction::TcpOutbound {
            outbound: outbound.to_string(),
        },
    };
    let resolved = match intent {
        TcpIntent::Deny(r) => refused(r),
        TcpIntent::GatewayDirect(id) => Intent::GatewayDirect {
            outbound: id.to_string(),
        },
        TcpIntent::ProtectedProxy(id) => Intent::ProtectedProxy {
            outbound: id.to_string(),
        },
    };
    let mut result = base(
        compiled,
        &trace,
        query,
        flow,
        predicted,
        resolved,
        intent.decision_point(),
    )?;
    result.destination_port = Some(port.get());
    result.transport = match transport {
        RequestTransport::Tcp => "tcp",
        RequestTransport::Udp => "udp",
        RequestTransport::Unknown => "unknown",
    };
    result.limitations.extend([
        "network_group_scope_not_individual_device",
        "ech_destination_recovery_not_proven",
        "udp_quic_tcp_proxy_route_not_supported",
    ]);
    result.limitations.push(if source.is_ipv4() {
        "ipv6_authorization_not_established"
    } else {
        "ipv4_authorization_not_established"
    });
    Ok(result)
}
