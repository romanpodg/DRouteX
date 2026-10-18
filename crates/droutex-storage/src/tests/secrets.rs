use super::*;
use droutex_security::{Keyring, SecretBytes, SecretContext, SecretPurpose};

const CANARY: &[u8] = b"synthetic-storage-secret-canary-2026";

pub(super) fn keys(active: u8, retained: &[(u8, u8)]) -> Keyring {
    let mut bytes = b"RTKEY\0\x00\x01".to_vec();
    bytes.push(retained.len() as u8);
    bytes.extend_from_slice(&[active; 16]);
    for &(id, key) in retained {
        bytes.extend_from_slice(&[id; 16]);
        bytes.extend_from_slice(&[key; 32]);
    }
    Keyring::from_owned_bytes(bytes).unwrap()
}
pub(super) fn context() -> SecretContext {
    SecretContext {
        reference: typed(99),
        purpose: SecretPurpose::ResolverVerifier,
    }
}
pub(super) fn record(keys: &Keyring) -> EncryptedSecret {
    keys.seal(
        context(),
        SecretBytes::from_owned_bytes(CANARY.to_vec()).unwrap(),
    )
    .unwrap()
}

#[test]
fn encrypted_records_are_immutable_and_reencryption_requires_exact_old_envelope() {
    run(async {
        let storage = memory_storage().await;
        let repository = storage.repository();
        let old = keys(1, &[(1, 7)]);
        let rotated = keys(2, &[(1, 7), (2, 9)]);
        let original = record(&old);
        repository.insert_secret(original.clone()).await.unwrap();
        assert_eq!(
            repository
                .insert_secret(original.clone())
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::Conflict
        );
        let saved = repository.load_secret(typed(99)).await.unwrap().unwrap();
        assert_eq!(old.open(context(), &saved).unwrap().expose_secret(), CANARY);
        let changed = rotated.reencrypt(context(), &saved).unwrap();
        repository
            .replace_secret(saved.clone(), changed.clone())
            .await
            .unwrap();
        assert_eq!(
            repository
                .replace_secret(saved, changed.clone())
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::Conflict
        );
        let mut other_context = context();
        other_context.purpose = SecretPurpose::ProxyOutboundCredential;
        let other = rotated
            .seal(
                other_context,
                SecretBytes::from_owned_bytes(CANARY.to_vec()).unwrap(),
            )
            .unwrap();
        assert_eq!(
            repository
                .replace_secret(changed.clone(), other)
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::InvalidSecret
        );
        assert_eq!(
            repository.load_secret(typed(99)).await.unwrap().unwrap(),
            changed
        );
        assert!(repository.load_secret(typed(100)).await.unwrap().is_none());
        storage.close().await.unwrap();
    });
}

#[test]
fn expired_and_abandoned_secret_commands_do_not_persist() {
    run(async {
        let storage = memory_storage().await;
        let repository = storage.repository();
        let keys = keys(1, &[(1, 7)]);
        let (reply, response) = oneshot::channel();
        repository
            .sender
            .try_send(Command {
                deadline: Instant::now() - Duration::from_secs(1),
                operation: Operation::InsertSecret {
                    record: record(&keys),
                    reply,
                },
            })
            .unwrap_or_else(|_| panic!("queue rejected fixture"));
        assert_eq!(
            response.await.unwrap().unwrap_err().code(),
            StorageErrorCode::DeadlineOutcomeUnknown
        );
        let (reply, response) = oneshot::channel();
        drop(response);
        repository
            .sender
            .try_send(Command {
                deadline: Instant::now() + RESPONSE_TIMEOUT,
                operation: Operation::InsertSecret {
                    record: record(&keys),
                    reply,
                },
            })
            .unwrap_or_else(|_| panic!("queue rejected fixture"));
        assert!(repository.load_secret(typed(99)).await.unwrap().is_none());
        storage.close().await.unwrap();
    });
}
