//! Owner-approved strict verification qualification; intentionally public fixtures.
use aws_lc_rs::signature::{ED25519, Ed25519KeyPair, UnparsedPublicKey};
use ed25519_dalek::{Signature, VerifyingKey};

struct Vector {
    seed: &'static str,
    public: &'static str,
    message: &'static str,
    signature: &'static str,
}
include!("rfc8032_vectors.rs");

fn hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn trusted_key(public: &[u8]) -> Option<VerifyingKey> {
    let public = public.try_into().ok()?;
    let key = VerifyingKey::from_bytes(public).ok()?;
    (!key.is_weak() && key.to_edwards().compress().as_bytes() == public).then_some(key)
}

fn strict(public: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let Some(key) = trusted_key(public) else {
        return false;
    };
    let Ok(signature) = Signature::try_from(signature) else {
        return false;
    };
    key.verify_strict(message, &signature).is_ok()
}

#[test]
fn all_five_rfc_vectors_and_aws_lc_signatures_verify_strictly() {
    assert_eq!(VECTORS.len(), 5);
    for (index, vector) in VECTORS.iter().enumerate() {
        let public = hex(vector.public);
        let message = hex(vector.message);
        let signature = hex(vector.signature);
        let key = trusted_key(&public).unwrap();
        assert_eq!(key.to_bytes().as_slice(), public);
        assert!(strict(&public, &message, &signature), "RFC vector {index}");
        let pair = Ed25519KeyPair::from_seed_and_public_key(&hex(vector.seed), &public).unwrap();
        assert_eq!(pair.try_sign(&message).unwrap().as_ref(), signature);
        let mut domain_message = b"DRouteX snapshot v1\0".to_vec();
        domain_message.extend_from_slice(&message);
        let signed = pair.try_sign(&domain_message).unwrap();
        assert!(strict(&public, &domain_message, signed.as_ref()));
        assert!(!strict(&public, &message, signed.as_ref()));
    }
}

#[test]
fn every_signature_key_and_message_byte_remains_authenticated() {
    for vector in VECTORS {
        let public = hex(vector.public);
        let message = hex(vector.message);
        let signature = hex(vector.signature);
        for index in 0..signature.len() {
            let mut altered = signature.clone();
            altered[index] ^= 1;
            assert!(!strict(&public, &message, &altered), "signature {index}");
        }
        for index in 0..public.len() {
            let mut altered = public.clone();
            altered[index] ^= 1;
            assert!(!strict(&altered, &message, &signature), "key {index}");
        }
        for index in 0..message.len() {
            let mut altered = message.clone();
            altered[index] ^= 1;
            assert!(!strict(&public, &altered, &signature), "message {index}");
        }
        let mut extended = message.clone();
        extended.push(0);
        assert!(!strict(&public, &extended, &signature));
        if !message.is_empty() {
            assert!(!strict(&public, &message[..message.len() - 1], &signature));
        }
    }
}

#[test]
fn weak_keys_and_their_noncanonical_aliases_are_refused_at_import() {
    // Public Edwards encodings: y=0, identity, order-two y=p-1 (upstream
    // ed25519-dalek tests EIGHT_TORSION_4), and ZIP-215 noncanonical aliases.
    let encodings = [
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000080",
        "0100000000000000000000000000000000000000000000000000000000000000",
        "0100000000000000000000000000000000000000000000000000000000000080",
        "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
        "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
        "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    ];
    for encoded in encodings {
        assert!(trusted_key(&hex(encoded)).is_none(), "weak key {encoded}");
    }
}

#[test]
fn noncanonical_nonweak_key_is_refused_instead_of_silently_normalized() {
    // y = p + 3 encodes the same point as canonical y = 3 under ZIP-215.
    let alias: [u8; 32] = hex("f0ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f")
        .try_into()
        .unwrap();
    let parsed = VerifyingKey::from_bytes(&alias).unwrap();
    assert!(!parsed.is_weak());
    let canonical = parsed.to_edwards().compress().to_bytes();
    assert_ne!(canonical, alias);
    assert!(trusted_key(&canonical).is_some());
    assert!(trusted_key(&alias).is_none());
}

#[test]
fn identity_forgery_is_rejected_for_every_message_without_a_private_key() {
    let mut identity = [0; 32];
    identity[0] = 1;
    let mut forged = [0; 64];
    forged[0] = 1;
    let parsed = VerifyingKey::from_bytes(&identity).unwrap();
    assert!(parsed.is_weak());
    for message in [b"".as_slice(), b"DRouteX snapshot v1\0candidate", b"other"] {
        // Differential reproduction of the original boundary defect.
        assert!(
            UnparsedPublicKey::new(&ED25519, identity)
                .verify(message, &forged)
                .is_ok()
        );
        assert!(
            parsed
                .verify_strict(message, &Signature::from_bytes(&forged))
                .is_err()
        );
        assert!(!strict(&identity, message, &forged));
    }
}

#[test]
fn malformed_lengths_noncanonical_scalars_and_weak_r_are_refused() {
    let vector = &VECTORS[0];
    let public = hex(vector.public);
    let signature = hex(vector.signature);
    for length in [0, 1, 31, 33, 64] {
        assert!(trusted_key(&vec![7; length]).is_none());
    }
    for length in [0, 1, 32, 63, 65, 128] {
        assert!(!strict(&public, b"", &vec![7; length]));
    }
    let mut altered = signature.clone();
    altered[32..].copy_from_slice(&hex(
        "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010",
    ));
    assert!(!strict(&public, b"", &altered));
    altered[32..].fill(255);
    assert!(!strict(&public, b"", &altered));
    for r in [
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0100000000000000000000000000000000000000000000000000000000000000",
        "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    ] {
        altered = signature.clone();
        altered[..32].copy_from_slice(&hex(r));
        assert!(!strict(&public, b"", &altered));
    }
    assert!(!strict(&hex(VECTORS[1].public), b"", &signature));
}
