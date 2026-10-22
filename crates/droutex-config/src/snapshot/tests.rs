use super::*;
use crate::{InputFormat, ReferenceCatalog, parse_configuration};
use droutex_common::*;
use droutex_security::{
    Keyring, SecretBytes, SecretContext, SecretPurpose, SigningKey, SigningKeyId, SnapshotTrust,
};
use serde_json::{Value, json};
use std::str::FromStr;

fn typed<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn hex(value: &str) -> Vec<u8> {
    let text: String = value.split_whitespace().collect();
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn signer(second: bool) -> (SigningKey, [u8; 32]) {
    // Public RFC8032 test seeds only. PKCS#8 v1, no generated/operational key.
    let (seed, public) = if second {
        (
            "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
            "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        )
    } else {
        (
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        )
    };
    let der = hex(&format!("302e020100300506032b657004220420{seed}"));
    let public: [u8; 32] = hex(public).try_into().unwrap();
    (
        SigningKey::from_pkcs8(
            SigningKeyId::from_bytes([if second { 2 } else { 1 }; 16]).unwrap(),
            der,
            &public,
        )
        .unwrap(),
        public,
    )
}
fn metadata(sequence: u64) -> SnapshotMetadata {
    SnapshotMetadata {
        node: typed(1),
        snapshot: typed(2),
        source: typed(3),
        sequence: SnapshotSequence::new(sequence).unwrap(),
    }
}
fn empty_resources() -> TrustedResources {
    TrustedResources::new(vec![]).unwrap()
}
fn empty_content() -> SnapshotContent {
    SnapshotContent::new(
        parse_configuration(
            br#"{"schemaVersion":1}"#,
            InputFormat::Json,
            &ReferenceCatalog::default(),
        )
        .unwrap(),
        vec![],
        &empty_resources(),
    )
    .unwrap()
}

#[test]
fn leased_v2_three_format_fixture_has_one_canonical_signed_vector() {
    let formats = [
        (
            include_bytes!("../../tests/fixtures/configuration-v2.json").as_slice(),
            InputFormat::Json,
        ),
        (
            include_bytes!("../../tests/fixtures/configuration-v2.yaml").as_slice(),
            InputFormat::Yaml,
        ),
        (
            include_bytes!("../../tests/fixtures/configuration-v2.toml").as_slice(),
            InputFormat::Toml,
        ),
    ];
    let mut encoded = Vec::new();
    for (bytes, format) in formats {
        let checked = parse_configuration(bytes, format, &ReferenceCatalog::default()).unwrap();
        let content = SnapshotContent::new(checked, vec![], &empty_resources()).unwrap();
        encoded.push(encode_snapshot(&content, metadata(1), &signer(false).0).unwrap());
    }
    assert!(encoded.windows(2).all(|p| p[0] == p[1]));
    assert_eq!(
        encoded[0],
        hex(include_str!("../../tests/fixtures/snapshot-v2.hex"))
    );
}

#[test]
fn signed_domain_text_is_preserved_while_matching_uses_canonical_names() {
    let document = json!({"schemaVersion":1,"profiles":[{
        "id":typed::<ProfileId>(1).to_string(),"name":"p",
        "defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"deny"},
        "customRules":[{"id":typed::<RuleId>(2).to_string(),"enabled":true,"priority":0,
        "matcher":{"mode":"suffix","value":"BÜCHER.Example."},
        "action":{"plane":"dns","action":{"kind":"block"}}}]
    }]});
    let checked = parse_configuration(
        &serde_json::to_vec(&document).unwrap(),
        InputFormat::Json,
        &ReferenceCatalog::default(),
    )
    .unwrap();
    let content = SnapshotContent::new(checked, vec![], &empty_resources()).unwrap();
    let (key, _) = signer(false);
    let bytes = encode_snapshot(&content, metadata(1), &key).unwrap();
    let restored = verify(&bytes).unwrap();
    let matcher =
        &restored.content().configuration().document().profiles[0].custom_rules[0].matcher;
    assert_eq!(matcher.value, "BÜCHER.Example.");
    assert!(
        matcher
            .normalized()
            .unwrap()
            .matches(&"child.xn--bcher-kva.example".parse().unwrap())
    );
    assert_eq!(
        encode_snapshot(restored.content(), metadata(1), &key).unwrap(),
        bytes
    );
    assert_eq!(restored.content().as_bytes(), content.as_bytes());
}

#[test]
fn retained_verification_requires_exact_trusted_identity_without_resetting_floor() {
    let (key, public) = signer(false);
    let trust = SnapshotTrust::new(vec![(key.id(), public)]).unwrap();
    let bytes = encode_snapshot(&empty_content(), metadata(19), &key).unwrap();
    let expected = RetainedSnapshotIdentity {
        metadata: metadata(19),
        signing_key: key.id(),
        envelope_hash: ContentHash::of(&bytes),
    };
    let floor = ReplayState::from_trusted_high_water(typed(1), SnapshotSequence::new(19).unwrap());
    assert_eq!(
        verify_snapshot(&bytes, &trust, &floor, &empty_resources())
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Replay
    );
    assert_eq!(
        restore_snapshot(&bytes, &trust, expected, &empty_resources())
            .unwrap()
            .metadata(),
        metadata(19)
    );
    for wrong in [
        RetainedSnapshotIdentity {
            metadata: metadata(18),
            ..expected
        },
        RetainedSnapshotIdentity {
            metadata: SnapshotMetadata {
                snapshot: typed(9),
                ..metadata(19)
            },
            ..expected
        },
        RetainedSnapshotIdentity {
            metadata: SnapshotMetadata {
                source: typed(9),
                ..metadata(19)
            },
            ..expected
        },
        RetainedSnapshotIdentity {
            metadata: SnapshotMetadata {
                node: typed(9),
                ..metadata(19)
            },
            ..expected
        },
        RetainedSnapshotIdentity {
            signing_key: SigningKeyId::from_bytes([2; 16]).unwrap(),
            ..expected
        },
        RetainedSnapshotIdentity {
            envelope_hash: ContentHash::of(b"wrong"),
            ..expected
        },
    ] {
        assert!(restore_snapshot(&bytes, &trust, wrong, &empty_resources()).is_err());
    }
    let mut tampered = bytes.clone();
    tampered[140] ^= 1;
    let replaced_hash = RetainedSnapshotIdentity {
        envelope_hash: ContentHash::of(&tampered),
        ..expected
    };
    assert!(restore_snapshot(&tampered, &trust, replaced_hash, &empty_resources()).is_err());
    assert!(verify_snapshot(&bytes, &trust, &floor, &empty_resources()).is_err());
}
fn verify(bytes: &[u8]) -> Result<VerifiedSnapshot, SnapshotError> {
    let (key, public) = signer(false);
    verify_snapshot(
        bytes,
        &SnapshotTrust::new(vec![(key.id(), public)]).unwrap(),
        &ReplayState::first_provisioning(typed(1)),
        &empty_resources(),
    )
}
fn resign(payload: &[u8]) -> Vec<u8> {
    let (key, _) = signer(false);
    let mut bytes = encode_snapshot(&empty_content(), metadata(1), &key).unwrap()[..121].to_vec();
    bytes[85..89].copy_from_slice(&u32::try_from(payload.len()).unwrap().to_be_bytes());
    bytes[89..121].copy_from_slice(ContentHash::of(payload).as_bytes());
    bytes.extend_from_slice(payload);
    let signature = key.sign_snapshot(&bytes).unwrap();
    bytes.extend_from_slice(&signature);
    bytes
}

#[test]
fn independent_openssl_golden_and_all_three_formats_have_identical_bytes() {
    let golden = hex(include_str!("../../tests/fixtures/snapshot-v1.hex"));
    let (key, _) = signer(false);
    for (format, bytes) in [
        (InputFormat::Json, "{\"schemaVersion\":1}"),
        (InputFormat::Yaml, "schemaVersion: 1\n"),
        (InputFormat::Toml, "schemaVersion = 1\n"),
    ] {
        let checked =
            parse_configuration(bytes.as_bytes(), format, &ReferenceCatalog::default()).unwrap();
        let content = SnapshotContent::new(checked, vec![], &empty_resources()).unwrap();
        assert_eq!(
            content.hash().to_string(),
            "c4a15ead88f4c0f5ccc1888cb37c85e563495d14fa10bffb271da8d87e2fb385"
        );
        let encoded = encode_snapshot(&content, metadata(1), &key).unwrap();
        assert_eq!(encoded, golden);
        let candidate = verify(&encoded).unwrap();
        assert_eq!(candidate.metadata(), metadata(1));
        assert_eq!(candidate.signing_key(), key.id());
        assert_eq!(candidate.content().as_bytes(), content.as_bytes());
        assert!(candidate.content().credential_links().is_empty());
    }
}

#[test]
fn every_envelope_byte_truncation_and_trailing_data_is_refused() {
    let (key, _) = signer(false);
    let bytes = encode_snapshot(&empty_content(), metadata(1), &key).unwrap();
    for index in 0..bytes.len() {
        let mut altered = bytes.clone();
        altered[index] ^= 1;
        assert!(verify(&altered).is_err(), "byte {index}");
        assert!(verify(&bytes[..index]).is_err(), "length {index}");
    }
    let mut extended = bytes;
    extended.push(0);
    assert!(verify(&extended).is_err());
    assert_eq!(
        verify(&vec![0; MAX_SNAPSHOT_BYTES + 1]).unwrap_err().code(),
        SnapshotErrorCode::Size
    );
}

#[test]
fn replay_scope_gaps_rollback_rotation_and_revocation_keep_one_sequence_stream() {
    let content = empty_content();
    let (old, old_public) = signer(false);
    let (new, new_public) = signer(true);
    let trust = SnapshotTrust::new(vec![(old.id(), old_public), (new.id(), new_public)]).unwrap();
    let floor = ReplayState::from_trusted_high_water(typed(1), SnapshotSequence::new(5).unwrap());
    for sequence in [1, 4, 5] {
        let bytes = encode_snapshot(&content, metadata(sequence), &new).unwrap();
        assert_eq!(
            verify_snapshot(&bytes, &trust, &floor, &empty_resources())
                .unwrap_err()
                .code(),
            SnapshotErrorCode::Replay
        );
    }
    let republished = encode_snapshot(&content, metadata(9), &new).unwrap();
    assert_eq!(
        verify_snapshot(&republished, &trust, &floor, &empty_resources())
            .unwrap()
            .content()
            .hash(),
        content.hash()
    );
    let mut wrong = metadata(9);
    wrong.node = typed(4);
    assert_eq!(
        verify_snapshot(
            &encode_snapshot(&content, wrong, &old).unwrap(),
            &trust,
            &floor,
            &empty_resources()
        )
        .unwrap_err()
        .code(),
        SnapshotErrorCode::Scope
    );
    let old_bytes = encode_snapshot(&content, metadata(9), &old).unwrap();
    let revoked = SnapshotTrust::new(vec![(new.id(), new_public)]).unwrap();
    assert_eq!(
        verify_snapshot(&old_bytes, &revoked, &floor, &empty_resources())
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Authentication
    );
    assert!(SnapshotSequence::new(0).is_err());
    assert_eq!(
        SnapshotSequence::new(5)
            .unwrap()
            .checked_next()
            .unwrap()
            .get(),
        6
    );
    assert!(
        SnapshotSequence::new(u64::MAX)
            .unwrap()
            .checked_next()
            .is_err()
    );
    // A correctly signed zero sequence is also rejected at decode.
    let mut zero = encode_snapshot(&content, metadata(1), &old).unwrap();
    zero[77..85].fill(0);
    let end = zero.len() - 64;
    let signature = old.sign_snapshot(&zero[..end]).unwrap();
    zero[end..].copy_from_slice(&signature);
    assert_eq!(
        verify(&zero).unwrap_err().code(),
        SnapshotErrorCode::Sequence
    );
}

#[test]
fn even_authenticated_noncanonical_unknown_float_and_deep_payloads_fail() {
    let content = empty_content();
    let mut whitespace = content.as_bytes().to_vec();
    whitespace.push(b'\n');
    assert_eq!(
        verify(&resign(&whitespace)).unwrap_err().code(),
        SnapshotErrorCode::Canonical
    );
    let mut value: Value = serde_json::from_slice(content.as_bytes()).unwrap();
    value["unexpected"] = json!(true);
    assert_eq!(
        verify(&resign(&serde_json::to_vec(&value).unwrap()))
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Format
    );
    value.as_object_mut().unwrap().remove("unexpected");
    value["configuration"]
        .as_object_mut()
        .unwrap()
        .remove("profiles");
    assert_eq!(
        verify(&resign(&serde_json::to_vec(&value).unwrap()))
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Canonical
    );
    value["configuration"]["schemaVersion"] = json!(1.0);
    assert_eq!(
        verify(&resign(&serde_json::to_vec(&value).unwrap()))
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Format
    );
    let duplicate = format!(
        "{{\"configuration\":{{}},{}",
        std::str::from_utf8(content.as_bytes())
            .unwrap()
            .trim_start_matches('{')
    );
    assert_eq!(
        verify(&resign(duplicate.as_bytes())).unwrap_err().code(),
        SnapshotErrorCode::Canonical
    );
    let mut deep = json!(0);
    for _ in 0..41 {
        deep = json!([deep]);
    }
    value["configuration"]["schemaVersion"] = json!(1);
    value["unexpected"] = deep;
    assert_eq!(
        verify(&resign(&serde_json::to_vec(&value).unwrap()))
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Size
    );
}

#[test]
fn signed_bad_metadata_hash_and_non_utf8_do_not_become_candidates() {
    let (key, _) = signer(false);
    let original = encode_snapshot(&empty_content(), metadata(1), &key).unwrap();
    for (offset, code) in [
        (8, SnapshotErrorCode::Schema),
        (12, SnapshotErrorCode::Schema),
        (29, SnapshotErrorCode::Format),
        (45, SnapshotErrorCode::Format),
        (61, SnapshotErrorCode::Format),
        (85, SnapshotErrorCode::Format),
        (89, SnapshotErrorCode::Authentication),
    ] {
        let mut bytes = original.clone();
        match offset {
            29 | 45 | 61 => bytes[offset..offset + 16].fill(0),
            _ => bytes[offset] ^= 1,
        }
        let end = bytes.len() - 64;
        let signature = key.sign_snapshot(&bytes[..end]).unwrap();
        bytes[end..].copy_from_slice(&signature);
        assert_eq!(verify(&bytes).unwrap_err().code(), code, "offset {offset}");
    }
    assert_eq!(
        verify(&resign(&[255])).unwrap_err().code(),
        SnapshotErrorCode::Format
    );
    let mut value: Value = serde_json::from_slice(empty_content().as_bytes()).unwrap();
    value["configuration"]["rawSecret"] = json!("never-accepted-secret");
    assert_eq!(
        verify(&resign(&serde_json::to_vec(&value).unwrap()))
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Configuration
    );
    let maximum = encode_snapshot(&empty_content(), metadata(u64::MAX), &key).unwrap();
    let (_, public) = signer(false);
    let trust = SnapshotTrust::new(vec![(key.id(), public)]).unwrap();
    assert!(
        verify_snapshot(
            &maximum,
            &trust,
            &ReplayState::from_trusted_high_water(
                typed(1),
                SnapshotSequence::new(u64::MAX - 1).unwrap()
            ),
            &empty_resources()
        )
        .is_ok()
    );
}

#[test]
fn parsed_uuid_socket_and_default_spellings_normalize_with_nonempty_content() {
    let first = br#"{"schemaVersion":1,"listeners":[{"id":"10000000-0000-4000-8000-0000000000ab","name":"DNS","enabled":false,"bind":"[::1]:5353","transport":"dns_udp"}]}"#;
    let second = "schemaVersion=1\n[[listeners]]\nid='10000000-0000-4000-8000-0000000000AB'\nname='DNS'\nenabled=false\nbind='[0:0:0:0:0:0:0:1]:5353'\ntransport='dns_udp'\ndualStack=false\n";
    let resources = empty_resources();
    let a = SnapshotContent::new(
        parse_configuration(first, InputFormat::Json, &ReferenceCatalog::default()).unwrap(),
        vec![],
        &resources,
    )
    .unwrap();
    let b = SnapshotContent::new(
        parse_configuration(
            second.as_bytes(),
            InputFormat::Toml,
            &ReferenceCatalog::default(),
        )
        .unwrap(),
        vec![],
        &resources,
    )
    .unwrap();
    assert_eq!(a.as_bytes(), b.as_bytes());
    assert_eq!(a.hash(), b.hash());
}

#[test]
fn normalized_configuration_still_obeys_existing_size_limit() {
    let listeners: Vec<_> = (1..=7800)
        .map(|id| {
            json!({"id":typed::<ListenerId>(id).to_string(),
        "name":"D","enabled":false,"bind":"127.0.0.1:53","transport":"dns_udp"})
        })
        .collect();
    let input = serde_json::to_vec(&json!({"schemaVersion":1,"listeners":listeners})).unwrap();
    assert!(input.len() < crate::MAX_INPUT_BYTES);
    let checked =
        parse_configuration(&input, InputFormat::Json, &ReferenceCatalog::default()).unwrap();
    assert_eq!(
        SnapshotContent::new(checked, vec![], &empty_resources())
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Size
    );
}

fn encrypted(reference: SecretRefId, purpose: SecretPurpose) -> Vec<u8> {
    let mut ring = b"RTKEY\0\x00\x01".to_vec();
    ring.push(1);
    ring.extend_from_slice(&[1; 16]);
    ring.extend_from_slice(&[1; 16]);
    ring.extend_from_slice(&[7; 32]);
    Keyring::from_owned_bytes(ring)
        .unwrap()
        .seal(
            SecretContext { reference, purpose },
            SecretBytes::from_owned_bytes(b"synthetic-snapshot-secret-canary".to_vec()).unwrap(),
        )
        .unwrap()
        .as_bytes()
        .to_vec()
}
fn pins() -> Vec<ResourcePin> {
    let resources = [
        SnapshotResource::Device(typed(120)),
        SnapshotResource::Upstream(typed(90)),
        SnapshotResource::Filter(typed(101)),
        SnapshotResource::Service(typed(100)),
    ];
    let mut pins: Vec<_> = resources
        .into_iter()
        .map(|r| ResourcePin::from_bytes(r, b"independently-verified-fixture").unwrap())
        .collect();
    for (id, purpose) in [
        (98, SecretPurpose::ResolverVerifier),
        (99, SecretPurpose::ProxyOutboundCredential),
    ] {
        pins.push(
            ResourcePin::from_bytes(
                SnapshotResource::Secret {
                    reference: typed(id),
                    purpose,
                },
                &encrypted(typed(id), purpose),
            )
            .unwrap(),
        );
    }
    pins
}
fn links() -> Vec<CredentialLink> {
    [20, 21, 22]
        .into_iter()
        .map(|id| CredentialLink {
            credential: typed(id),
            reference: typed(98),
        })
        .collect()
}
fn full(
    value: &Value,
    resources: &TrustedResources,
    links: Vec<CredentialLink>,
) -> Result<SnapshotContent, SnapshotError> {
    SnapshotContent::new(
        parse_configuration(
            &serde_json::to_vec(value).unwrap(),
            InputFormat::Json,
            &resources.catalog(),
        )
        .unwrap(),
        links,
        resources,
    )
}

#[test]
fn full_fixture_reorders_tables_and_sets_without_changing_policy_or_content() {
    let mut pins = pins();
    let resources = TrustedResources::new(pins.clone()).unwrap();
    let mut value: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/configuration-v1.json")).unwrap();
    let content = full(&value, &resources, links()).unwrap();
    for field in [
        "profiles",
        "endpoints",
        "resolverCredentials",
        "listeners",
        "networkGroups",
        "legacyGrants",
        "gatewayGrants",
        "outbounds",
        "privateRecords",
    ] {
        value[field].as_array_mut().unwrap().reverse();
    }
    for profile in value["profiles"].as_array_mut().unwrap() {
        for field in ["customRules", "serviceRules", "filterLists"] {
            if let Some(rows) = profile.get_mut(field).and_then(Value::as_array_mut) {
                rows.reverse();
            }
        }
    }
    for endpoint in value["endpoints"].as_array_mut().unwrap() {
        endpoint["access"].as_array_mut().unwrap().reverse();
        for access in endpoint["access"].as_array_mut().unwrap() {
            let field = if access.get("credentials").is_some() {
                "credentials"
            } else {
                "grants"
            };
            access[field].as_array_mut().unwrap().reverse();
        }
    }
    pins.reverse();
    let resources = TrustedResources::new(pins).unwrap();
    let mut reversed_links = links();
    reversed_links.reverse();
    let reordered = full(&value, &resources, reversed_links).unwrap();
    assert_eq!(content.as_bytes(), reordered.as_bytes());
    assert_eq!(content.hash(), reordered.hash());
    let (key, public) = signer(false);
    let bytes = encode_snapshot(&content, metadata(1), &key).unwrap();
    let trust = SnapshotTrust::new(vec![(key.id(), public)]).unwrap();
    let verified = verify_snapshot(
        &bytes,
        &trust,
        &ReplayState::first_provisioning(typed(1)),
        &resources,
    )
    .unwrap();
    assert_eq!(verified.content().credential_links().len(), 3);
    assert_eq!(verified.content().resource_pins().len(), 6);
    assert_eq!(
        verified.content().configuration().document().profiles.len(),
        2
    );
    assert!(
        !bytes
            .windows(b"synthetic-snapshot-secret-canary".len())
            .any(|w| w == b"synthetic-snapshot-secret-canary")
    );
    let debug = format!("{verified:?}");
    assert!(!debug.contains("Family"));
}

#[test]
fn resource_substitution_missing_links_wrong_secret_purpose_and_pin_bounds_fail() {
    let pins = pins();
    let resources = TrustedResources::new(pins.clone()).unwrap();
    let value: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/configuration-v1.json")).unwrap();
    assert_eq!(
        full(&value, &resources, vec![]).unwrap_err().code(),
        SnapshotErrorCode::CredentialLink
    );
    let mut duplicate = links();
    duplicate[2] = duplicate[0];
    assert_eq!(
        full(&value, &resources, duplicate).unwrap_err().code(),
        SnapshotErrorCode::CredentialLink
    );
    let content = full(&value, &resources, links()).unwrap();
    let (key, public) = signer(false);
    let bytes = encode_snapshot(&content, metadata(1), &key).unwrap();
    let trust = SnapshotTrust::new(vec![(key.id(), public)]).unwrap();
    let initial = ReplayState::first_provisioning(typed(1));
    assert_eq!(
        verify_snapshot(&bytes, &trust, &initial, &empty_resources())
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Resource
    );
    // Same logical secret under fresh encryption changes its exact artifact pin.
    let mut changed = pins.clone();
    changed[4] = ResourcePin::from_bytes(
        SnapshotResource::Secret {
            reference: typed(98),
            purpose: SecretPurpose::ResolverVerifier,
        },
        &encrypted(typed(98), SecretPurpose::ResolverVerifier),
    )
    .unwrap();
    assert_eq!(
        verify_snapshot(
            &bytes,
            &trust,
            &initial,
            &TrustedResources::new(changed).unwrap()
        )
        .unwrap_err()
        .code(),
        SnapshotErrorCode::Resource
    );
    let mut wrong = pins.clone();
    wrong[5] = ResourcePin::from_bytes(
        SnapshotResource::Secret {
            reference: typed(99),
            purpose: SecretPurpose::ResolverVerifier,
        },
        &encrypted(typed(99), SecretPurpose::ResolverVerifier),
    )
    .unwrap();
    assert_eq!(
        full(&value, &TrustedResources::new(wrong).unwrap(), links())
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Resource
    );
    let resource = SnapshotResource::Secret {
        reference: typed(99),
        purpose: SecretPurpose::ResolverVerifier,
    };
    assert!(ResourcePin::from_bytes(resource, b"raw secret").is_err());
    assert!(
        ResourcePin::from_bytes(
            resource,
            &encrypted(typed(99), SecretPurpose::ProxyOutboundCredential)
        )
        .is_err()
    );
    assert!(
        ResourcePin::from_bytes(resource, &vec![0; droutex_security::MAX_ENVELOPE_BYTES + 1])
            .is_err()
    );
    assert!(TrustedResources::new(vec![pins[0]; 2]).is_err());
    assert_eq!(
        TrustedResources::new(vec![pins[0]; MAX_RESOURCE_PINS + 1])
            .unwrap_err()
            .code(),
        SnapshotErrorCode::Size
    );
}
