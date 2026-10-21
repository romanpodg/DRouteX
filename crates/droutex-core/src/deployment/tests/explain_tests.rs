use super::*;
use droutex_common::DnsProfileBinding;
use droutex_config::{SimulationError, SimulationRequest};
use serde_json::Value;

pub(super) fn report(decision: &Decision<'_>) -> Value {
    serde_json::to_value(
        decision
            .simulate(
                SimulationRequest::dns(
                    DnsProfileBinding {
                        endpoint: typed(78),
                        profile: typed(77),
                    },
                    "RETAINED.test.",
                ),
                vec![],
            )
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn generation_reports_are_immutable_redacted_and_match_reviewed_golden() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    let (bytes, artifacts) = compiled_candidate(1, false, true, false);
    deployment
        .activate(bytes, artifacts, &Cancellation::default())
        .unwrap();
    let old = deployment.pin().unwrap();
    let first = report(&old);
    assert_eq!(first["network_test_performed"], false);
    assert!(first["effective_route"].is_null());
    assert_eq!(first["generation"], 1);
    assert_eq!(first["selection"]["canonical_domain"], "retained.test");
    assert_eq!(
        first["selection"]["private_response_precedence"],
        "pending_p3"
    );
    assert_eq!(first["selection"]["private_owners"][0]["record_type"], "a");
    let text = serde_json::to_string(&first).unwrap();
    for forbidden in [
        "192.0.2.7",
        "203.0.113.7",
        "private DNS metadata",
        "Original synthetic fixture.",
        "owner:synthetic-test",
    ] {
        assert!(!text.contains(forbidden), "leaked {forbidden}");
    }
    assert_eq!(
        first["selection"]["trace"][0]["source"]["resource_hash"],
        ContentHash::of(old.artifact(SnapshotResource::Filter(typed(92))).unwrap()).to_string()
    );
    let (bytes, artifacts) = compiled_candidate(2, true, false, false);
    assert_eq!(
        failure(deployment.activate(bytes, artifacts, &Cancellation::default())),
        ErrorCode::Compilation
    );
    assert_eq!(report(&deployment.pin().unwrap()), first);
    let (bytes, artifacts) = compiled_candidate(2, false, false, false);
    deployment
        .activate(bytes, artifacts, &Cancellation::default())
        .unwrap();
    let current = report(&deployment.pin().unwrap());
    assert_eq!(current["generation"], 2);
    assert_ne!(first["content_hash"], current["content_hash"]);
    assert_eq!(
        current["selection"]["private_owners"],
        serde_json::json!([])
    );
    assert_eq!(report(&old), first);
    assert_eq!(
        first,
        serde_json::from_str::<Value>(include_str!(
            "../../../../../api/schemas/explain/simulation-v1.example.json"
        ))
        .unwrap()
    );
}
#[test]
fn invalid_diagnostic_scope_and_domain_do_not_leak_partial_reports() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    let (bytes, artifacts) = compiled_candidate(1, false, true, false);
    deployment
        .activate(bytes, artifacts, &Cancellation::default())
        .unwrap();
    let pin = deployment.pin().unwrap();
    for (endpoint, profile) in [(999, 77), (78, 999)] {
        assert!(matches!(
            pin.simulate(
                SimulationRequest::dns(
                    DnsProfileBinding {
                        endpoint: typed(endpoint),
                        profile: typed(profile)
                    },
                    "retained.test"
                ),
                vec![]
            ),
            Err(SimulationError::Evaluation(_))
        ));
    }
    let err = pin
        .simulate(
            SimulationRequest::dns(
                DnsProfileBinding {
                    endpoint: typed(78),
                    profile: typed(77),
                },
                "SECRET.invalid/path",
            ),
            vec![],
        )
        .unwrap_err();
    assert_eq!(err, SimulationError::Domain);
    assert!(!format!("{err:?} {err}").contains("SECRET"));
}
