use super::*;
use std::{
    cell::RefCell,
    fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    sync::atomic::AtomicU64,
};
type Hook = Box<dyn FnMut(&str) -> Result<()>>;

#[test]
fn compiled_tables_restore_without_cp_and_failed_candidates_preserve_the_same_generation() {
    let fixture = Fixture::new();
    let deployment = fixture.provision();
    let (bytes, artifacts) = compiled_candidate(1, false, true, false);
    deployment
        .activate(bytes, artifacts, &Cancellation::default())
        .unwrap();
    let simulation_before = super::explain_tests::report(&deployment.pin().unwrap());
    drop(deployment);
    fs::remove_dir(fixture.parent.join("cp")).unwrap();
    fs::remove_dir(fixture.parent.join("backups")).unwrap();
    let deployment = fixture.open().unwrap();
    let old = deployment.pin().unwrap();
    assert_eq!(super::explain_tests::report(&old), simulation_before);
    assert_eq!(
        old.compiled_configuration().private_records()[0]
            .owner
            .as_str(),
        "retained.test"
    );
    let (bytes, artifacts) = compiled_candidate(2, false, false, true);
    assert_eq!(
        failure(deployment.activate(bytes, artifacts, &Cancellation::default())),
        ErrorCode::Compilation
    );
    let current = deployment.pin().unwrap();
    assert!(std::ptr::eq(
        old.compiled_configuration(),
        current.compiled_configuration()
    ));
    assert_eq!(current.metadata().sequence.get(), 1);
    assert_eq!(super::explain_tests::report(&current), simulation_before);
}
thread_local! { static HOOK: RefCell<Option<Hook>> = RefCell::new(None); }
pub(super) fn checkpoint(phase: &str) -> Result<()> {
    HOOK.with_borrow_mut(|hook| match hook {
        Some(hook) => hook(phase),
        None => Ok(()),
    })
}
fn hook(callback: impl FnMut(&str) -> Result<()> + 'static) {
    HOOK.with_borrow_mut(|slot| *slot = Some(Box::new(callback)));
}
fn clear_hook() {
    HOOK.with_borrow_mut(|slot| *slot = None);
}
struct Fixture {
    parent: PathBuf,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let parent = std::env::temp_dir().join(format!(
            "droutex-activation-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(parent.join("cp")).unwrap();
        fs::create_dir(parent.join("backups")).unwrap();
        Self {
            root: parent.join("dp"),
            parent,
        }
    }
    fn provision(&self) -> Deployment {
        Deployment::provision(
            &self.root,
            &self.parent.join("cp"),
            &self.parent.join("backups"),
            typed(1),
            trust(false),
        )
        .unwrap()
    }
    fn open(&self) -> Result<Deployment> {
        Deployment::open(&self.root, typed(1), trust(false))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        clear_hook();
        fs::remove_dir_all(&self.parent).unwrap();
    }
}

#[test]
fn cp_database_loss_restart_and_single_writer_lock() {
    let fixture = Fixture::new();
    let deployment = fixture.provision();
    assert_eq!(failure(fixture.open()), ErrorCode::Busy);
    apply(&deployment, 1, false, b"one").unwrap();
    apply(&deployment, 2, false, b"two").unwrap();
    drop(deployment);
    fs::remove_dir(fixture.parent.join("cp")).unwrap();
    fs::remove_dir(fixture.parent.join("backups")).unwrap();
    let deployment = fixture.open().unwrap();
    assert_version(&deployment, 2, b"two");
    assert_eq!(
        failure(apply(&deployment, 1, false, b"old")),
        ErrorCode::Snapshot(snapshot::SnapshotErrorCode::Replay)
    );
    apply(&deployment, 3, false, b"one").unwrap();
    drop(deployment);
    assert_version(&fixture.open().unwrap(), 3, b"one");
}

#[test]
fn drop_releases_lock_even_if_an_open_file_description_is_duplicated() {
    let fixture = Fixture::new();
    let deployment = fixture.provision();
    apply(&deployment, 1, false, b"one").unwrap();
    // Models the same OS open-file-description lifetime as a spawn inheriting a
    // descriptor before exec closes it. The duplicate deliberately outlives owner.
    let duplicate = match &deployment.writer.lock().unwrap().store {
        Store::Files(store) => store.duplicate_lock_for_test().unwrap(),
        Store::Memory => unreachable!(),
    };
    drop(deployment);
    let reopened = fixture.open().unwrap();
    assert_version(&reopened, 1, b"one");
    drop(duplicate);
    assert_eq!(failure(fixture.open()), ErrorCode::Busy); // Closing the old duplicate cannot unlock the new owner.
    drop(reopened);
    assert_version(&fixture.open().unwrap(), 1, b"one");
}

#[test]
fn rotation_recovery_does_not_require_revoked_previous_key() {
    let fixture = Fixture::new();
    let deployment = fixture.provision();
    apply(&deployment, 1, false, b"old").unwrap();
    let (k1, p1) = signer(false);
    let (k2, p2) = signer(true);
    deployment
        .replace_trust(SnapshotTrust::new(vec![(k1.id(), p1), (k2.id(), p2)]).unwrap())
        .unwrap();
    apply(&deployment, 2, true, b"new").unwrap();
    drop(deployment);
    assert!(fixture.open().is_err()); // Active signer absent, no fallback to previous.
    let restored = Deployment::open(&fixture.root, typed(1), trust(true)).unwrap();
    assert_version(&restored, 2, b"new");
}

#[test]
fn missing_corrupt_or_revoked_active_refuses_without_fallback() {
    for damage in [
        "manifest",
        "envelope",
        "artifact",
        "missing-envelope",
        "missing-artifact",
        "missing-manifest",
        "wrong-scope",
    ] {
        let fixture = Fixture::new();
        let deployment = fixture.provision();
        apply(&deployment, 1, false, b"old").unwrap();
        apply(&deployment, 2, false, b"good").unwrap();
        drop(deployment);
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture.root.join("manifest.json")).unwrap()).unwrap();
        let slot = fixture.root.join(format!(
            "slot-{}",
            manifest["active"]["slot"].as_u64().unwrap()
        ));
        match damage {
            "manifest" => fs::write(fixture.root.join("manifest.json"), b"{}").unwrap(),
            "envelope" => fs::write(slot.join("snapshot"), b"tamper").unwrap(),
            "artifact" => fs::write(slot.join(artifacts::filename(resource())), b"tamper").unwrap(),
            "missing-envelope" => fs::remove_file(slot.join("snapshot")).unwrap(),
            "missing-artifact" => {
                fs::remove_file(slot.join(artifacts::filename(resource()))).unwrap()
            }
            "missing-manifest" => fs::remove_file(fixture.root.join("manifest.json")).unwrap(),
            "wrong-scope" => {
                assert!(Deployment::open(&fixture.root, typed(123), trust(false)).is_err());
                continue;
            }
            _ => unreachable!(),
        }
        assert!(fixture.open().is_err(), "{damage}");
        assert!(
            Deployment::provision(
                &fixture.root,
                &fixture.parent.join("cp"),
                &fixture.parent.join("backups"),
                typed(1),
                trust(false)
            )
            .is_err()
        );
    }
}

#[test]
fn private_owner_modes_symlinks_hardlinks_and_unsafe_placement_refuse() {
    let fixture = Fixture::new();
    let deployment = fixture.provision();
    apply(&deployment, 1, false, b"one").unwrap();
    drop(deployment);
    fs::set_permissions(&fixture.root, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(fixture.open().is_err());
    fs::set_permissions(&fixture.root, fs::Permissions::from_mode(0o700)).unwrap();
    let manifest = fixture.root.join("manifest.json");
    let saved = fs::read(&manifest).unwrap();
    fs::hard_link(&manifest, fixture.parent.join("hardlink")).unwrap();
    assert!(fixture.open().is_err());
    fs::remove_file(fixture.parent.join("hardlink")).unwrap();
    fs::remove_file(&manifest).unwrap();
    fs::write(fixture.parent.join("outside"), saved).unwrap();
    symlink(fixture.parent.join("outside"), &manifest).unwrap();
    assert!(fixture.open().is_err());
    assert!(
        Deployment::provision(
            &fixture.parent.join("cp/forbidden"),
            &fixture.parent.join("cp"),
            &fixture.parent.join("backups"),
            typed(1),
            trust(false)
        )
        .is_err()
    );
    assert!(Deployment::open(Path::new("relative"), typed(1), trust(false)).is_err());
}

#[test]
fn unsafe_ancestor_and_root_substitution_cannot_redirect_writes() {
    let fixture = Fixture::new();
    fs::set_permissions(&fixture.parent, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(
        Deployment::provision(
            &fixture.root,
            &fixture.parent.join("cp"),
            &fixture.parent.join("backups"),
            typed(1),
            trust(false)
        )
        .is_err()
    );
    fs::set_permissions(&fixture.parent, fs::Permissions::from_mode(0o700)).unwrap();
    let deployment = fixture.provision();
    apply(&deployment, 1, false, b"one").unwrap();
    fs::set_permissions(&fixture.parent, fs::Permissions::from_mode(0o777)).unwrap();
    assert_eq!(
        failure(apply(&deployment, 2, false, b"two")),
        ErrorCode::UnsafePath
    );
    fs::set_permissions(&fixture.parent, fs::Permissions::from_mode(0o700)).unwrap();
    let other = fixture.parent.join("other");
    fs::create_dir(&other).unwrap();
    fs::set_permissions(&other, fs::Permissions::from_mode(0o700)).unwrap();
    fs::rename(&fixture.root, fixture.parent.join("moved")).unwrap();
    symlink(&other, &fixture.root).unwrap();
    assert_eq!(
        failure(apply(&deployment, 2, false, b"redirected")),
        ErrorCode::UnsafePath
    );
    assert_eq!(fs::read_dir(&other).unwrap().count(), 0);
    assert_version(&deployment, 1, b"one");
    drop(deployment);
    assert!(fixture.open().is_err());
}

#[test]
fn previous_retention_must_remain_private_flat_and_bounded() {
    for damage in ["link", "mode", "size", "aggregate"] {
        let fixture = Fixture::new();
        let deployment = fixture.provision();
        apply(&deployment, 1, false, b"one").unwrap();
        apply(&deployment, 2, false, b"two").unwrap();
        drop(deployment);
        let previous = fixture.root.join("slot-0");
        match damage {
            "link" => {
                fs::remove_dir_all(&previous).unwrap();
                symlink(fixture.parent.join("cp"), &previous).unwrap();
            }
            "mode" => fs::set_permissions(&previous, fs::Permissions::from_mode(0o755)).unwrap(),
            "size" => fs::OpenOptions::new()
                .write(true)
                .open(previous.join(artifacts::filename(resource())))
                .unwrap()
                .set_len((MAX_RESOURCE_BYTES + 1) as u64)
                .unwrap(),
            "aggregate" => {
                fs::OpenOptions::new()
                    .write(true)
                    .open(previous.join(artifacts::filename(resource())))
                    .unwrap()
                    .set_len(MAX_RESOURCE_BYTES as u64)
                    .unwrap();
                let extra =
                    previous.join(artifacts::filename(SnapshotResource::Device(typed(100))));
                fs::write(&extra, b"x").unwrap();
                fs::set_permissions(extra, fs::Permissions::from_mode(0o600)).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(fixture.open().is_err(), "{damage}");
    }
}

#[test]
fn strict_manifest_refuses_inconsistent_floor_schema_unknowns_and_size() {
    for damage in ["floor", "schema", "unknown", "size", "duplicate"] {
        let fixture = Fixture::new();
        let deployment = fixture.provision();
        apply(&deployment, 1, false, b"one").unwrap();
        drop(deployment);
        let path = fixture.root.join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match damage {
            "floor" => manifest["floor"] = serde_json::json!(0),
            "schema" => manifest["version"] = serde_json::json!(2),
            "unknown" => manifest["unexpected"] = serde_json::json!(true),
            "size" => {
                fs::write(&path, vec![b' '; MAX_MANIFEST_BYTES + 1]).unwrap();
                assert!(fixture.open().is_err());
                continue;
            }
            "duplicate" => {
                let text = String::from_utf8(fs::read(&path).unwrap())
                    .unwrap()
                    .replacen('{', "{\"version\":1,", 1);
                fs::write(&path, text).unwrap();
                assert!(fixture.open().is_err());
                continue;
            }
            _ => unreachable!(),
        }
        fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(fixture.open().is_err(), "{damage}");
    }
}

#[test]
fn cancellation_at_prepared_manifest_preserves_old_but_after_commit_is_applied() {
    for phase in ["manifest-prepared", "manifest-durable"] {
        let fixture = Fixture::new();
        let deployment = fixture.provision();
        apply(&deployment, 1, false, b"one").unwrap();
        let cancel = Arc::new(Cancellation::default());
        let signal = Arc::clone(&cancel);
        hook(move |at| {
            if at == phase {
                signal.cancel();
            }
            Ok(())
        });
        let (bytes, artifacts) = candidate(2, false, b"two");
        let outcome = deployment.activate(bytes, artifacts, &cancel);
        clear_hook();
        if phase == "manifest-prepared" {
            assert_eq!(failure(outcome), ErrorCode::Cancelled);
            assert_version(&deployment, 1, b"one");
        } else {
            assert_eq!(outcome.unwrap().sequence.get(), 2);
            assert_version(&deployment, 2, b"two");
        }
        drop(deployment);
        let recovered = fixture.open().unwrap();
        if phase == "manifest-prepared" {
            assert_version(&recovered, 1, b"one");
        } else {
            assert_version(&recovered, 2, b"two");
        }
    }
}

#[test]
fn precommit_write_flush_disk_full_and_cancel_preserve_active() {
    for phase in [
        "file-created",
        "file-written",
        "file-synced",
        "stage-envelope",
        "stage-resource",
        "stage-durable",
        "manifest-prepared",
    ] {
        let fixture = Fixture::new();
        let deployment = fixture.provision();
        apply(&deployment, 1, false, b"one").unwrap();
        hook(move |at| {
            if at == phase {
                Err(DeploymentError(ErrorCode::Filesystem))
            } else {
                Ok(())
            }
        });
        assert_eq!(
            failure(apply(&deployment, 2, false, b"two")),
            ErrorCode::Filesystem
        );
        clear_hook();
        assert_version(&deployment, 1, b"one");
        drop(deployment);
        assert_version(&fixture.open().unwrap(), 1, b"one");
    }
    let fixture = Fixture::new();
    let deployment = fixture.provision();
    apply(&deployment, 1, false, b"one").unwrap();
    hook(|at| {
        if at == "file-created" {
            // Real unprivileged kernel ENOSPC; do not fill or mount a host filesystem.
            let error = fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .unwrap()
                .write_all(b"x")
                .unwrap_err();
            assert_eq!(error.raw_os_error(), Some(28));
            return Err(DeploymentError(ErrorCode::Filesystem));
        }
        Ok(())
    });
    assert!(apply(&deployment, 2, false, b"two").is_err());
    clear_hook();
    assert_version(&deployment, 1, b"one");
    let cancel = Arc::new(Cancellation::default());
    let signal = Arc::clone(&cancel);
    hook(move |at| {
        if at == "stage-durable" {
            signal.cancel();
        }
        Ok(())
    });
    let (bytes, artifacts) = candidate(2, false, b"two");
    assert_eq!(
        failure(deployment.activate(bytes, artifacts, &cancel)),
        ErrorCode::Cancelled
    );
    clear_hook();
    drop(deployment);
    assert_version(&fixture.open().unwrap(), 1, b"one");
}

#[test]
fn uncertain_commit_freezes_writer_and_recovery_finds_committed_generation() {
    for phase in ["manifest-renamed", "manifest-durable"] {
        let fixture = Fixture::new();
        let deployment = fixture.provision();
        apply(&deployment, 1, false, b"one").unwrap();
        hook(move |at| {
            if at == phase {
                Err(DeploymentError(ErrorCode::Filesystem))
            } else {
                Ok(())
            }
        });
        assert_eq!(
            failure(apply(&deployment, 2, false, b"two")),
            ErrorCode::UncertainCommit
        );
        clear_hook();
        assert_version(&deployment, 1, b"one");
        assert_eq!(
            failure(apply(&deployment, 3, false, b"three")),
            ErrorCode::RecoveryRequired
        );
        drop(deployment);
        let recovered = fixture.open().unwrap();
        assert_version(&recovered, 2, b"two");
        assert_eq!(
            failure(apply(&recovered, 2, false, b"replay")),
            ErrorCode::Snapshot(snapshot::SnapshotErrorCode::Replay)
        );
    }
}

#[test]
fn crash_child() {
    let Ok(root) = std::env::var("DROUTEX_TEST_CRASH_ROOT") else {
        return;
    };
    let phase = std::env::var("DROUTEX_TEST_CRASH_PHASE").unwrap();
    let mut occurrence: usize = std::env::var("DROUTEX_TEST_CRASH_NTH")
        .unwrap()
        .parse()
        .unwrap();
    let recovery = std::env::var("DROUTEX_TEST_RECOVERY").is_ok();
    let deployment = if recovery {
        None
    } else {
        Some(Deployment::open(Path::new(&root), typed(1), trust(false)).unwrap())
    };
    hook(move |at| {
        if at == phase {
            occurrence -= 1;
            if occurrence == 0 {
                std::process::exit(88);
            }
        }
        Ok(())
    });
    if recovery {
        Deployment::open(Path::new(&root), typed(1), trust(false)).unwrap();
    } else {
        apply(deployment.as_ref().unwrap(), 2, false, b"two").unwrap();
    }
    panic!("crash point not reached");
}

fn uncertain_third(fixture: &Fixture) {
    let deployment = fixture.provision();
    apply(&deployment, 1, false, b"one").unwrap();
    apply(&deployment, 2, false, b"two").unwrap();
    hook(|phase| {
        if phase == "manifest-renamed" {
            Err(DeploymentError(ErrorCode::Filesystem))
        } else {
            Ok(())
        }
    });
    assert_eq!(
        failure(apply(&deployment, 3, false, b"three")),
        ErrorCode::UncertainCommit
    );
    clear_hook();
}

#[test]
fn recovery_sync_failure_cannot_delete_a_formerly_retained_slot() {
    let fixture = Fixture::new();
    uncertain_third(&fixture);
    let formerly_retained = fixture.root.join("slot-0/snapshot");
    let bytes = fs::read(&formerly_retained).unwrap();
    hook(|phase| {
        if phase == "recovery-before-sync" {
            Err(DeploymentError(ErrorCode::Filesystem))
        } else {
            Ok(())
        }
    });
    assert_eq!(failure(fixture.open()), ErrorCode::Filesystem);
    clear_hook();
    assert_eq!(fs::read(&formerly_retained).unwrap(), bytes);
    assert_version(&fixture.open().unwrap(), 3, b"three");
    assert!(!formerly_retained.exists());
}

#[test]
fn real_process_exit_during_uncertain_recovery_preserves_complete_selection() {
    for phase in [
        "recovery-before-sync",
        "recovery-durable",
        "recovery-cleaned",
    ] {
        let fixture = Fixture::new();
        uncertain_third(&fixture);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "deployment::tests::linux::crash_child",
                "--test-threads=1",
            ])
            .env("DROUTEX_TEST_CRASH_ROOT", &fixture.root)
            .env("DROUTEX_TEST_CRASH_PHASE", phase)
            .env("DROUTEX_TEST_CRASH_NTH", "1")
            .env("DROUTEX_TEST_RECOVERY", "1")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(88),
            "{phase}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fixture.root.join("slot-0").exists(),
            phase != "recovery-cleaned"
        );
        assert_version(&fixture.open().unwrap(), 3, b"three");
    }
}

#[test]
fn real_process_exit_at_every_durable_transition_recovers_complete_generation() {
    let phases = [
        ("stage-directory", 1),
        ("file-created", 1),
        ("file-written", 1),
        ("file-synced", 1),
        ("stage-envelope", 1),
        ("stage-resource", 1),
        ("stage-durable", 1),
        ("file-created", 3),
        ("file-written", 3),
        ("file-synced", 3),
        ("manifest-prepared", 1),
        ("manifest-renamed", 1),
        ("manifest-durable", 1),
    ];
    for (phase, nth) in phases {
        let fixture = Fixture::new();
        let deployment = fixture.provision();
        apply(&deployment, 1, false, b"one").unwrap();
        drop(deployment);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "deployment::tests::linux::crash_child",
                "--test-threads=1",
            ])
            .env("DROUTEX_TEST_CRASH_ROOT", &fixture.root)
            .env("DROUTEX_TEST_CRASH_PHASE", phase)
            .env("DROUTEX_TEST_CRASH_NTH", nth.to_string())
            .output()
            .unwrap();
        assert_eq!(
            status.status.code(),
            Some(88),
            "{phase}/{nth}: {}",
            String::from_utf8_lossy(&status.stderr)
        );
        let deployment = fixture.open().unwrap();
        if matches!(phase, "manifest-renamed" | "manifest-durable") {
            assert_version(&deployment, 2, b"two");
        } else {
            assert_version(&deployment, 1, b"one");
        }
    }
}
