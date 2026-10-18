use super::*;
use crate::tests::secrets::{context, keys, record};
use droutex_security::{EncryptedSecret, MAX_ENVELOPE_BYTES, SecretErrorCode};

#[test]
fn schema3_upgrade_preserves_draft_metadata_without_fabricating_secret_records() {
    let directory = Directory::new();
    run(async {
        let paths = paths::prepare(directory.data()).unwrap();
        let mut connection = migrations::options(&paths).connect().await.unwrap();
        let mut previous = migrations::migrations().migrations.into_owned();
        previous.truncate(3);
        migrations::apply(
            &mut connection,
            &paths,
            &Migrator::with_migrations(previous),
        )
        .await
        .unwrap();
        database::register_catalog(
            &mut connection,
            CatalogRevision::Filter(typed(101)),
            [1; 32],
        )
        .await
        .unwrap();
        database::register_catalog(
            &mut connection,
            CatalogRevision::Service(typed(100)),
            [2; 32],
        )
        .await
        .unwrap();
        database::save(&mut connection, full_write(500), revision(500, None))
            .await
            .unwrap();
        connection.close().await.unwrap();
        drop(paths);
        let storage = Storage::open(directory.data()).await.unwrap();
        assert_eq!(
            storage.repository().draft_head().await.unwrap(),
            Some(typed(500))
        );
        let loaded = storage
            .repository()
            .load_configuration(typed(500))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.credential_links()[0].reference, typed(99));
        assert!(
            storage
                .repository()
                .load_secret(typed(99))
                .await
                .unwrap()
                .is_none()
        );
        storage.close().await.unwrap();
        let backup = fs::read_dir(directory.data())
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p.to_string_lossy().ends_with(".backup.db"))
            .unwrap();
        let mut backup = SqliteConnectOptions::new()
            .filename(backup)
            .read_only(true)
            .connect()
            .await
            .unwrap();
        let version: i64 = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
            .fetch_one(&mut backup)
            .await
            .unwrap();
        assert_eq!(version, 3);
        backup.close().await.unwrap();
    });
}

#[test]
fn encrypted_database_backup_restore_and_failed_reencryption_preserve_data() {
    let directory = Directory::new();
    run(async {
        let storage = Storage::open(directory.data()).await.unwrap();
        let repository = storage.repository();
        let old = keys(1, &[(1, 7)]);
        let rotated = keys(2, &[(1, 7), (2, 9)]);
        let original = record(&old);
        repository.insert_secret(original.clone()).await.unwrap();
        register_catalogs(&repository).await;
        repository
            .save_configuration(full_write(500), revision(500, None))
            .await
            .unwrap();
        let mut db = external(&directory.data()).await;
        sqlx::query("CREATE TRIGGER reject_reencryption AFTER UPDATE ON encrypted_secrets BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END").execute(&mut db).await.unwrap();
        let changed = rotated.reencrypt(context(), &original).unwrap();
        assert_eq!(
            repository
                .replace_secret(original.clone(), changed.clone())
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::Conflict
        );
        assert_eq!(
            repository.load_secret(typed(99)).await.unwrap().unwrap(),
            original
        );
        sqlx::query("DROP TRIGGER reject_reencryption")
            .execute(&mut db)
            .await
            .unwrap();
        db.close().await.unwrap();
        storage.close().await.unwrap();
        let restored = directory.0.join("restored");
        fs::DirBuilder::new().mode(0o700).create(&restored).unwrap();
        fs::copy(
            directory.data().join("droutex.db"),
            restored.join("droutex.db"),
        )
        .unwrap();
        // Rotate the current copy while the retained backup still needs the old key.
        let storage = Storage::open(directory.data()).await.unwrap();
        storage
            .repository()
            .replace_secret(original, changed)
            .await
            .unwrap();
        storage.close().await.unwrap();
        let restored_storage = Storage::open(restored.clone()).await.unwrap();
        let saved = restored_storage
            .repository()
            .load_secret(typed(99))
            .await
            .unwrap()
            .unwrap();
        assert!(rotated.open(context(), &saved).is_ok());
        assert_eq!(
            keys(2, &[(2, 9)])
                .open(context(), &saved)
                .unwrap_err()
                .code(),
            SecretErrorCode::MissingKey
        );
        assert_eq!(
            keys(1, &[(1, 8)])
                .open(context(), &saved)
                .unwrap_err()
                .code(),
            SecretErrorCode::Authentication
        );
        assert!(
            restored_storage
                .repository()
                .load_configuration(typed(500))
                .await
                .unwrap()
                .is_some()
        );
        restored_storage.close().await.unwrap();
        for root in [directory.data(), restored] {
            for entry in fs::read_dir(root).unwrap() {
                let bytes = fs::read(entry.unwrap().path()).unwrap();
                assert!(
                    !bytes
                        .windows(b"synthetic-storage-secret-canary-2026".len())
                        .any(|w| w == b"synthetic-storage-secret-canary-2026")
                );
                for key in [7, 9] {
                    assert!(!bytes.windows(32).any(|w| w == [key; 32]));
                }
            }
        }
    });
}

#[test]
fn corrupt_blob_lengths_headers_and_reference_substitution_fail_closed() {
    let directory = Directory::new();
    run(async {
        let storage = Storage::open(directory.data()).await.unwrap();
        let repository = storage.repository();
        let original = record(&keys(1, &[(1, 7)]));
        repository.insert_secret(original.clone()).await.unwrap();
        let mut db = external(&directory.data()).await;
        sqlx::query("PRAGMA ignore_check_constraints=ON")
            .execute(&mut db)
            .await
            .unwrap();
        let mut invalid_header = original.as_bytes().to_vec();
        invalid_header[0] = 0;
        let mut other_ref = original.as_bytes().to_vec();
        other_ref[40] ^= 1;
        assert!(EncryptedSecret::from_owned_bytes(other_ref.clone()).is_ok());
        for bad in [
            vec![0; MAX_ENVELOPE_BYTES + 1],
            vec![],
            invalid_header,
            other_ref,
        ] {
            sqlx::query("UPDATE encrypted_secrets SET envelope=?")
                .bind(bad)
                .execute(&mut db)
                .await
                .unwrap();
            assert_eq!(
                repository.load_secret(typed(99)).await.unwrap_err().code(),
                StorageErrorCode::CorruptData
            );
        }
        db.close().await.unwrap();
        storage.close().await.unwrap();
    });
}
