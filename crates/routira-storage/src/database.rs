use crate::{
    CatalogRevision, ConfigurationWrite, CredentialVerifierLink, StorageError, StorageErrorCode,
    StoredConfiguration, error::database,
};
use routira_common::{ConfigurationRevisionId, ResolverCredentialId, SecretRefId};
use routira_config::{CredentialState, InputFormat, MAX_RESOURCE_ENTRIES};
use sqlx::{Connection, Row, SqliteConnection};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) async fn save(
    connection: &mut SqliteConnection,
    write: ConfigurationWrite,
) -> Result<(), StorageError> {
    let ConfigurationWrite {
        id,
        format,
        input,
        checked,
        links,
    } = write;
    let document = checked.document();
    let links: BTreeMap<_, _> = links
        .into_iter()
        .map(|link| (link.credential, link.reference))
        .collect();
    let mut transaction = connection.begin().await.map_err(database)?;
    let db = &mut *transaction;
    let revision = id.as_bytes().as_slice();
    let format = match format {
        InputFormat::Json => 0_i64,
        InputFormat::Yaml => 1,
        InputFormat::Toml => 2,
    };
    sqlx::query("INSERT INTO configuration_revisions (id, format, document) VALUES (?, ?, ?)")
        .bind(revision)
        .bind(format)
        .bind(input)
        .execute(&mut *db)
        .await
        .map_err(database)?;
    let mut references = BTreeSet::new();
    for profile in &document.profiles {
        sqlx::query("INSERT INTO configuration_profiles (revision, id) VALUES (?, ?)")
            .bind(revision)
            .bind(profile.id.as_bytes().as_slice())
            .execute(&mut *db)
            .await
            .map_err(database)?;
        references.extend(
            profile
                .filter_lists
                .iter()
                .map(|entry| CatalogRevision::Filter(entry.revision)),
        );
        references.extend(
            profile
                .service_rules
                .iter()
                .map(|entry| CatalogRevision::Service(entry.revision)),
        );
    }
    for endpoint in &document.endpoints {
        sqlx::query("INSERT INTO endpoint_assignments (revision, id, profile, enabled, device) VALUES (?, ?, ?, ?, ?)")
            .bind(revision).bind(endpoint.id.as_bytes().as_slice()).bind(endpoint.profile.as_bytes().as_slice())
            .bind(endpoint.enabled).bind(endpoint.device.as_ref().map(|id| id.as_bytes().as_slice()))
            .execute(&mut *db).await.map_err(database)?;
    }
    for credential in &document.resolver_credentials {
        let state = match credential.state {
            CredentialState::Active => "active",
            CredentialState::Disabled => "disabled",
            CredentialState::Expired => "expired",
            CredentialState::Revoked => "revoked",
        };
        sqlx::query("INSERT INTO credential_metadata (revision, id, endpoint, state, verifier_ref) VALUES (?, ?, ?, ?, ?)")
            .bind(revision).bind(credential.id.as_bytes().as_slice()).bind(credential.endpoint.as_bytes().as_slice())
            .bind(state).bind(links.get(&credential.id).map(|id| id.as_bytes().as_slice()))
            .execute(&mut *db).await.map_err(database)?;
    }
    for listener in &document.listeners {
        sqlx::query("INSERT INTO configuration_listeners (revision, id) VALUES (?, ?)")
            .bind(revision)
            .bind(listener.id.as_bytes().as_slice())
            .execute(&mut *db)
            .await
            .map_err(database)?;
    }
    for group in &document.network_groups {
        sqlx::query("INSERT INTO configuration_network_groups (revision, id, traffic_profile) VALUES (?, ?, ?)")
            .bind(revision).bind(group.id.as_bytes().as_slice()).bind(group.traffic_profile.as_bytes().as_slice())
            .execute(&mut *db).await.map_err(database)?;
    }
    for grant in &document.legacy_grants {
        sqlx::query("INSERT INTO legacy_dns_grants (revision, id, source, listener, network_group, endpoint) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(revision).bind(grant.id.as_bytes().as_slice()).bind(grant.source.to_string())
            .bind(grant.listener.as_bytes().as_slice()).bind(grant.group.as_bytes().as_slice()).bind(grant.endpoint.as_bytes().as_slice())
            .execute(&mut *db).await.map_err(database)?;
    }
    for grant in &document.gateway_grants {
        sqlx::query("INSERT INTO gateway_network_grants (revision, id, source, listener, network_group) VALUES (?, ?, ?, ?, ?)")
            .bind(revision).bind(grant.id.as_bytes().as_slice()).bind(grant.source.to_string())
            .bind(grant.listener.as_bytes().as_slice()).bind(grant.group.as_bytes().as_slice())
            .execute(&mut *db).await.map_err(database)?;
        references.extend(grant.services.iter().copied().map(CatalogRevision::Service));
    }
    for reference in references {
        let (kind, catalog_id) = reference.parts();
        sqlx::query(
            "INSERT INTO configuration_catalog_refs (revision, kind, catalog_id) VALUES (?, ?, ?)",
        )
        .bind(revision)
        .bind(kind)
        .bind(catalog_id.as_slice())
        .execute(&mut *db)
        .await
        .map_err(database)?;
    }
    transaction.commit().await.map_err(database)
}

pub(crate) async fn load(
    connection: &mut SqliteConnection,
    id: ConfigurationRevisionId,
) -> Result<Option<StoredConfiguration>, StorageError> {
    let mut transaction = connection.begin().await.map_err(database)?;
    let row = sqlx::query("SELECT format, CASE WHEN length(document) BETWEEN 1 AND 1048576 THEN document END AS document FROM configuration_revisions WHERE id=?")
        .bind(id.as_bytes().as_slice()).fetch_optional(&mut *transaction).await.map_err(database)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let format = match row.try_get::<i64, _>("format").map_err(database)? {
        0 => InputFormat::Json,
        1 => InputFormat::Yaml,
        2 => InputFormat::Toml,
        _ => return Err(StorageError::new(StorageErrorCode::CorruptData)),
    };
    let input: Option<Vec<u8>> = row.try_get("document").map_err(database)?;
    let input = input.ok_or_else(|| StorageError::new(StorageErrorCode::CorruptData))?;
    let rows = sqlx::query("SELECT CASE WHEN length(id)=16 THEN id END AS id, CASE WHEN length(verifier_ref)=16 THEN verifier_ref END AS verifier_ref FROM credential_metadata WHERE revision=? AND verifier_ref IS NOT NULL ORDER BY id LIMIT 10001")
        .bind(id.as_bytes().as_slice()).fetch_all(&mut *transaction).await.map_err(database)?;
    if rows.len() > MAX_RESOURCE_ENTRIES {
        return Err(StorageError::new(StorageErrorCode::CorruptData));
    }
    let mut links = Vec::with_capacity(rows.len());
    for row in rows {
        let credential = ResolverCredentialId::from_bytes(id_bytes(&row, "id")?)
            .map_err(|_| StorageError::new(StorageErrorCode::CorruptData))?;
        let reference = SecretRefId::from_bytes(id_bytes(&row, "verifier_ref")?)
            .map_err(|_| StorageError::new(StorageErrorCode::CorruptData))?;
        links.push(CredentialVerifierLink {
            credential,
            reference,
        });
    }
    transaction.commit().await.map_err(database)?;
    Ok(Some(StoredConfiguration {
        format,
        input,
        links,
    }))
}

fn id_bytes(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<[u8; 16], StorageError> {
    let value: Option<Vec<u8>> = row.try_get(column).map_err(database)?;
    value
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| StorageError::new(StorageErrorCode::CorruptData))
}

pub(crate) async fn register_catalog(
    connection: &mut SqliteConnection,
    revision: CatalogRevision,
    sha256: [u8; 32],
) -> Result<(), StorageError> {
    let (kind, id) = revision.parts();
    sqlx::query("INSERT INTO catalog_revisions (kind, id, sha256) VALUES (?, ?, ?)")
        .bind(kind)
        .bind(id.as_slice())
        .bind(sha256.as_slice())
        .execute(connection)
        .await
        .map_err(database)?;
    Ok(())
}

pub(crate) async fn catalog_digest(
    connection: &mut SqliteConnection,
    revision: CatalogRevision,
) -> Result<Option<[u8; 32]>, StorageError> {
    let (kind, id) = revision.parts();
    let digest: Option<Option<Vec<u8>>> = sqlx::query_scalar("SELECT CASE WHEN length(sha256)=32 THEN sha256 END FROM catalog_revisions WHERE kind=? AND id=?")
        .bind(kind).bind(id.as_slice()).fetch_optional(connection).await.map_err(database)?;
    match digest {
        None => Ok(None),
        Some(Some(bytes)) => bytes
            .try_into()
            .map(Some)
            .map_err(|_| StorageError::new(StorageErrorCode::CorruptData)),
        Some(None) => Err(StorageError::new(StorageErrorCode::CorruptData)),
    }
}
