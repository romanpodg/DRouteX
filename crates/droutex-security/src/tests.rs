use super::*;

const CANARY: &[u8] = b"synthetic-P1-07-secret-never-log";

fn context() -> SecretContext {
    SecretContext {
        reference: "10000000-0000-4000-8000-000000000001".parse().unwrap(),
        purpose: SecretPurpose::ResolverVerifier,
    }
}

fn key_bytes(active: u8, entries: &[(u8, u8)]) -> Vec<u8> {
    let mut bytes = b"RTKEY\0\x00\x01".to_vec();
    bytes.push(entries.len() as u8);
    bytes.extend_from_slice(&[active; 16]);
    for &(id, value) in entries {
        bytes.extend_from_slice(&[id; 16]);
        bytes.extend_from_slice(&[value; 32]);
    }
    bytes
}

fn ring(active: u8, entries: &[(u8, u8)]) -> Keyring {
    Keyring::from_owned_bytes(key_bytes(active, entries)).unwrap()
}

fn secret() -> SecretBytes {
    SecretBytes::from_owned_bytes(CANARY.to_vec()).unwrap()
}

#[test]
fn golden_header_context_roundtrip_and_redaction() {
    let keys = ring(1, &[(1, 7)]);
    let record = keys.seal_with_nonce(context(), secret(), [8; 12]).unwrap();
    let mut header = b"DXSEC\0\x00\x01\x01".to_vec();
    header.extend_from_slice(&[1; 16]);
    header.extend_from_slice(&[0x10, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]);
    header.push(1);
    header.extend_from_slice(&[8; 12]);
    header.extend_from_slice(&[0, 0, 0, 32]);
    assert_eq!(record.header(), header);
    assert_eq!(
        keys.open(context(), &record).unwrap().expose_secret(),
        CANARY
    );
    assert!(!record.as_bytes().windows(CANARY.len()).any(|w| w == CANARY));
    assert_eq!(format!("{:?}", secret()), "SecretBytes([REDACTED])");
    assert_eq!(format!("{keys:?}"), "Keyring([REDACTED])");
    for text in [
        format!("{record:?}"),
        format!("{}", SecretError(SecretErrorCode::Authentication)),
    ] {
        assert!(!text.contains(std::str::from_utf8(CANARY).unwrap()));
    }
}

#[test]
fn every_envelope_byte_and_truncation_is_rejected_or_fails_authentication() {
    let keys = ring(1, &[(1, 7)]);
    let record = keys.seal_with_nonce(context(), secret(), [8; 12]).unwrap();
    for index in 0..record.as_bytes().len() {
        let mut bytes = record.as_bytes().to_vec();
        bytes[index] ^= 1;
        if let Ok(changed) = EncryptedSecret::from_owned_bytes(bytes) {
            assert!(keys.open(context(), &changed).is_err(), "byte {index}");
        }
    }
    for length in 0..record.as_bytes().len() {
        assert!(
            EncryptedSecret::from_owned_bytes(record.as_bytes()[..length].to_vec()).is_err(),
            "length {length}"
        );
    }
    let mut bytes = record.as_bytes().to_vec();
    bytes.push(0);
    assert!(EncryptedSecret::from_owned_bytes(bytes).is_err());
}

#[test]
fn authentication_binds_context_and_key_id_even_when_caller_copies_changed_context() {
    let keys = ring(1, &[(1, 7), (2, 7)]);
    let record = keys.seal(context(), secret()).unwrap();
    for (range, value) in [(9..25, 2), (25..26, 0x20), (41..42, 2)] {
        let mut bytes = record.as_bytes().to_vec();
        bytes[range].fill(value);
        let changed = EncryptedSecret::from_owned_bytes(bytes).unwrap();
        assert_eq!(
            keys.open(changed.context(), &changed).unwrap_err().code(),
            SecretErrorCode::Authentication
        );
    }
    let mut other = context();
    other.purpose = SecretPurpose::ProxyOutboundCredential;
    assert_eq!(
        keys.open(other, &record).unwrap_err().code(),
        SecretErrorCode::ContextMismatch
    );
}

#[test]
fn rotation_retained_backup_wrong_and_missing_keys_are_explicit() {
    let old = ring(1, &[(1, 7)]);
    let backup = old.seal(context(), secret()).unwrap();
    let rotated = ring(2, &[(1, 7), (2, 9)]);
    let changed = rotated.reencrypt(context(), &backup).unwrap();
    assert_eq!(changed.key_id(), rotated.active_key_id());
    assert_eq!(
        rotated.open(context(), &backup).unwrap().expose_secret(),
        CANARY
    );
    assert_eq!(
        rotated.open(context(), &changed).unwrap().expose_secret(),
        CANARY
    );
    assert_eq!(
        old.open(context(), &changed).unwrap_err().code(),
        SecretErrorCode::MissingKey
    );
    assert_eq!(
        ring(1, &[(1, 8)])
            .open(context(), &backup)
            .unwrap_err()
            .code(),
        SecretErrorCode::Authentication
    );
    assert_eq!(
        ring(2, &[(2, 9)])
            .reencrypt(context(), &backup)
            .unwrap_err()
            .code(),
        SecretErrorCode::MissingKey
    );
}

#[test]
fn keyring_rejects_ambiguous_truncated_unknown_and_oversized_inputs() {
    let good = key_bytes(1, &[(1, 7)]);
    assert!(Keyring::from_owned_bytes(good.clone()).is_ok());
    for length in 0..good.len() {
        assert!(Keyring::from_owned_bytes(good[..length].to_vec()).is_err());
    }
    for bad in [
        key_bytes(1, &[(1, 7), (1, 8)]),
        key_bytes(1, &[(2, 7), (1, 8)]),
        key_bytes(2, &[(1, 7)]),
        key_bytes(0, &[(0, 7)]),
        key_bytes(1, &[]),
        key_bytes(1, &(1..=33).map(|id| (id, 7)).collect::<Vec<_>>()),
    ] {
        assert!(Keyring::from_owned_bytes(bad).is_err());
    }
    let mut extended = good.clone();
    extended.push(0);
    assert!(Keyring::from_owned_bytes(extended).is_err());
    let mut unknown = good;
    unknown[7] = 2;
    assert!(Keyring::from_owned_bytes(unknown).is_err());
    assert!(
        Keyring::from_owned_bytes(key_bytes(
            32,
            &(1..=32).map(|id| (id, 7)).collect::<Vec<_>>()
        ))
        .is_ok()
    );
}

#[test]
fn material_bounds_and_random_production_nonces() {
    assert!(SecretBytes::from_owned_bytes(vec![]).is_err());
    assert!(SecretBytes::from_owned_bytes(vec![0; MAX_SECRET_BYTES + 1]).is_err());
    let keys = ring(1, &[(1, 7)]);
    let record = keys
        .seal(
            context(),
            SecretBytes::from_owned_bytes(vec![7; MAX_SECRET_BYTES]).unwrap(),
        )
        .unwrap();
    assert_eq!(record.as_bytes().len(), MAX_ENVELOPE_BYTES);
    assert_eq!(
        keys.open(context(), &record).unwrap().expose_secret(),
        vec![7; MAX_SECRET_BYTES]
    );
    // OS API smoke check, not a statistical entropy proof.
    let first = keys.seal(context(), secret()).unwrap();
    let second = keys.seal(context(), secret()).unwrap();
    assert_ne!(first.nonce().unwrap(), second.nonce().unwrap());
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
#[test]
fn unsupported_file_provider_fails_closed() {
    use std::path::Path;
    assert_eq!(
        Keyring::load_file(Path::new("key"), Path::new("db"), Path::new("backup"))
            .unwrap_err()
            .code(),
        SecretErrorCode::UnsupportedPlatform
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "tests/linux.rs"]
mod linux;
