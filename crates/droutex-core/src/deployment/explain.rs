//! Complete D44 report assembly is private to the actual pinned generation.
use super::Decision;
use droutex_config::{
    OutboundObservation, SelectionExplanation, SimulationError, SimulationRequest,
};
use serde::Serialize;
use std::fmt;

/// Generation-bound prediction, not a network result, live permit or RBAC token.
/// Fields are private; neither deserialization nor caller-supplied generation
/// labels can construct a report. P6 must authorize Profile visibility externally.
/// ```compile_fail
/// use droutex_core::deployment::SimulationReport;
/// let report: SimulationReport = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// use droutex_core::deployment::SimulationReport;
/// fn relabel(mut report: SimulationReport) { report.generation = 999; }
/// ```
#[derive(Serialize)]
pub struct SimulationReport {
    schema_version: u32,
    node: String,
    snapshot: String,
    source_revision: String,
    generation: u64,
    content_hash: String,
    network_test_performed: bool,
    effective_route: (),
    selection: SelectionExplanation,
}
impl fmt::Debug for SimulationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationReport")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}
impl Decision<'_> {
    /// Pure simulation using only this pin's immutable inputs. Caller observations
    /// remain assumptions; actual ingress/visibility/address/TLS checks are later.
    pub fn simulate(
        &self,
        request: SimulationRequest<'_>,
        observations: Vec<OutboundObservation>,
    ) -> Result<SimulationReport, SimulationError> {
        let content = self.generation.content();
        let selection =
            content.explain_selection(&self.generation.compiled, request, observations)?;
        let metadata = self.generation.metadata();
        Ok(SimulationReport {
            schema_version: 1,
            node: metadata.node.to_string(),
            snapshot: metadata.snapshot.to_string(),
            source_revision: metadata.source.to_string(),
            generation: metadata.sequence.get(),
            content_hash: content.hash().to_string(),
            network_test_performed: false,
            effective_route: (),
            selection,
        })
    }
}
