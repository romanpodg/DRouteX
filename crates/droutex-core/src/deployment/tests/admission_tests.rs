use super::*;
use droutex_common::{ListenerId, OutboundAvailability, ProxyOutboundKind};
use droutex_config::{CompletePublication, OutboundObservation};
use droutex_policy::TcpCapability;
use serde_json::{Value, json};
use std::{net::IpAddr, num::NonZeroU16};

fn text(n: usize) -> String {
    typed::<ListenerId>(n).to_string()
}
pub(super) fn document() -> Value {
    json!({"schemaVersion":2,
        "profiles":[{"id":text(10),"name":"DNS","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"deny"}},
            {"id":text(11),"name":"Traffic","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"outbound","outbound":text(90)}}],
        "listeners":[{"id":text(20),"name":"UDP","enabled":true,"bind":"127.0.0.1:5353","transport":"dns_udp"},
            {"id":text(21),"name":"TCP","enabled":true,"bind":"127.0.0.1:5353","transport":"dns_tcp"},
            {"id":text(22),"name":"Gateway","enabled":true,"bind":"127.0.0.1:10443","transport":"gateway_tcp"}],
        "endpoints":[{"id":text(30),"name":"DNS endpoint","enabled":true,"profile":text(10),"access":[
            {"transport":"udp","listener":text(20),"grants":[text(40)]},{"transport":"tcp","listener":text(21),"grants":[text(41)]}]}],
        "networkGroups":[{"id":text(50),"name":"Shared group","trafficProfile":text(11)}],
        "legacyGrants":[{"id":text(40),"source":"203.0.113.7","listener":text(20),"group":text(50),"endpoint":text(30),"lease":{"cause":"manual","issuedAt":900,"expiresAt":1500}},
            {"id":text(41),"source":"203.0.113.7","listener":text(21),"group":text(50),"endpoint":text(30),"lease":{"cause":"manual","issuedAt":900,"expiresAt":1500}}],
        "gatewayGrants":[{"id":text(42),"source":"203.0.113.7","listener":text(22),"group":text(50),
            "targets":[{"mode":"suffix","value":"service.test"}],"ports":[{"from":443,"to":443}],"outbounds":[text(90)],"lease":{"cause":"manual","issuedAt":900,"expiresAt":1500}}],
        "outbounds":[{"id":text(90),"name":"Direct","enabled":true,"transport":{"kind":"vps_direct"}}]})
}
pub(super) fn candidate(document: &Value, sequence: u64) -> (Vec<u8>, Artifacts) {
    let checked = parse_configuration(
        &serde_json::to_vec(document).unwrap(),
        InputFormat::Json,
        &ReferenceCatalog::default(),
    )
    .unwrap();
    let publication = CompletePublication::new(checked, vec![], &[], &[]).unwrap();
    (
        publication
            .encode(
                SnapshotMetadata {
                    node: typed(1),
                    snapshot: typed(2),
                    source: typed(3),
                    sequence: SnapshotSequence::new(sequence).unwrap(),
                },
                &signer(false).0,
            )
            .unwrap(),
        Artifacts::new(vec![]).unwrap(),
    )
}
pub(super) fn apply_document(deployment: &Deployment, document: &Value, sequence: u64) {
    let (bytes, artifacts) = candidate(document, sequence);
    deployment
        .activate(bytes, artifacts, &Cancellation::default())
        .unwrap();
}
pub(super) fn start(deployment: &Deployment, wall: u64) {
    deployment.admission.lock().unwrap().fake_start(wall);
}
pub(super) fn time(deployment: &Deployment, wall: u64, elapsed: u64) {
    deployment
        .admission
        .lock()
        .unwrap()
        .fake_time(wall, elapsed);
}
pub(super) fn peer() -> IpAddr {
    "203.0.113.7".parse().unwrap()
}
fn observations() -> Vec<OutboundObservation> {
    vec![OutboundObservation {
        id: typed(90),
        kind: ProxyOutboundKind::GatewayDirect,
        availability: OutboundAvailability::Available,
        tcp: TcpCapability::Supported,
    }]
}

#[test]
fn signed_invalid_upstream_candidate_retains_actual_lkg_and_original_admission() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let mut document = document();
    document["profiles"][0]["defaultDnsAction"] = json!({"kind":"resolve","upstream":text(60)});
    document["originUpstream"] = json!(text(60));
    let document_bytes = serde_json::to_vec(&document).unwrap();
    let candidate = |payload: &[u8], sequence, complete| {
        let configuration = parse_configuration(
            &document_bytes,
            InputFormat::Json,
            &ReferenceCatalog {
                upstreams: [typed(60)].into(),
                ..Default::default()
            },
        )
        .unwrap();
        let artifacts = Artifacts::new(vec![(
            SnapshotResource::Upstream(typed(60)),
            payload.to_vec(),
        )])
        .unwrap();
        let metadata = SnapshotMetadata {
            node: typed(1),
            snapshot: typed(2),
            source: typed(3),
            sequence: SnapshotSequence::new(sequence).unwrap(),
        };
        let encoded = if complete {
            CompletePublication::new(configuration, vec![], &artifacts.data(), &[])
                .unwrap()
                .encode(metadata, &signer(false).0)
                .unwrap()
        } else {
            let content =
                SnapshotContent::new(configuration, vec![], &artifacts.trusted().unwrap()).unwrap();
            encode_snapshot(&content, metadata, &signer(false).0).unwrap()
        };
        (encoded, artifacts)
    };
    let valid = br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"udp_tcp","address":"1.1.1.1:53"}]}"#;
    let (bytes, artifacts) = candidate(valid, 1, true);
    deployment
        .activate(bytes, artifacts, &Cancellation::default())
        .unwrap();
    time(&deployment, 1001, 1);
    let auth = deployment
        .prepare_legacy_dns(typed(20), peer())
        .unwrap()
        .consume()
        .unwrap();
    let check = auth.dns_io_check().unwrap();
    assert_eq!(
        auth.decision().origin_upstream().unwrap().members()[0]
            .address()
            .to_string(),
        "1.1.1.1:53"
    );
    for invalid in [b"opaque descriptor".as_slice(), br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"udp_tcp","address":"127.0.0.1:53"}]}"#] {
        let (bytes, artifacts) = candidate(invalid, 2, false);
        assert_eq!(deployment.activate(bytes, artifacts, &Cancellation::default()).unwrap_err().code(), ErrorCode::Compilation);
        let current = deployment.pin().unwrap();
        assert_eq!(current.metadata().sequence.get(), 1);
        assert_eq!(current.artifact(SnapshotResource::Upstream(typed(60))), Some(valid.as_slice()));
        assert_eq!(current.compiled_configuration().upstream(typed(60)).unwrap().members()[0].address().to_string(), "1.1.1.1:53");
        auth.verify_current().unwrap();
        check.verify_current().unwrap();
    }
    // Refused sequence2 candidates did not consume its replay floor.
    let (bytes, artifacts) = candidate(valid, 2, true);
    deployment
        .activate(bytes, artifacts, &Cancellation::default())
        .unwrap();
    assert_eq!(deployment.pin().unwrap().metadata().sequence.get(), 2);
    assert_eq!(
        check.verify_current().unwrap_err().code(),
        AdmissionCode::Retired
    );
}
fn gateway<'store>(
    deployment: &'store Deployment,
    peer: IpAddr,
    domain: &str,
    port: u16,
) -> std::result::Result<AdmissionAttempt<'store>, AdmissionError> {
    deployment.prepare_gateway(
        typed(22),
        peer,
        domain,
        NonZeroU16::new(port).unwrap(),
        observations(),
    )
}

#[test]
fn manual_current_admission_keeps_dns_and_gateway_profiles_separate() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    time(&deployment, 1001, 1);
    for listener in [20, 21] {
        let request = deployment
            .prepare_legacy_dns(typed(listener), peer())
            .unwrap()
            .consume()
            .unwrap();
        assert!(
            matches!(request.scope(),AdmissionScope::LegacyDns(binding) if binding.endpoint==typed(30) && binding.profile==typed(10))
        );
        assert_eq!(request.decision().metadata().sequence.get(), 1);
    }
    let request = gateway(&deployment, peer(), "CDN.Service.Test.", 443)
        .unwrap()
        .consume()
        .unwrap();
    assert!(
        matches!(request.scope(),AdmissionScope::Gateway(candidate) if candidate.identity().group==typed(50) && candidate.action().profile()==typed(11) && candidate.outbound()==typed(90))
    );
    for bad_peer in ["203.0.113.8", "2001:db8::7", "::ffff:203.0.113.7"] {
        assert!(
            deployment
                .prepare_legacy_dns(typed(20), bad_peer.parse().unwrap())
                .is_err()
        );
        assert!(gateway(&deployment, bad_peer.parse().unwrap(), "service.test", 443).is_err());
    }
    assert!(gateway(&deployment, peer(), "service.test.attacker.test", 443).is_err());
    assert!(gateway(&deployment, peer(), "service.test", 80).is_err());
    assert!(
        deployment
            .prepare_gateway(
                typed(22),
                peer(),
                "service.test",
                NonZeroU16::new(443).unwrap(),
                vec![]
            )
            .is_err()
    );
    assert_eq!(
        deployment.automatic_learning(),
        AutomaticLearning::UnavailableUntilSecureDns
    );
}

#[test]
fn v1_associations_and_held_old_attempts_never_authorize_new_requests() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let mut v1 = document();
    v1["schemaVersion"] = json!(1);
    for table in ["legacyGrants", "gatewayGrants"] {
        for grant in v1[table].as_array_mut().unwrap() {
            grant.as_object_mut().unwrap().remove("lease");
        }
    }
    apply_document(&deployment, &v1, 1);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Unavailable
    );
    apply_document(&deployment, &document(), 2);
    time(&deployment, 1001, 1);
    let old = deployment.prepare_legacy_dns(typed(20), peer()).unwrap();
    let accepted = gateway(&deployment, peer(), "service.test", 443)
        .unwrap()
        .consume()
        .unwrap();
    let mut replacement = document();
    for grant in replacement["legacyGrants"].as_array_mut().unwrap() {
        grant["source"] = json!("203.0.113.8");
    }
    replacement["gatewayGrants"][0]["source"] = json!("203.0.113.8");
    apply_document(&deployment, &replacement, 3);
    assert_eq!(old.consume().unwrap_err().code(), AdmissionCode::Retired);
    assert_eq!(
        accepted.verify_current().unwrap_err().code(),
        AdmissionCode::Retired
    );
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::UnknownSource
    );
    assert!(
        deployment
            .prepare_legacy_dns(typed(20), "203.0.113.8".parse().unwrap())
            .unwrap()
            .consume()
            .is_ok()
    );
    assert_eq!(accepted.decision().metadata().sequence.get(), 2);
}

#[test]
fn expiry_clock_fault_and_missing_checkpoint_are_independent_denials() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1004, 4);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::CheckpointRequired
    );
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    assert!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap()
            .consume()
            .is_ok()
    );
    time(&deployment, 1000, 5);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Clock
    );
    time(&deployment, 1012, 12);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Clock
    );
    deployment
        .reanchor_authorization_clock(&Cancellation::default())
        .unwrap();
    assert!(deployment.prepare_legacy_dns(typed(20), peer()).is_ok());
    time(&deployment, 1498, 498);
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Lease
    );
}

#[test]
fn consumption_retry_retains_one_original_generation_and_quota_reservation() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1001, 1);
    let attempt = deployment.prepare_legacy_dns(typed(20), peer()).unwrap();
    time(&deployment, 1004, 4);
    let AdmissionConsumption::Refused { attempt, error } = attempt.try_consume() else {
        panic!("missing checkpoint authorized an attempt");
    };
    assert_eq!(error.code(), AdmissionCode::CheckpointRequired);
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    let mut others = Vec::new();
    for _ in 0..63 {
        others.push(deployment.prepare_legacy_dns(typed(21), peer()).unwrap());
    }
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(21), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    let AdmissionConsumption::Authorized(auth) = attempt.try_consume() else {
        panic!("successful refresh did not permit the same reservation");
    };
    assert_eq!(auth.decision().metadata().sequence.get(), 1);
    assert!(
        matches!(auth.scope(), AdmissionScope::LegacyDns(binding) if binding.profile == typed(10))
    );
    drop(auth);
    assert!(deployment.prepare_legacy_dns(typed(21), peer()).is_ok());
}

#[test]
fn dns_io_check_loses_owner_without_holding_quota_or_a_generation_pin() {
    use std::{sync::mpsc, thread, time::Duration};
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1002, 2);
    let auth = deployment
        .prepare_legacy_dns(typed(20), peer())
        .unwrap()
        .consume()
        .unwrap();
    assert_eq!(auth.network_group(), typed(50));
    let check = auth.dns_io_check().unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (continue_tx, continue_rx) = mpsc::channel();
    let task = thread::spawn(move || {
        check.verify_current().unwrap();
        ready_tx.send(()).unwrap();
        continue_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            check.verify_current().unwrap_err().code(),
            AdmissionCode::Retired
        );
        check
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let others: Vec<_> = (0..63)
        .map(|_| deployment.prepare_legacy_dns(typed(21), peer()).unwrap())
        .collect();
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(21), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    drop(auth);
    let new_auth = deployment
        .prepare_legacy_dns(typed(21), peer())
        .unwrap()
        .consume()
        .unwrap();
    new_auth.dns_io_check().unwrap().verify_current().unwrap();
    continue_tx.send(()).unwrap();
    let old_check = task.join().unwrap();
    drop(new_auth);
    drop(others);
    // A retained checker cannot keep retired generations busy or revive under a
    // later admission/publication with the same signed grant intent.
    for sequence in 2..=4 {
        apply_document(&deployment, &document(), sequence);
        assert_eq!(
            old_check.verify_current().unwrap_err().code(),
            AdmissionCode::Retired
        );
    }
    drop(deployment);
    assert_eq!(
        old_check.verify_current().unwrap_err().code(),
        AdmissionCode::Retired
    );
}

#[test]
fn dns_io_check_shares_current_revocation_lease_clock_and_checkpoint_denials() {
    for failure in [
        "replace",
        "revoke",
        "lease",
        "clock",
        "checkpoint",
        "failed checkpoint",
    ] {
        let deployment = Deployment::in_memory(typed(1), trust(false));
        start(&deployment, 1000);
        apply_document(&deployment, &document(), 1);
        time(&deployment, 1001, 1);
        let auth = deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap()
            .consume()
            .unwrap();
        let check = auth.dns_io_check().unwrap();
        let expected = match failure {
            "replace" | "revoke" => {
                let mut replacement = document();
                if failure == "revoke" {
                    replacement["legacyGrants"][0]["source"] = json!("203.0.113.8");
                }
                apply_document(&deployment, &replacement, 2);
                AdmissionCode::Retired
            }
            "lease" => {
                time(&deployment, 1498, 498);
                deployment
                    .refresh_authorization_clock(&Cancellation::default())
                    .unwrap();
                AdmissionCode::Lease
            }
            "clock" => {
                time(&deployment, 1000, 5);
                AdmissionCode::Clock
            }
            "checkpoint" => {
                time(&deployment, 1004, 4);
                AdmissionCode::CheckpointRequired
            }
            "failed checkpoint" => {
                deployment.admission.lock().unwrap().fail_checkpoint();
                AdmissionCode::Unavailable
            }
            _ => unreachable!(),
        };
        assert_eq!(
            check.verify_current().unwrap_err().code(),
            expected,
            "{failure}"
        );
        assert_eq!(
            auth.verify_current().unwrap_err().code(),
            expected,
            "{failure}"
        );
        assert_eq!(
            auth.dns_io_check()
                .unwrap()
                .verify_current()
                .unwrap_err()
                .code(),
            expected,
            "{failure}"
        );
        if failure == "checkpoint" {
            deployment
                .refresh_authorization_clock(&Cancellation::default())
                .unwrap();
            check.verify_current().unwrap();
            auth.verify_current().unwrap();
            assert_eq!(auth.decision().metadata().sequence.get(), 1);
        }
    }
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1001, 1);
    let gateway = gateway(&deployment, peer(), "service.test", 443)
        .unwrap()
        .consume()
        .unwrap();
    assert_eq!(
        gateway.dns_io_check().unwrap_err().code(),
        AdmissionCode::Scope
    );
}

#[test]
fn retained_consumption_never_reacquires_after_revocation_expiry_or_checkpoint_failure() {
    for failure in [
        AdmissionCode::Retired,
        AdmissionCode::Lease,
        AdmissionCode::Unavailable,
    ] {
        let deployment = Deployment::in_memory(typed(1), trust(false));
        start(&deployment, 1000);
        apply_document(&deployment, &document(), 1);
        time(&deployment, 1001, 1);
        let attempt = deployment.prepare_legacy_dns(typed(20), peer()).unwrap();
        time(&deployment, 1004, 4);
        let AdmissionConsumption::Refused { attempt, error } = attempt.try_consume() else {
            panic!("missing checkpoint authorized an attempt");
        };
        assert_eq!(error.code(), AdmissionCode::CheckpointRequired);
        match failure {
            AdmissionCode::Retired => apply_document(&deployment, &document(), 2),
            AdmissionCode::Lease => time(&deployment, 1498, 498),
            AdmissionCode::Unavailable => (),
            _ => unreachable!(),
        }
        deployment
            .refresh_authorization_clock(&Cancellation::default())
            .unwrap();
        if failure == AdmissionCode::Unavailable {
            deployment.admission.lock().unwrap().fail_checkpoint();
        }
        let AdmissionConsumption::Refused { attempt, error } = attempt.try_consume() else {
            panic!("refresh bypassed {failure:?}");
        };
        assert_eq!(error.code(), failure);
        drop(attempt);
    }
}

#[test]
fn shared_group_quotas_survive_renewal_and_release_on_drop() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1002, 2);
    let mut held = Vec::new();
    for _ in 0..64 {
        held.push(deployment.prepare_legacy_dns(typed(20), peer()).unwrap());
    }
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(21), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    drop(held.pop());
    assert!(deployment.prepare_legacy_dns(typed(21), peer()).is_ok());
    drop(held);
    // Total rate credits are shared across both DNS transports, not reset by publication.
    for _ in 0..135 {
        drop(deployment.prepare_legacy_dns(typed(21), peer()).unwrap());
    }
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    apply_document(&deployment, &document(), 2);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    time(&deployment, 1003, 3);
    assert!(deployment.prepare_legacy_dns(typed(20), peer()).is_ok());
}

#[test]
fn invalid_gateway_targets_still_consume_attempt_rate_without_leaking_concurrency() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1002, 2);
    for _ in 0..40 {
        assert!(gateway(&deployment, peer(), "ungranted.test", 443).is_err());
    }
    assert_eq!(
        gateway(&deployment, peer(), "service.test", 443)
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    time(&deployment, 1003, 3);
    assert!(gateway(&deployment, peer(), "service.test", 443).is_ok());
}

#[test]
fn future_or_expired_lease_cannot_be_renewed_by_republication_or_v1_rollback() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let mut future = document();
    for grant in future["legacyGrants"].as_array_mut().unwrap() {
        grant["lease"] = json!({"cause":"manual","issuedAt":1100,"expiresAt":1200});
    }
    apply_document(&deployment, &future, 1);
    time(&deployment, 1001, 1);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Lease
    );
    apply_document(&deployment, &document(), 2);
    time(&deployment, 1498, 498);
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    apply_document(&deployment, &document(), 3);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Lease
    );
    let floor = deployment
        .writer
        .lock()
        .unwrap()
        .journal
        .authorization_floor()
        .unwrap();
    let mut v1 = document();
    v1["schemaVersion"] = json!(1);
    for table in ["legacyGrants", "gatewayGrants"] {
        for grant in v1[table].as_array_mut().unwrap() {
            grant.as_object_mut().unwrap().remove("lease");
        }
    }
    apply_document(&deployment, &v1, 4);
    assert_eq!(
        deployment
            .writer
            .lock()
            .unwrap()
            .journal
            .authorization_floor(),
        Some(floor)
    );
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Unavailable
    );
}

#[test]
fn concurrent_reservations_share_limits_and_cannot_consume_after_applied_revocation() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1002, 2);
    let barrier = std::sync::Barrier::new(9);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    let held: Vec<_> = (0..9)
                        .filter_map(|_| deployment.prepare_legacy_dns(typed(20), peer()).ok())
                        .collect();
                    barrier.wait();
                    barrier.wait();
                    let count = held.len();
                    for reservation in held {
                        assert_eq!(
                            reservation.consume().unwrap_err().code(),
                            AdmissionCode::Retired
                        );
                    }
                    count
                })
            })
            .collect();
        barrier.wait();
        // A fresh generation is applied while all64 old reservations remain alive.
        apply_document(&deployment, &document(), 2);
        time(&deployment, 1003, 3);
        assert_eq!(
            deployment
                .prepare_legacy_dns(typed(20), peer())
                .unwrap_err()
                .code(),
            AdmissionCode::Quota
        );
        barrier.wait();
        assert_eq!(
            workers
                .into_iter()
                .map(|w| w.join().unwrap())
                .sum::<usize>(),
            64
        );
    });
    assert!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap()
            .consume()
            .is_ok()
    );
}

#[test]
fn clock_slew_never_revives_an_expired_lease() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let mut bounded = document();
    for grant in bounded["legacyGrants"].as_array_mut().unwrap() {
        grant["lease"]["expiresAt"] = json!(1014);
    }
    apply_document(&deployment, &bounded, 1);
    // Monotonic time grows normally; a legal2s wall correction must not restore expiry.
    time(&deployment, 1012, 10);
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Lease
    );
    time(&deployment, 1011, 11);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Lease
    );
}

#[test]
fn a_gateway_attempt_retains_one_exact_destination_inside_its_wider_grant() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1001, 1);
    let request = gateway(&deployment, peer(), "CDN.Service.Test.", 443)
        .unwrap()
        .consume()
        .unwrap();
    let AdmissionScope::Gateway(candidate) = request.scope() else {
        panic!("gateway scope")
    };
    assert_eq!(candidate.domain().as_str(), "cdn.service.test");
    assert_eq!(candidate.port().get(), 443);
    assert!(
        request
            .verify_gateway_destination("cdn.SERVICE.test.", NonZeroU16::new(443).unwrap())
            .is_ok()
    );
    for (name, port) in [
        ("other.service.test", 443),
        ("cdn.service.test", 80),
        ("bad..test", 443),
    ] {
        assert_eq!(
            request
                .verify_gateway_destination(name, NonZeroU16::new(port).unwrap())
                .unwrap_err()
                .code(),
            AdmissionCode::Scope
        );
    }
    let dns = deployment
        .prepare_legacy_dns(typed(20), peer())
        .unwrap()
        .consume()
        .unwrap();
    assert_eq!(
        dns.verify_gateway_destination("cdn.service.test", NonZeroU16::new(443).unwrap())
            .unwrap_err()
            .code(),
        AdmissionCode::Scope
    );
}

#[test]
fn same_peer_can_select_two_dns_profiles_without_changing_gateway_profile() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let mut split = document();
    let mut second = split["profiles"][0].clone();
    second["id"] = json!(text(12));
    second["name"] = json!("Other DNS");
    split["profiles"].as_array_mut().unwrap().push(second);
    split["endpoints"][0]["access"]
        .as_array_mut()
        .unwrap()
        .pop();
    split["endpoints"].as_array_mut().unwrap().push(json!({"id":text(31),"name":"Other endpoint","enabled":true,"profile":text(12),"access":[{"transport":"tcp","listener":text(21),"grants":[text(41)]}]}));
    split["legacyGrants"][1]["endpoint"] = json!(text(31));
    apply_document(&deployment, &split, 1);
    time(&deployment, 1001, 1);
    for listener in [20, 21, 20] {
        let request = deployment
            .prepare_legacy_dns(typed(listener), peer())
            .unwrap()
            .consume()
            .unwrap();
        let (endpoint, profile) = if listener == 20 { (30, 10) } else { (31, 12) };
        assert!(
            matches!(request.scope(),AdmissionScope::LegacyDns(b) if b.endpoint == typed(endpoint) && b.profile == typed(profile))
        );
        let tcp = gateway(&deployment, peer(), "service.test", 443)
            .unwrap()
            .consume()
            .unwrap();
        assert!(
            matches!(tcp.scope(), AdmissionScope::Gateway(c) if c.action().profile() == typed(11))
        );
    }
}

#[test]
fn exact_ipv6_peers_are_admitted_without_ipv4_or_neighbor_aliases() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let mut v6 = document();
    for table in ["legacyGrants", "gatewayGrants"] {
        for grant in v6[table].as_array_mut().unwrap() {
            grant["source"] = json!("2001:db8::7");
        }
    }
    apply_document(&deployment, &v6, 1);
    time(&deployment, 1001, 1);
    let source = "2001:db8::7".parse().unwrap();
    assert!(
        deployment
            .prepare_legacy_dns(typed(20), source)
            .unwrap()
            .consume()
            .is_ok()
    );
    assert!(
        gateway(&deployment, source, "service.test", 443)
            .unwrap()
            .consume()
            .is_ok()
    );
    for wrong in ["2001:db8::8", "203.0.113.7", "::ffff:203.0.113.7"] {
        assert!(
            deployment
                .prepare_legacy_dns(typed(20), wrong.parse().unwrap())
                .is_err()
        );
        assert!(gateway(&deployment, wrong.parse().unwrap(), "service.test", 443).is_err());
    }
}

#[test]
fn inventory_preflight_pauses_new_acquisitions_and_drop_restores_them() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1001, 1);
    let mut journal = deployment.writer.lock().unwrap().journal.clone();
    let groups = [typed(50)].into();
    let pause = deployment
        .pause_admission(&mut journal, 2, &groups)
        .unwrap();
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Unavailable
    );
    assert_eq!(
        gateway(&deployment, peer(), "service.test", 443)
            .unwrap_err()
            .code(),
        AdmissionCode::Unavailable
    );
    drop(pause);
    assert!(deployment.prepare_legacy_dns(typed(20), peer()).is_ok());
    assert!(gateway(&deployment, peer(), "service.test", 443).is_ok());
}

fn smartdns_document() -> Value {
    let mut doc = document();
    doc["profiles"][0]["defaultDnsAction"] = json!({"kind":"gateway","outbound":text(90)});
    doc["gatewayGrants"][0]["lease"]["expiresAt"] = json!(1010);
    doc["smartDnsRoutes"] = json!([{"endpoint":text(30),"grant":text(42),
        "domain":"service.test","port":443,"outbound":text(90),
        "addresses":["203.0.113.1","2001:db8::1"],"ttl":30}]);
    doc
}

#[test]
fn smartdns_binding_checks_both_leases_and_current_generation_without_granting_tcp() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let doc = smartdns_document();
    apply_document(&deployment, &doc, 1);
    time(&deployment, 1001, 1);
    let dns = deployment
        .prepare_legacy_dns(typed(20), peer())
        .unwrap()
        .consume()
        .unwrap();
    let binding = dns.smart_dns_binding("SeRvIcE.TeSt.", typed(90)).unwrap();
    binding.verify_current().unwrap();
    assert_eq!(binding.ttl(), 30);
    assert!(dns.smart_dns_binding("other.test", typed(90)).is_err());
    assert!(dns.smart_dns_binding("service.test", typed(91)).is_err());
    assert!(
        gateway(
            &deployment,
            "203.0.113.8".parse().unwrap(),
            "service.test",
            443
        )
        .is_err()
    );
    let tcp = gateway(&deployment, peer(), "service.test", 443)
        .unwrap()
        .consume()
        .unwrap();
    assert!(tcp.smart_dns_binding("service.test", typed(90)).is_err());
    time(&deployment, 1008, 8);
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    assert_eq!(
        binding.verify_current().unwrap_err().code(),
        AdmissionCode::Lease
    );
    dns.verify_current().unwrap(); // DNS lease alone still remains valid.
    apply_document(&deployment, &doc, 2);
    assert_eq!(
        binding.verify_current().unwrap_err().code(),
        AdmissionCode::Retired
    );
}

#[test]
fn smartdns_cached_address_never_authorizes_removed_route_or_wrong_source() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let doc = smartdns_document();
    apply_document(&deployment, &doc, 1);
    time(&deployment, 1001, 1);
    let dns = deployment
        .prepare_legacy_dns(typed(20), peer())
        .unwrap()
        .consume()
        .unwrap();
    let binding = dns.smart_dns_binding("service.test", typed(90)).unwrap();
    let cached = *binding
        .addresses()
        .iter()
        .find(|address| address.is_ipv4())
        .unwrap();
    assert_eq!(cached, "203.0.113.1".parse::<IpAddr>().unwrap());
    let attempt = deployment
        .prepare_smartdns_gateway(
            typed(22),
            peer(),
            "service.test",
            NonZeroU16::new(443).unwrap(),
            observations(),
        )
        .unwrap();
    assert!(
        deployment
            .prepare_smartdns_gateway(
                typed(22),
                "203.0.113.8".parse().unwrap(),
                "service.test",
                NonZeroU16::new(443).unwrap(),
                observations()
            )
            .is_err()
    );
    let mut replacement = doc.clone();
    replacement["smartDnsRoutes"] = json!([]);
    apply_document(&deployment, &replacement, 2);
    assert_eq!(
        binding.verify_current().unwrap_err().code(),
        AdmissionCode::Retired
    );
    assert_eq!(
        attempt.consume().unwrap_err().code(),
        AdmissionCode::Retired
    );
    assert!(
        deployment
            .prepare_smartdns_gateway(
                typed(22),
                peer(),
                "service.test",
                NonZeroU16::new(443).unwrap(),
                observations()
            )
            .is_err()
    );
    assert!(
        gateway(&deployment, peer(), "service.test", 443).is_ok(),
        "ordinary separate grant remains independent"
    );
}

#[test]
fn smartdns_replacement_uses_new_generation_without_old_answer_grace() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let doc = smartdns_document();
    apply_document(&deployment, &doc, 1);
    time(&deployment, 1001, 1);
    let old_dns = deployment
        .prepare_legacy_dns(typed(20), peer())
        .unwrap()
        .consume()
        .unwrap();
    let old = old_dns
        .smart_dns_binding("service.test", typed(90))
        .unwrap();
    let previous = deployment
        .prepare_smartdns_gateway(
            typed(22),
            peer(),
            "service.test",
            NonZeroU16::new(443).unwrap(),
            observations(),
        )
        .unwrap();
    let mut replacement = doc.clone();
    replacement["smartDnsRoutes"][0]["addresses"] = json!(["203.0.113.2"]);
    apply_document(&deployment, &replacement, 2);
    assert_eq!(
        old.verify_current().unwrap_err().code(),
        AdmissionCode::Retired
    );
    assert_eq!(
        previous.consume().unwrap_err().code(),
        AdmissionCode::Retired
    );
    time(&deployment, 1002, 2);
    let dns = deployment
        .prepare_legacy_dns(typed(20), peer())
        .unwrap()
        .consume()
        .unwrap();
    let current = dns.smart_dns_binding("service.test", typed(90)).unwrap();
    assert_eq!(
        current.addresses(),
        &["203.0.113.2".parse::<IpAddr>().unwrap()]
    );
    assert!(
        deployment
            .prepare_smartdns_gateway(
                typed(22),
                peer(),
                "service.test",
                NonZeroU16::new(443).unwrap(),
                observations()
            )
            .unwrap()
            .consume()
            .is_ok()
    );
}

#[test]
fn gateway_io_check_requires_owner_and_current_gateway_lease() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1001, 1);
    let dns = deployment
        .prepare_legacy_dns(typed(20), peer())
        .unwrap()
        .consume()
        .unwrap();
    assert!(matches!(dns.gateway_io_check(), Err(error) if error.code() == AdmissionCode::Scope));
    let auth = gateway(&deployment, peer(), "service.test", 443)
        .unwrap()
        .consume()
        .unwrap();
    let check = auth.gateway_io_check().unwrap();
    check.verify_current().unwrap();
    let mut revoked = document();
    revoked["gatewayGrants"][0]["source"] = json!("203.0.113.8");
    apply_document(&deployment, &revoked, 2);
    assert_eq!(
        check.verify_current().unwrap_err().code(),
        AdmissionCode::Retired
    );
    drop(auth);
    assert_eq!(
        check.verify_current().unwrap_err().code(),
        AdmissionCode::Retired
    );
    drop(dns);
    drop(deployment);
    assert_eq!(
        check.verify_current().unwrap_err().code(),
        AdmissionCode::Retired
    );
}

#[test]
fn gateway_io_check_expires_with_original_gateway_lease() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let mut doc = document();
    doc["gatewayGrants"][0]["lease"]["expiresAt"] = json!(1010);
    apply_document(&deployment, &doc, 1);
    time(&deployment, 1001, 1);
    let auth = gateway(&deployment, peer(), "service.test", 443)
        .unwrap()
        .consume()
        .unwrap();
    let check = auth.gateway_io_check().unwrap();
    time(&deployment, 1008, 8);
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    assert_eq!(
        check.verify_current().unwrap_err().code(),
        AdmissionCode::Lease
    );
}

#[test]
fn live_smartdns_gateway_check_requires_current_exact_route() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let doc = smartdns_document();
    apply_document(&deployment, &doc, 1);
    time(&deployment, 1001, 1);
    let auth = gateway(&deployment, peer(), "service.test", 443)
        .unwrap()
        .consume()
        .unwrap();
    auth.verify_smartdns_gateway_destination("service.test", NonZeroU16::new(443).unwrap())
        .unwrap();
    assert!(
        auth.verify_smartdns_gateway_destination("other.test", NonZeroU16::new(443).unwrap())
            .is_err()
    );
    let mut removed = doc;
    removed["smartDnsRoutes"] = json!([]);
    apply_document(&deployment, &removed, 2);
    assert_eq!(
        auth.verify_smartdns_gateway_destination("service.test", NonZeroU16::new(443).unwrap())
            .unwrap_err()
            .code(),
        AdmissionCode::Retired
    );
    let generic = gateway(&deployment, peer(), "service.test", 443)
        .unwrap()
        .consume()
        .unwrap();
    assert_eq!(
        generic
            .verify_smartdns_gateway_destination("service.test", NonZeroU16::new(443).unwrap())
            .unwrap_err()
            .code(),
        AdmissionCode::Scope
    );
}

#[test]
fn transparent_smartdns_selects_only_an_unambiguous_signed_port() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let mut doc = smartdns_document();
    apply_document(&deployment, &doc, 1);
    time(&deployment, 1001, 1);
    let auth = deployment
        .prepare_smartdns_gateway_for_name(typed(22), peer(), "service.test", observations())
        .unwrap()
        .consume()
        .unwrap();
    assert!(
        matches!(auth.scope(), AdmissionScope::Gateway(candidate) if candidate.port().get() == 443)
    );
    assert!(
        deployment
            .prepare_smartdns_gateway_for_name(
                typed(22),
                "203.0.113.8".parse().unwrap(),
                "service.test",
                observations()
            )
            .is_err()
    );
    drop(auth);
    doc["gatewayGrants"][0]["ports"] = json!([{"from":443,"to":8443}]);
    doc["endpoints"].as_array_mut().unwrap().push(json!({"id":text(31),"name":"second","enabled":true,"profile":text(10),"access":[{"transport":"udp","listener":text(20),"grants":[text(43)]}]}));
    doc["legacyGrants"].as_array_mut().unwrap().push(json!({"id":text(43),"source":"203.0.113.8","listener":text(20),"group":text(50),"endpoint":text(31),"lease":{"cause":"manual","issuedAt":900,"expiresAt":1500}}));
    doc["smartDnsRoutes"].as_array_mut().unwrap().push(json!({"endpoint":text(31),"grant":text(42),"domain":"service.test","port":8443,"outbound":text(90),"addresses":["203.0.113.2"],"ttl":30}));
    apply_document(&deployment, &doc, 2);
    assert!(
        deployment
            .prepare_smartdns_gateway_for_name(typed(22), peer(), "service.test", observations())
            .is_err()
    );
}

#[test]
fn transparent_smartdns_invalid_names_spend_gateway_rate_credit() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    apply_document(&deployment, &smartdns_document(), 1);
    time(&deployment, 1002, 2);
    for _ in 0..40 {
        assert!(
            deployment
                .prepare_smartdns_gateway_for_name(
                    typed(22),
                    peer(),
                    "unknown.test",
                    observations()
                )
                .is_err()
        );
    }
    assert_eq!(
        deployment
            .prepare_smartdns_gateway_for_name(typed(22), peer(), "service.test", observations())
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    time(&deployment, 1003, 3);
    assert!(
        deployment
            .prepare_smartdns_gateway_for_name(typed(22), peer(), "service.test", observations())
            .is_ok()
    );
}

#[test]
fn smartdns_ingress_reservation_does_not_repin_after_publication() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    start(&deployment, 1000);
    let doc = smartdns_document();
    apply_document(&deployment, &doc, 1);
    time(&deployment, 1001, 1);
    let ingress = deployment
        .reserve_smartdns_gateway_ingress(typed(22), peer())
        .unwrap();
    apply_document(&deployment, &doc, 2);
    let attempt = ingress
        .prepare_for_name("service.test", observations())
        .unwrap();
    assert_eq!(
        attempt.consume().unwrap_err().code(),
        AdmissionCode::Retired
    );
}
