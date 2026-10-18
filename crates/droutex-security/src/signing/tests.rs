use super::*;

fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn fixture() -> (Vec<u8>, [u8; 32], SigningKeyId) {
    // Intentionally public RFC8032 test1 seed encoded as PKCS#8 v1.
    let der = hex(
        "302e020100300506032b6570042204209d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
    );
    let public = hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
        .try_into()
        .unwrap();
    (der, public, SigningKeyId::from_bytes([1; 16]).unwrap())
}

#[test]
fn signing_binds_domain_and_trusted_key_without_private_key_debug() {
    let (der, public, id) = fixture();
    let pair = Ed25519KeyPair::from_pkcs8(&der).unwrap();
    let key = SigningKey::from_pkcs8(id, der, &public).unwrap();
    let trust = SnapshotTrust::new(vec![(id, public)]).unwrap();
    let signature = key.sign_snapshot(b"candidate").unwrap();
    trust.verify_snapshot(id, b"candidate", &signature).unwrap();
    assert!(trust.verify_snapshot(id, b"other", &signature).is_err());
    let raw: [u8; 64] = pair
        .try_sign(b"candidate")
        .unwrap()
        .as_ref()
        .try_into()
        .unwrap();
    assert!(trust.verify_snapshot(id, b"candidate", &raw).is_err());
    assert_eq!(format!("{key:?}"), "SigningKey([REDACTED])");
    assert!(key.sign_snapshot(&vec![0; MAX_SIGNED_BYTES + 1]).is_err());
    assert_eq!(
        sha256(b"abc").as_slice(),
        hex("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
}

#[test]
fn malformed_key_wrong_identity_and_invalid_trust_are_refused() {
    let (der, public, id) = fixture();
    assert!(SigningKeyId::from_bytes([0; 16]).is_err());
    assert!(SigningKey::from_pkcs8(id, der.clone(), &[0; 32]).is_err());
    for length in 0..der.len() {
        let error = SigningKey::from_pkcs8(id, der[..length].to_vec(), &public).unwrap_err();
        assert_eq!(error.code(), SigningErrorCode::Key);
        assert_eq!(error.to_string(), "snapshot signing Key");
    }
    assert!(SigningKey::from_pkcs8(id, vec![0; MAX_PKCS8_BYTES + 1], &public).is_err());
    assert!(SnapshotTrust::new(vec![]).is_err());
    assert!(SnapshotTrust::new(vec![(id, public); 2]).is_err());
    assert!(SnapshotTrust::new(vec![(id, public); 33]).is_err());
    let other = SigningKeyId::from_bytes([2; 16]).unwrap();
    assert!(
        SnapshotTrust::new(vec![(other, public)])
            .unwrap()
            .verify_snapshot(id, b"x", &[0; 64])
            .is_err()
    );
    assert_eq!(
        SnapshotTrust::new(vec![(id, [0; 32])]).unwrap_err().code(),
        SigningErrorCode::Trust
    );
}

#[test]
fn weak_public_keys_are_refused_when_trust_is_constructed() {
    let id = SigningKeyId::from_bytes([1; 16]).unwrap();
    // Public small-order encodings: all-zero and the Edwards identity (y = 1).
    // Refusal belongs at import, independently of any message-dependent forgery.
    let mut identity = [0; 32];
    identity[0] = 1;
    for (name, public) in [("all-zero", [0; 32]), ("identity", identity)] {
        let error = SnapshotTrust::new(vec![(id, public)]).expect_err(name);
        assert_eq!(error.code(), SigningErrorCode::Trust, "{name}");
    }
}

#[test]
fn weak_identity_key_cannot_authorize_arbitrary_snapshot_bytes() {
    let id = SigningKeyId::from_bytes([1; 16]).unwrap();
    // Public identity point A = R = identity, S = 0: no signing key is needed.
    let mut public = [0; 32];
    public[0] = 1;
    let mut signature = [0; 64];
    signature[0] = 1;
    for bytes in [b"".as_slice(), b"candidate", b"different snapshot"] {
        let verification = SnapshotTrust::new(vec![(id, public)])
            .and_then(|trust| trust.verify_snapshot(id, bytes, &signature));
        assert!(verification.is_err(), "identity-key forgery was accepted");
    }
}

#[test]
fn noncanonical_public_key_is_refused_at_the_trust_boundary() {
    let id = SigningKeyId::from_bytes([1; 16]).unwrap();
    let alias = hex("f0ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f")
        .try_into()
        .unwrap();
    assert_eq!(
        SnapshotTrust::new(vec![(id, alias)]).unwrap_err().code(),
        SigningErrorCode::Trust
    );
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
#[test]
fn unsupported_signing_file_provider_fails_closed() {
    let (_, public, id) = fixture();
    assert_eq!(
        SigningKey::load_file(
            Path::new("key"),
            Path::new("db"),
            Path::new("backup"),
            id,
            &public
        )
        .unwrap_err()
        .code(),
        SigningErrorCode::ProtectedFile(crate::SecretErrorCode::UnsupportedPlatform)
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn signing_file_uses_private_modes_and_its_own_der_bound() {
    use std::{
        fs,
        os::unix::fs::{DirBuilderExt, PermissionsExt, symlink},
        time::{SystemTime, UNIX_EPOCH},
    };
    let (der, public, id) = fixture();
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/signing-key-tests");
    fs::create_dir_all(&base).unwrap();
    let root = fs::canonicalize(base).unwrap().join(format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    for name in ["keys", "db", "backup"] {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(root.join(name))
            .unwrap();
    }
    let path = root.join("keys/key");
    fs::write(&path, &der).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let load = |path: &Path| {
        SigningKey::load_file(path, &root.join("db"), &root.join("backup"), id, &public)
    };
    assert!(load(&path).is_ok());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(load(&path).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    symlink(&path, root.join("keys/link")).unwrap();
    assert!(load(&root.join("keys/link")).is_err());
    fs::hard_link(&path, root.join("keys/hard")).unwrap();
    assert!(load(&path).is_err());
    fs::remove_file(root.join("keys/hard")).unwrap();
    fs::write(&path, vec![0; MAX_PKCS8_BYTES + 1]).unwrap();
    assert!(load(&path).is_err());
    fs::write(&path, &der).unwrap();
    for directory in ["db", "backup"] {
        let target = root.join(directory).join("key");
        fs::copy(&path, &target).unwrap();
        assert!(load(&target).is_err());
    }
    fs::remove_dir_all(root).unwrap();
}
