//! D44 unbound, allow-listed diagnostic adapters. Core alone binds a generation.
mod output;
use crate::{
    ActionPreparationError, CompiledConfiguration, OutboundObservation,
    snapshot::{SnapshotContent, TrustedResources},
};
use droutex_common::{DnsProfileBinding, GatewayNetworkIdentity};
use droutex_policy::{EvaluationError, GatewayReadiness, RequestTransport};
pub use output::{DiagnosticContext, Evidence, SelectionExplanation};
use std::{error::Error, fmt, num::NonZeroU16};

/// Simulation assumptions only, without ingress or diagnostic visibility authority.
pub enum SimulationRequest<'a> {
    Dns {
        binding: DnsProfileBinding,
        domain: &'a str,
        readiness: GatewayReadiness,
        context: DiagnosticContext,
    },
    Tcp {
        identity: GatewayNetworkIdentity,
        domain: &'a str,
        port: NonZeroU16,
        transport: RequestTransport,
        context: DiagnosticContext,
    },
}
impl<'a> SimulationRequest<'a> {
    /// Unknown compatibility/observations remain unknown, without a probe.
    pub fn dns(binding: DnsProfileBinding, domain: &'a str) -> Self {
        Self::Dns {
            binding,
            domain,
            readiness: GatewayReadiness::Unknown,
            context: DiagnosticContext::default(),
        }
    }
    pub fn tcp(identity: GatewayNetworkIdentity, domain: &'a str, port: NonZeroU16) -> Self {
        Self::Tcp {
            identity,
            domain,
            port,
            transport: RequestTransport::Tcp,
            context: DiagnosticContext::default(),
        }
    }
}
#[derive(Debug, Eq, PartialEq)]
pub enum SimulationError {
    Domain,
    Scope,
    Inconsistent,
    Evaluation(EvaluationError),
    Observations(ActionPreparationError),
}
impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("policy simulation refused")
    }
}
impl Error for SimulationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evaluation(e) => Some(e),
            Self::Observations(e) => Some(e),
            _ => None,
        }
    }
}

impl SnapshotContent {
    /// Unbound adapter only. Neither this input nor its returned value supplies
    /// generation or user/RBAC authority. Core must pair its own immutable inputs.
    pub fn explain_selection(
        &self,
        compiled: &CompiledConfiguration,
        request: SimulationRequest<'_>,
        observations: Vec<OutboundObservation>,
    ) -> Result<SelectionExplanation, SimulationError> {
        let references = TrustedResources::new(self.resource_pins().to_vec())
            .map_err(|_| SimulationError::Inconsistent)?
            .catalog();
        let resolver = self
            .configuration()
            .prepare_action_resolver(&references, observations)
            .map_err(SimulationError::Observations)?;
        match request {
            SimulationRequest::Dns {
                binding,
                domain,
                readiness,
                context,
            } => {
                let query = domain.parse().map_err(|_| SimulationError::Domain)?;
                let trace = compiled
                    .policy()
                    .explain_dns(&binding, &query)
                    .map_err(SimulationError::Evaluation)?;
                let intent = crate::resolve_dns_action(&resolver, trace.selection(), readiness)
                    .map_err(|_| SimulationError::Scope)?
                    .intent();
                output::dns(compiled, trace, &query, intent, context)
            }
            SimulationRequest::Tcp {
                identity,
                domain,
                port,
                transport,
                context,
            } => {
                if transport == RequestTransport::Tcp && context.quic == Evidence::Present {
                    return Err(SimulationError::Inconsistent);
                }
                let query = domain.parse().map_err(|_| SimulationError::Domain)?;
                let scope = compiled
                    .gateway_scope(identity.grant)
                    .ok_or(SimulationError::Scope)?;
                if !scope.matches_request_scope(&identity, &query, port) {
                    return Err(SimulationError::Scope);
                }
                let trace = compiled
                    .policy()
                    .explain_tcp(&identity, &query, port)
                    .map_err(SimulationError::Evaluation)?;
                if let crate::TrafficAction::Outbound { outbound } = trace.selection().action()
                    && !scope.matches_static_scope(&identity, &query, port, *outbound)
                {
                    return Err(SimulationError::Scope);
                }
                let intent = crate::resolve_tcp_action(&resolver, trace.selection(), transport)
                    .map_err(|_| SimulationError::Scope)?
                    .intent();
                output::tcp(
                    compiled,
                    trace,
                    &query,
                    intent,
                    output::TcpContext {
                        flow: context,
                        source: identity.source,
                        port,
                        transport,
                    },
                )
            }
        }
    }
}
