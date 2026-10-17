//! Isolated Ed25519 qualification. Public RFC fixtures only; no snapshot codec.
use aws_lc_rs::signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey};

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

fn verify(public: &[u8], message: &[u8], signature: &[u8]) -> bool {
    UnparsedPublicKey::new(&ED25519, public)
        .verify(message, signature)
        .is_ok()
}

#[test]
fn all_five_rfc8032_pure_ed25519_vectors_sign_and_verify() {
    assert_eq!(VECTORS.len(), 5);
    let lengths: Vec<_> = VECTORS.iter().map(|v| hex(v.message).len()).collect();
    assert_eq!(lengths, [0, 1, 2, 1023, 64]);
    for (index, vector) in VECTORS.iter().enumerate() {
        let public = hex(vector.public);
        let pair = Ed25519KeyPair::from_seed_and_public_key(&hex(vector.seed), &public).unwrap();
        assert_eq!(pair.public_key().as_ref(), public);
        let message = hex(vector.message);
        let expected = hex(vector.signature);
        assert_eq!(pair.try_sign(&message).unwrap().as_ref(), expected);
        assert!(verify(&public, &message, &expected), "RFC vector {index}");
    }
}

#[test]
fn message_public_key_and_every_signature_byte_are_authenticated() {
    for vector in VECTORS {
        let public = hex(vector.public);
        let message = hex(vector.message);
        let signature = hex(vector.signature);
        assert!(verify(&public, &message, &signature));
        for index in 0..signature.len() {
            let mut altered = signature.clone();
            altered[index] ^= 1;
            assert!(!verify(&public, &message, &altered));
        }
        for index in 0..public.len() {
            let mut altered = public.clone();
            altered[index] ^= 1;
            assert!(!verify(&altered, &message, &signature));
        }
        let mut extended = message.clone();
        extended.push(0);
        assert!(!verify(&public, &extended, &signature));
        for index in 0..message.len() {
            let mut altered = message.clone();
            altered[index] ^= 1;
            assert!(!verify(&public, &altered, &signature));
        }
        if !message.is_empty() {
            assert!(!verify(&public, &message[..message.len() - 1], &signature));
        }
    }
}

#[test]
fn malformed_lengths_wrong_trusted_key_and_mismatched_keypair_fail() {
    let vector = &VECTORS[0];
    let public = hex(vector.public);
    let seed = hex(vector.seed);
    let signature = hex(vector.signature);
    for length in [0, 1, 31, 33, 64] {
        assert!(Ed25519KeyPair::from_seed_and_public_key(&vec![7; length], &public).is_err());
        assert!(Ed25519KeyPair::from_seed_and_public_key(&seed, &vec![7; length]).is_err());
        assert!(!verify(&vec![7; length], b"", &signature));
    }
    for length in [0, 1, 32, 63, 65, 128] {
        assert!(!verify(&public, b"", &vec![7; length]));
    }
    let other_public = hex(VECTORS[1].public);
    assert!(!verify(&other_public, b"", &signature));
    assert!(Ed25519KeyPair::from_seed_and_public_key(&seed, &other_public).is_err());
    assert!(!verify(&[0; 32], b"", &signature));
    assert!(!verify(&[255; 32], b"", &signature));
}

#[test]
fn noncanonical_scalar_and_point_encodings_are_rejected() {
    let vector = &VECTORS[0];
    let public = hex(vector.public);
    let mut signature = hex(vector.signature);
    // RFC8032 section5.1.7 requires S < L. L encoded little-endian is public data.
    let order = hex("edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010");
    signature[32..].copy_from_slice(&order);
    assert!(!verify(&public, b"", &signature));
    signature[32..].fill(255);
    assert!(!verify(&public, b"", &signature));
    signature = hex(vector.signature);
    // The encoded y coordinate p is out of range; no curve implementation here.
    let point = hex("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f");
    signature[..32].copy_from_slice(&point);
    assert!(!verify(&public, b"", &signature));
    assert!(!verify(&point, b"", &hex(vector.signature)));
}

#[test]
fn library_pkcs8_roundtrip_preserves_identity_and_rejects_truncation() {
    let vector = &VECTORS[2];
    let public = hex(vector.public);
    let pair = Ed25519KeyPair::from_seed_and_public_key(&hex(vector.seed), &public).unwrap();
    let document = pair.to_pkcs8().unwrap();
    let imported = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    assert_eq!(imported.public_key().as_ref(), public);
    assert_eq!(
        imported.try_sign(&hex(vector.message)).unwrap().as_ref(),
        hex(vector.signature)
    );
    for length in 0..document.as_ref().len() {
        assert!(Ed25519KeyPair::from_pkcs8(&document.as_ref()[..length]).is_err());
    }
}
