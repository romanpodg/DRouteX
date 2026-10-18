use crate::{StorageError, StorageErrorCode, error::database};
use droutex_common::SecretRefId;
use droutex_security::{EncryptedSecret, MAX_ENVELOPE_BYTES};
use sqlx::{Row, SqliteConnection};

pub(crate) async fn insert(
    db: &mut SqliteConnection,
    record: EncryptedSecret,
) -> Result<(), StorageError> {
    sqlx::query("INSERT INTO encrypted_secrets (id, envelope) VALUES (?, ?)")
        .bind(record.context().reference.as_bytes().as_slice())
        .bind(record.as_bytes())
        .execute(db)
        .await
        .map_err(database)?;
    Ok(())
}

pub(crate) async fn replace(
    db: &mut SqliteConnection,
    expected: EncryptedSecret,
    replacement: EncryptedSecret,
) -> Result<(), StorageError> {
    let result = sqlx::query("UPDATE encrypted_secrets SET envelope=? WHERE id=? AND envelope=?")
        .bind(replacement.as_bytes())
        .bind(expected.context().reference.as_bytes().as_slice())
        .bind(expected.as_bytes())
        .execute(db)
        .await
        .map_err(database)?;
    if result.rows_affected() != 1 {
        return Err(StorageError::new(StorageErrorCode::Conflict));
    }
    Ok(())
}

pub(crate) async fn load(
    db: &mut SqliteConnection,
    id: SecretRefId,
) -> Result<Option<EncryptedSecret>, StorageError> {
    // Bound allocation even for a corrupted/replaced DB with missing constraints.
    let row = sqlx::query("SELECT CASE WHEN typeof(envelope)='blob' AND length(envelope) BETWEEN 75 AND ? THEN envelope END AS envelope FROM encrypted_secrets WHERE id=?")
        .bind(MAX_ENVELOPE_BYTES as i64).bind(id.as_bytes().as_slice())
        .fetch_optional(db).await.map_err(database)?;
    row.map(|row| {
        let bytes: Option<Vec<u8>> = row.try_get("envelope").map_err(database)?;
        let record =
            EncryptedSecret::from_owned_bytes(bytes.ok_or_else(corrupt)?).map_err(|_| corrupt())?;
        if record.context().reference != id {
            return Err(corrupt());
        }
        Ok(record)
    })
    .transpose()
}

fn corrupt() -> StorageError {
    StorageError::new(StorageErrorCode::CorruptData)
}
