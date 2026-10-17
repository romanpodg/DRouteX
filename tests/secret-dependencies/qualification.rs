//! Isolated primitive qualification; no Routira storage or secret API is adopted here.
use aws_lc_rs::{
    aead::{AES_256_GCM_SIV, Aad, LessSafeKey, Nonce, UnboundKey},
    rand::fill,
};
use zeroize::{Zeroize, Zeroizing};

struct Vector {
    key: &'static str,
    nonce: &'static str,
    aad: &'static str,
    plaintext: &'static str,
    result: &'static str,
}
include!("rfc8452_vectors.rs");

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

fn key(bytes: &[u8]) -> LessSafeKey {
    LessSafeKey::new(UnboundKey::new(&AES_256_GCM_SIV, bytes).unwrap())
}

fn nonce(bytes: &[u8]) -> Nonce {
    Nonce::try_assume_unique_for_key(bytes).unwrap()
}

#[test]
fn official_rfc8452_aes256_vectors_encrypt_and_decrypt() {
    assert_eq!(VECTORS.len(), 24);
    for (index, vector) in VECTORS.iter().enumerate() {
        let key = key(&hex(vector.key));
        let nonce_bytes = hex(vector.nonce);
        let aad = hex(vector.aad);
        let plaintext = hex(vector.plaintext);
        let mut buffer = plaintext.clone();
        key.seal_in_place_append_tag(nonce(&nonce_bytes), Aad::from(&aad), &mut buffer)
            .unwrap();
        assert_eq!(buffer, hex(vector.result), "RFC vector {index}");
        let opened = key
            .open_in_place(nonce(&nonce_bytes), Aad::from(&aad), &mut buffer)
            .unwrap();
        assert_eq!(opened, plaintext, "RFC vector {index}");
    }
}

fn sealed_fixture() -> (LessSafeKey, [u8; 12], Vec<u8>, Vec<u8>) {
    let key = key(&[7; 32]);
    let nonce = [8; 12];
    // Synthetic test data only. A production envelope still requires its own design gate.
    let aad = b"routira-test:v1:secret-id:outbound-auth".to_vec();
    let mut buffer = b"synthetic-secret-canary".to_vec();
    key.seal_in_place_append_tag(self::nonce(&nonce), Aad::from(&aad), &mut buffer)
        .unwrap();
    (key, nonce, aad, buffer)
}

#[test]
fn every_ciphertext_and_tag_byte_is_authenticated() {
    let (key, nonce, aad, ciphertext) = sealed_fixture();
    for index in 0..ciphertext.len() {
        let mut altered = ciphertext.clone();
        altered[index] ^= 1;
        assert!(
            key.open_in_place(self::nonce(&nonce), Aad::from(&aad), &mut altered)
                .is_err()
        );
    }
}

#[test]
fn wrong_key_nonce_and_record_context_are_rejected() {
    let (key, nonce, aad, ciphertext) = sealed_fixture();
    let wrong_key = self::key(&[9; 32]);
    assert!(
        wrong_key
            .open_in_place(
                self::nonce(&nonce),
                Aad::from(&aad),
                &mut ciphertext.clone()
            )
            .is_err()
    );
    for index in 0..nonce.len() {
        let mut changed = nonce;
        changed[index] ^= 1;
        assert!(
            key.open_in_place(
                self::nonce(&changed),
                Aad::from(&aad),
                &mut ciphertext.clone()
            )
            .is_err()
        );
    }
    for index in 0..aad.len() {
        let mut changed = aad.clone();
        changed[index] ^= 1;
        assert!(
            key.open_in_place(
                self::nonce(&nonce),
                Aad::from(&changed),
                &mut ciphertext.clone()
            )
            .is_err()
        );
    }
}

#[test]
fn missing_and_truncated_ciphertext_or_tag_is_rejected() {
    let (key, nonce, aad, ciphertext) = sealed_fixture();
    for length in 0..ciphertext.len() {
        let mut truncated = ciphertext[..length].to_vec();
        assert!(
            key.open_in_place(self::nonce(&nonce), Aad::from(&aad), &mut truncated)
                .is_err()
        );
    }
}

#[test]
fn algorithm_refuses_wrong_key_and_nonce_lengths() {
    for length in [0, 1, 16, 31, 33, 64] {
        assert!(UnboundKey::new(&AES_256_GCM_SIV, &vec![7; length]).is_err());
    }
    for length in [0, 1, 11, 13, 24] {
        assert!(Nonce::try_assume_unique_for_key(&vec![8; length]).is_err());
    }
}

#[test]
fn system_random_smoke_and_explicit_zeroization() {
    // A smoke test of the OS-backed API, not a statistical proof of entropy.
    let mut material = Zeroizing::new([0_u8; 32]);
    fill(material.as_mut()).unwrap();
    assert_ne!(*material, [0; 32]);
    let mut first = [0_u8; 12];
    let mut second = [0_u8; 12];
    fill(&mut first).unwrap();
    fill(&mut second).unwrap();
    assert_ne!(first, second);
    material.zeroize();
    assert_eq!(*material, [0; 32]);
    // No claim about copies in registers, allocator pages, dumps or OS buffers.
}
