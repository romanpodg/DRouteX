use super::super::admission_tests::{apply_document, document, peer, start, time};
use super::*;

#[test]
fn successful_checkpoint_can_require_another_floor_when_commit_crosses_a_window() {
    let fixture = Fixture::new();
    let deployment = Arc::new(fixture.provision());
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1001, 1);
    let retained = deployment.prepare_legacy_dns(typed(20), peer()).unwrap();
    let during_commit = Arc::clone(&deployment);
    hook(move |phase| {
        if phase == "manifest-prepared" {
            // The transaction already sampled1005. Its actual commit completes
            // after the conservative clock upper bound requires1010 instead.
            time(&during_commit, 1004, 4);
        }
        Ok(())
    });
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    clear_hook();
    assert_eq!(Arc::strong_count(&deployment), 1);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.root.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["clock_floor"], 1005);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::CheckpointRequired
    );
    assert_eq!(
        retained.consume().unwrap_err().code(),
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
}

#[test]
fn manifest_commit_pauses_acquisitions_and_known_failure_releases_pause() {
    let fixture = Fixture::new();
    // Hook owns a test-only reference while exercising the real commit callback.
    let deployment = Arc::new(fixture.provision());
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1001, 1);
    let during_commit = Arc::clone(&deployment);
    hook(move |phase| {
        if phase == "manifest-prepared" {
            assert_eq!(
                during_commit
                    .prepare_legacy_dns(typed(20), peer())
                    .unwrap_err()
                    .code(),
                AdmissionCode::Unavailable
            );
            Err(DeploymentError(ErrorCode::Filesystem))
        } else {
            Ok(())
        }
    });
    let (bytes, artifacts) = super::super::admission_tests::candidate(&document(), 2);
    assert_eq!(
        deployment
            .activate(bytes, artifacts, &Cancellation::default())
            .unwrap_err()
            .code(),
        ErrorCode::Filesystem
    );
    clear_hook();
    assert_eq!(Arc::strong_count(&deployment), 1);
    assert_eq!(deployment.pin().unwrap().metadata().sequence.get(), 1);
    assert!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap()
            .consume()
            .is_ok()
    );
    apply_document(&deployment, &document(), 2);
    assert!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap()
            .consume()
            .is_ok()
    );
}

#[test]
fn bounded_leases_recover_without_cp_but_restart_cannot_recover_consumed_time() {
    let fixture = Fixture::new();
    let deployment = fixture.provision();
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1001, 1);
    assert!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap()
            .consume()
            .is_ok()
    );
    drop(deployment);
    fs::remove_dir(fixture.parent.join("cp")).unwrap();
    fs::remove_dir(fixture.parent.join("backups")).unwrap();
    let deployment = fixture.open().unwrap();
    start(&deployment, 1000);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::RestartWait
    );
    time(&deployment, 1007, 7);
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
    drop(deployment);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.root.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["version"], 2);
    assert_eq!(manifest["clock_floor"], 1500);
}

#[test]
fn checkpoint_failures_and_applied_removal_never_reset_a_floor_or_restore_old_authority() {
    let fixture = Fixture::new();
    let deployment = fixture.provision();
    start(&deployment, 1000);
    apply_document(&deployment, &document(), 1);
    time(&deployment, 1004, 4);
    hook(|phase| {
        if phase == "manifest-prepared" {
            Err(DeploymentError(ErrorCode::Filesystem))
        } else {
            Ok(())
        }
    });
    assert_eq!(
        deployment
            .refresh_authorization_clock(&Cancellation::default())
            .unwrap_err()
            .code(),
        ErrorCode::Filesystem
    );
    clear_hook();
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Unavailable
    );
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    assert!(deployment.prepare_legacy_dns(typed(20), peer()).is_ok());
    time(&deployment, 1009, 9);
    hook(|phase| {
        if phase == "manifest-renamed" {
            Err(DeploymentError(ErrorCode::Filesystem))
        } else {
            Ok(())
        }
    });
    assert_eq!(
        deployment
            .refresh_authorization_clock(&Cancellation::default())
            .unwrap_err()
            .code(),
        ErrorCode::UncertainCommit
    );
    clear_hook();
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::Unavailable
    );
    assert_eq!(
        deployment
            .refresh_authorization_clock(&Cancellation::default())
            .unwrap_err()
            .code(),
        ErrorCode::RecoveryRequired
    );
    drop(deployment);
    let deployment = fixture.open().unwrap();
    start(&deployment, 1009);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::RestartWait
    );
    time(&deployment, 1017, 8);
    deployment
        .refresh_authorization_clock(&Cancellation::default())
        .unwrap();
    let old = deployment.prepare_legacy_dns(typed(20), peer()).unwrap();
    let mut removed = document();
    removed["endpoints"] = serde_json::json!([]);
    removed["legacyGrants"] = serde_json::json!([]);
    removed["gatewayGrants"] = serde_json::json!([]);
    apply_document(&deployment, &removed, 2);
    assert_eq!(old.consume().unwrap_err().code(), AdmissionCode::Retired);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::UnknownSource
    );
    drop(deployment);
    let deployment = fixture.open().unwrap();
    start(&deployment, 1025);
    assert_eq!(
        deployment
            .prepare_legacy_dns(typed(20), peer())
            .unwrap_err()
            .code(),
        AdmissionCode::UnknownSource
    );
    drop(deployment);
    // Removing the initialized clock floor is corruption, never first provisioning.
    let manifest_path = fixture.root.join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["version"] = serde_json::json!(1);
    manifest.as_object_mut().unwrap().remove("clock_floor");
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert_eq!(failure(fixture.open()), ErrorCode::State);
}
