use crate::{
    StorageError, StorageErrorCode,
    error::database,
    paths::{self, Paths},
};
use sqlx::{
    ConnectOptions, Row, SqlSafeStr, SqliteConnection,
    migrate::{Migration, MigrationType, Migrator},
    sqlite::{SqliteConnectOptions, SqliteSynchronous},
};
use std::{borrow::Cow, time::Duration};

pub(crate) fn migrations() -> Migrator {
    Migrator::with_migrations(vec![
        Migration::new(
            1,
            Cow::Borrowed("configuration"),
            MigrationType::Simple,
            include_str!("../../../migrations/0001_configuration.sql").into_sql_str(),
            false,
        ),
        Migration::new(
            2,
            Cow::Borrowed("credential verifier references"),
            MigrationType::Simple,
            include_str!("../../../migrations/0002_credential_verifier_refs.sql").into_sql_str(),
            false,
        ),
        Migration::new(
            3,
            Cow::Borrowed("draft revision provenance"),
            MigrationType::Simple,
            include_str!("../../../migrations/0003_draft_provenance.sql").into_sql_str(),
            false,
        ),
    ])
}

pub(crate) fn options(paths: &Paths) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(&paths.database)
        .create_if_missing(false)
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(1))
        .command_buffer_size(8)
        .row_buffer_size(8)
        .pragma("trusted_schema", "OFF")
        .disable_statement_logging()
}

pub(crate) async fn initialize(paths: &Paths) -> Result<SqliteConnection, StorageError> {
    let mut connection = options(paths).connect().await.map_err(database)?;
    let version: String = sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(&mut connection)
        .await
        .map_err(database)?;
    if version != "3.53.2" {
        return Err(StorageError::new(StorageErrorCode::Unavailable));
    }
    apply(&mut connection, paths, &migrations()).await?;
    let directory = paths.directory.clone();
    tokio::task::spawn_blocking(move || paths::verify_files(directory))
        .await
        .map_err(|_| StorageError::new(StorageErrorCode::Filesystem))??;
    Ok(connection)
}

pub(crate) async fn apply(
    connection: &mut SqliteConnection,
    paths: &Paths,
    migrator: &Migrator,
) -> Result<(), StorageError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name='_sqlx_migrations'",
    )
    .fetch_one(&mut *connection)
    .await
    .map_err(database)?;
    let applied = if count == 0 {
        let tables: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_one(&mut *connection)
        .await
        .map_err(database)?;
        if tables != 0 {
            return Err(StorageError::new(StorageErrorCode::Migration));
        }
        0
    } else {
        let rows = sqlx::query("SELECT version, success, CASE WHEN length(checksum)=48 THEN checksum END AS checksum FROM _sqlx_migrations ORDER BY version LIMIT ?")
            .bind(i64::try_from(migrator.migrations.len() + 1).map_err(|_| StorageError::new(StorageErrorCode::Migration))?)
            .fetch_all(&mut *connection).await.map_err(database)?;
        if rows.len() > migrator.migrations.len() {
            return Err(StorageError::new(StorageErrorCode::Migration));
        }
        for (row, expected) in rows.iter().zip(migrator.iter()) {
            let version: i64 = row.try_get("version").map_err(database)?;
            let success: i64 = row.try_get("success").map_err(database)?;
            let checksum: Option<Vec<u8>> = row.try_get("checksum").map_err(database)?;
            if version != expected.version
                || success != 1
                || checksum.as_deref() != Some(expected.checksum.as_ref())
            {
                return Err(StorageError::new(StorageErrorCode::Migration));
            }
        }
        rows.len()
    };
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode=WAL")
        .fetch_one(&mut *connection)
        .await
        .map_err(database)?;
    if mode != "wal" {
        return Err(StorageError::new(StorageErrorCode::Unavailable));
    }
    if applied != 0 && applied < migrator.migrations.len() {
        let directory = paths.directory.clone();
        let version = migrator.migrations[applied - 1].version;
        let target = tokio::task::spawn_blocking(move || paths::backup_target(directory, version))
            .await
            .map_err(|_| StorageError::new(StorageErrorCode::Filesystem))??;
        let filename = target
            .pending
            .to_str()
            .ok_or_else(|| StorageError::new(StorageErrorCode::UnsafePath))?;
        sqlx::query("VACUUM main INTO ?")
            .bind(filename)
            .execute(&mut *connection)
            .await
            .map_err(database)?;
        tokio::task::spawn_blocking(move || paths::finish_backup(target))
            .await
            .map_err(|_| StorageError::new(StorageErrorCode::Filesystem))??;
    }
    // SQLx's direct entry avoids the generic Acquire future's Send limitation.
    migrator
        .run_direct(None, connection, false)
        .await
        .map_err(|_| StorageError::new(StorageErrorCode::Migration))
}
