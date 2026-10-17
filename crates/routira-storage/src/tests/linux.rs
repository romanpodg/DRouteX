use super::*;
use sqlx::{
    Connection, SqlSafeStr,
    migrate::{Migration, MigrationType, Migrator},
    sqlite::SqliteConnectOptions,
};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt, symlink},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/storage-tests");
        fs::create_dir_all(&parent).unwrap();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = parent.join(format!("{}-{unique}", std::process::id()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        Self(directory)
    }
    fn data(&self) -> PathBuf {
        self.0.join("data")
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn full_write(number: usize) -> ConfigurationWrite {
    ConfigurationWrite::parse(
        typed(number),
        FIXTURE.to_vec(),
        InputFormat::Json,
        &catalog(),
        vec![CredentialVerifierLink {
            credential: typed(20),
            reference: typed(99),
        }],
    )
    .unwrap()
}

async fn register_catalogs(repository: &Repository) {
    repository
        .register_catalog(CatalogRevision::Filter(typed(101)), [1; 32])
        .await
        .unwrap();
    repository
        .register_catalog(CatalogRevision::Service(typed(100)), [2; 32])
        .await
        .unwrap();
}

async fn external(directory: &Path) -> SqliteConnection {
    SqliteConnectOptions::new()
        .filename(directory.join("routira.db"))
        .create_if_missing(false)
        .disable_statement_logging()
        .connect()
        .await
        .unwrap()
}

#[test]
fn complete_revision_reopens_with_independent_dns_gateway_and_verifier_links() {
    let directory = Directory::new();
    run(async {
        let storage = Storage::open(directory.data()).await.unwrap();
        let repository = storage.repository();
        register_catalogs(&repository).await;
        repository
            .save_configuration(full_write(500))
            .await
            .unwrap();
        let duplicate = repository
            .save_configuration(empty_write(500))
            .await
            .unwrap_err();
        assert_eq!(duplicate.code(), StorageErrorCode::Conflict);
        storage.close().await.unwrap();
        assert_eq!(
            repository
                .load_configuration(typed(500))
                .await
                .err()
                .unwrap()
                .code(),
            StorageErrorCode::Closed
        );

        let storage = Storage::open(directory.data()).await.unwrap();
        let repository = storage.repository();
        let stored = repository
            .load_configuration(typed(500))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(format!("{stored:?}"), "StoredConfiguration([REDACTED])");
        let checked = stored.check(&catalog()).unwrap();
        let document = checked.document();
        assert_eq!(document.endpoints.len(), 3);
        assert_ne!(document.endpoints[0].profile, document.endpoints[2].profile);
        assert_eq!(document.legacy_grants.len(), 2);
        let gateway = &document.gateway_grants[0];
        assert_ne!(gateway.listener, document.legacy_grants[0].listener);
        assert_eq!(gateway.targets[0].value, "example.com");
        assert_eq!(gateway.ports[0].from, 443);
        assert_eq!(gateway.ports[0].to, 443);
        assert_eq!(gateway.outbounds, [typed(70), typed(71)]);
        assert_eq!(
            document.private_records[0].name,
            document.private_records[1].name
        );
        assert_ne!(
            document.private_records[0].profile,
            document.private_records[1].profile
        );
        assert_eq!(
            stored.credential_links(),
            &[CredentialVerifierLink {
                credential: typed(20),
                reference: typed(99)
            }]
        );
        assert_eq!(
            repository
                .catalog_digest(CatalogRevision::Filter(typed(101)))
                .await
                .unwrap(),
            Some([1; 32])
        );
        assert_eq!(
            repository
                .catalog_digest(CatalogRevision::Service(typed(101)))
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            repository
                .register_catalog(CatalogRevision::Filter(typed(101)), [3; 32])
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::Conflict
        );
        assert_eq!(
            stored
                .check(&ReferenceCatalog::default())
                .unwrap_err()
                .code(),
            StorageErrorCode::InvalidConfiguration
        );
        storage.close().await.unwrap();
    });
}

#[test]
fn late_catalog_constraint_rolls_back_all_resource_indexes() {
    let directory = Directory::new();
    run(async {
        let storage = Storage::open(directory.data()).await.unwrap();
        let repository = storage.repository();
        assert_eq!(
            repository
                .save_configuration(full_write(500))
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::Conflict
        );
        assert!(
            repository
                .load_configuration(typed(500))
                .await
                .unwrap()
                .is_none()
        );
        let mut connection = external(&directory.data()).await;
        for query in [
            "SELECT count(*) FROM configuration_profiles",
            "SELECT count(*) FROM endpoint_assignments",
            "SELECT count(*) FROM credential_metadata",
            "SELECT count(*) FROM legacy_dns_grants",
            "SELECT count(*) FROM gateway_network_grants",
        ] {
            let count: i64 = sqlx::query_scalar(query)
                .fetch_one(&mut connection)
                .await
                .unwrap();
            assert_eq!(count, 0, "{query}");
        }
        connection.close().await.unwrap();
        register_catalogs(&repository).await;
        repository
            .save_configuration(full_write(500))
            .await
            .unwrap();
        storage.close().await.unwrap();
    });
}

#[test]
fn sqlite_full_leaves_prior_revision_and_all_indexes_intact() {
    let directory = Directory::new();
    run(async {
        let paths = paths::prepare(directory.data()).unwrap();
        let mut connection = migrations::initialize(&paths).await.unwrap();
        database::save(&mut connection, empty_write(500))
            .await
            .unwrap();
        sqlx::query("PRAGMA max_page_count=1")
            .execute(&mut connection)
            .await
            .unwrap();
        let mut input = b"{\"schemaVersion\":1}".to_vec();
        input.extend(std::iter::repeat_n(b' ', 200_000));
        let write = ConfigurationWrite::parse(
            typed(501),
            input,
            InputFormat::Json,
            &ReferenceCatalog::default(),
            vec![],
        )
        .unwrap();
        let error = database::save(&mut connection, write).await.unwrap_err();
        assert_eq!(error.code(), StorageErrorCode::DiskFull);
        assert!(
            database::load(&mut connection, typed(501))
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            database::load(&mut connection, typed(500))
                .await
                .unwrap()
                .is_some()
        );
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        assert_eq!(integrity, "ok");
        connection.close().await.unwrap();
        drop(paths);
        let storage = Storage::open(directory.data()).await.unwrap();
        assert!(
            storage
                .repository()
                .load_configuration(typed(500))
                .await
                .unwrap()
                .is_some()
        );
        storage.close().await.unwrap();
    });
}

#[test]
fn external_busy_and_single_owner_lock_fail_explicitly_then_recover() {
    let directory = Directory::new();
    run(async {
        let storage = Storage::open(directory.data()).await.unwrap();
        let error = Storage::open(directory.data()).await.err().unwrap();
        assert_eq!(error.code(), StorageErrorCode::Busy);
        let repository = storage.repository();
        let mut other = external(&directory.data()).await;
        let held = other.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let started = Instant::now();
        let error = repository
            .save_configuration(empty_write(500))
            .await
            .unwrap_err();
        assert_eq!(error.code(), StorageErrorCode::Busy);
        assert!(started.elapsed() < RESPONSE_TIMEOUT);
        held.rollback().await.unwrap();
        assert!(
            repository
                .load_configuration(typed(500))
                .await
                .unwrap()
                .is_none()
        );
        repository
            .save_configuration(empty_write(500))
            .await
            .unwrap();
        other.close().await.unwrap();
        storage.close().await.unwrap();
        Storage::open(directory.data())
            .await
            .unwrap()
            .close()
            .await
            .unwrap();
    });
}

#[test]
fn expired_and_abandoned_queued_commands_never_start_writes() {
    let directory = Directory::new();
    run(async {
        let storage = Storage::open(directory.data()).await.unwrap();
        let repository = storage.repository();
        let (reply, response) = oneshot::channel();
        repository
            .sender
            .send(Command {
                deadline: Instant::now() - Duration::from_secs(1),
                operation: Operation::Save {
                    write: Box::new(empty_write(500)),
                    reply,
                },
            })
            .await
            .unwrap_or_else(|_| panic!("writer closed"));
        assert_eq!(
            response.await.unwrap().unwrap_err().code(),
            StorageErrorCode::DeadlineOutcomeUnknown
        );
        let (reply, response) = oneshot::channel();
        drop(response);
        repository
            .sender
            .send(Command {
                deadline: Instant::now() + RESPONSE_TIMEOUT,
                operation: Operation::Save {
                    write: Box::new(empty_write(501)),
                    reply,
                },
            })
            .await
            .unwrap_or_else(|_| panic!("writer closed"));
        assert!(
            repository
                .load_configuration(typed(500))
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            repository
                .load_configuration(typed(501))
                .await
                .unwrap()
                .is_none()
        );
        storage.close().await.unwrap();
    });
}

#[test]
fn private_modes_symlink_hardlink_and_unrelated_database_are_rejected() {
    let directory = Directory::new();
    run(async {
        let unsafe_directory = directory.0.join("unsafe");
        fs::create_dir(&unsafe_directory).unwrap();
        fs::set_permissions(&unsafe_directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            Storage::open(unsafe_directory.clone())
                .await
                .err()
                .unwrap()
                .code(),
            StorageErrorCode::UnsafePath
        );
        assert_eq!(
            fs::metadata(unsafe_directory).unwrap().permissions().mode() & 0o777,
            0o755
        );
        let real = directory.0.join("real");
        fs::DirBuilder::new().mode(0o700).create(&real).unwrap();
        let link = directory.0.join("link");
        symlink(&real, &link).unwrap();
        assert_eq!(
            Storage::open(link).await.err().unwrap().code(),
            StorageErrorCode::UnsafePath
        );
        let target = directory.0.join("target");
        fs::write(&target, b"preserve-this-file").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        symlink(&target, real.join("routira.db")).unwrap();
        assert_eq!(
            Storage::open(real.clone()).await.err().unwrap().code(),
            StorageErrorCode::UnsafePath
        );
        fs::remove_file(real.join("routira.db")).unwrap();
        fs::hard_link(&target, real.join("routira.db")).unwrap();
        assert_eq!(
            Storage::open(real).await.err().unwrap().code(),
            StorageErrorCode::UnsafePath
        );
        assert_eq!(fs::read(target).unwrap(), b"preserve-this-file");

        let paths = paths::prepare(directory.data()).unwrap();
        let mut unrelated = migrations::options(&paths).connect().await.unwrap();
        sqlx::query("CREATE TABLE unrelated (value INTEGER)")
            .execute(&mut unrelated)
            .await
            .unwrap();
        assert_eq!(
            migrations::initialize(&paths).await.err().unwrap().code(),
            StorageErrorCode::Migration
        );
        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&mut unrelated)
            .await
            .unwrap();
        assert_eq!(mode, "delete");
        unrelated.close().await.unwrap();
    });
}

#[test]
fn wal_files_remain_private_and_preexisting_sidecar_symlink_is_rejected() {
    let directory = Directory::new();
    run(async {
        let storage = Storage::open(directory.data()).await.unwrap();
        assert_eq!(
            fs::metadata(directory.data()).unwrap().permissions().mode() & 0o777,
            0o700
        );
        for name in [
            "routira.db",
            "routira.db-wal",
            "routira.db-shm",
            "routira.lock",
        ] {
            assert_eq!(
                fs::metadata(directory.data().join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600,
                "{name}"
            );
        }
        storage.close().await.unwrap();
        let target = directory.0.join("unrelated");
        fs::write(&target, b"preserved").unwrap();
        symlink(&target, directory.data().join("routira.db-wal")).unwrap();
        assert_eq!(
            Storage::open(directory.data()).await.err().unwrap().code(),
            StorageErrorCode::UnsafePath
        );
        assert_eq!(fs::read(target).unwrap(), b"preserved");
    });
}

#[test]
fn upgrade_backup_includes_wal_and_failed_migration_preserves_prior_schema() {
    let directory = Directory::new();
    run(async {
        let paths = paths::prepare(directory.data()).unwrap();
        let mut connection = migrations::options(&paths).connect().await.unwrap();
        let mut first = migrations::migrations().migrations.into_owned();
        first.truncate(1);
        migrations::apply(&mut connection, &paths, &Migrator::with_migrations(first))
            .await
            .unwrap();
        sqlx::query("INSERT INTO configuration_revisions (id,format,document) VALUES (?,0,?)")
            .bind(typed::<ConfigurationRevisionId>(500).as_bytes().as_slice())
            .bind(b"{\"schemaVersion\":1}".as_slice())
            .execute(&mut connection)
            .await
            .unwrap();
        migrations::apply(&mut connection, &paths, &migrations::migrations())
            .await
            .unwrap();
        let backups: Vec<_> = fs::read_dir(&paths.directory)
            .unwrap()
            .map(Result::unwrap)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".backup.db"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            backups[0].metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
        let mut backup = SqliteConnectOptions::new()
            .filename(backups[0].path())
            .read_only(true)
            .disable_statement_logging()
            .connect()
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM configuration_revisions")
            .fetch_one(&mut backup)
            .await
            .unwrap();
        assert_eq!(count, 1);
        let version: i64 = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
            .fetch_one(&mut backup)
            .await
            .unwrap();
        assert_eq!(version, 1);
        backup.close().await.unwrap();

        let mut broken = migrations::migrations().migrations.into_owned();
        broken.push(Migration::new(
            3,
            "failure probe".into(),
            MigrationType::Simple,
            "CREATE TABLE incomplete (id INTEGER); INSERT INTO absent_table VALUES (1);"
                .into_sql_str(),
            false,
        ));
        assert_eq!(
            migrations::apply(&mut connection, &paths, &Migrator::with_migrations(broken))
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::Migration
        );
        let incomplete: i64 =
            sqlx::query_scalar("SELECT count(*) FROM sqlite_schema WHERE name='incomplete'")
                .fetch_one(&mut connection)
                .await
                .unwrap();
        assert_eq!(incomplete, 0);
        assert!(
            database::load(&mut connection, typed(500))
                .await
                .unwrap()
                .is_some()
        );
        let mut changed = migrations::migrations().migrations.into_owned();
        changed[0] = Migration::new(
            1,
            "changed".into(),
            MigrationType::Simple,
            "CREATE TABLE changed (id INTEGER)".into_sql_str(),
            false,
        );
        assert_eq!(
            migrations::apply(&mut connection, &paths, &Migrator::with_migrations(changed))
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::Migration
        );
        for invalid in [0_i64, 2] {
            sqlx::query("UPDATE _sqlx_migrations SET success=? WHERE version=1")
                .bind(invalid)
                .execute(&mut connection)
                .await
                .unwrap();
            assert_eq!(
                migrations::apply(&mut connection, &paths, &migrations::migrations())
                    .await
                    .unwrap_err()
                    .code(),
                StorageErrorCode::Migration
            );
        }
        connection.close().await.unwrap();
    });
}
