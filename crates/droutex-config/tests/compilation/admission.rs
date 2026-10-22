use super::*;

fn leased() -> Value {
    let mut value = document();
    value["schemaVersion"] = json!(2);
    for table in ["legacyGrants", "gatewayGrants"] {
        for grant in value[table].as_array_mut().unwrap() {
            grant["lease"] = json!({"cause":"manual","issuedAt":100,"expiresAt":1000});
        }
    }
    value
}
fn parse(value: &Value) -> Result<CheckedConfiguration, ConfigError> {
    parse_configuration(
        &serde_json::to_vec(value).unwrap(),
        InputFormat::Json,
        &references(),
    )
}
#[test]
fn v2_requires_bounded_manual_lease_and_v1_remains_association_only() {
    for lifetime in [60, 86400] {
        let mut value = leased();
        value["legacyGrants"][0]["lease"]["expiresAt"] = json!(100 + lifetime);
        assert!(parse(&value).is_ok());
    }
    for lease in [
        json!(null),
        json!([100, 1000, "manual"]),
        json!({"cause":"manual","issuedAt":100,"expiresAt":159}),
        json!({"cause":"manual","issuedAt":100,"expiresAt":86501}),
        json!({"cause":"manual","issuedAt":100,"expiresAt":99}),
        json!({"cause":"manual","issuedAt":253402300700u64,"expiresAt":253402300800u64}),
        json!({"cause":"secure_observation","issuedAt":100,"expiresAt":1000}),
        json!({"cause":"manual","issuedAt":100,"expiresAt":1000,"verified":true}),
        json!({"cause":"manual","issuedAt":"100","expiresAt":1000}),
    ] {
        let mut value = leased();
        value["gatewayGrants"][0]["lease"] = lease;
        assert!(parse(&value).is_err());
    }
    let mut missing = leased();
    missing["legacyGrants"][0]
        .as_object_mut()
        .unwrap()
        .remove("lease");
    assert!(parse(&missing).is_err());
    let mut old = leased();
    old["schemaVersion"] = json!(1);
    assert!(parse(&old).is_err());
    let mut duplicate = leased();
    let mut second = duplicate["gatewayGrants"][0].clone();
    second["id"] = json!(text(400));
    duplicate["gatewayGrants"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert!(parse(&duplicate).is_err());
    assert!(parse(&document()).is_ok());
}

#[test]
fn signed_v2_header_payload_and_resources_bind_the_same_schema() {
    let store = catalogs();
    let document = leased();
    let publication = publication(&document, &store).unwrap();
    let (key, trust) = key();
    let metadata = snapshot_metadata();
    let bytes = publication.encode(metadata, &key).unwrap();
    assert_eq!(&bytes[8..12], &2u32.to_be_bytes());
    let refs = TrustedResources::new(
        data(&store)
            .iter()
            .map(|r| ResourcePin::from_bytes(r.resource, r.bytes).unwrap())
            .collect(),
    )
    .unwrap();
    let replay = ReplayState::first_provisioning(metadata.node);
    let verified = snapshot::verify_snapshot(&bytes, &trust, &replay, &refs).unwrap();
    assert_eq!(
        verified.content().configuration().document().schema_version,
        2
    );
    assert!(verified.compile(&data(&store)).is_ok());
    let signature_at = bytes.len() - 64;
    let mut mixed = bytes.clone();
    mixed[8..12].copy_from_slice(&1u32.to_be_bytes());
    let signature = key.sign_snapshot(&mixed[..signature_at]).unwrap();
    mixed[signature_at..].copy_from_slice(&signature);
    assert_eq!(
        snapshot::verify_snapshot(&mixed, &trust, &replay, &refs)
            .unwrap_err()
            .code(),
        snapshot::SnapshotErrorCode::Schema
    );
    let mut tampered = bytes.clone();
    tampered[8..12].copy_from_slice(&1u32.to_be_bytes());
    assert_eq!(
        snapshot::verify_snapshot(&tampered, &trust, &replay, &refs)
            .unwrap_err()
            .code(),
        snapshot::SnapshotErrorCode::Authentication
    );
    assert!(
        snapshot::verify_snapshot(
            &bytes,
            &trust,
            &ReplayState::from_trusted_high_water(metadata.node, metadata.sequence),
            &refs
        )
        .is_err()
    );
    let mut unsupported = bytes;
    unsupported[8..12].copy_from_slice(&3u32.to_be_bytes());
    assert_eq!(
        snapshot::verify_snapshot(&unsupported, &trust, &replay, &refs)
            .unwrap_err()
            .code(),
        snapshot::SnapshotErrorCode::Schema
    );
}

#[test]
fn gateway_candidate_uses_real_service_scope_and_independent_policy_action() {
    use droutex_common::{OutboundAvailability, ProxyOutboundKind};
    use droutex_policy::TcpCapability;
    let store = catalogs();
    let publication = publication(&leased(), &store).unwrap();
    let observation = || {
        vec![OutboundObservation {
            id: id(50),
            kind: ProxyOutboundKind::GatewayDirect,
            availability: OutboundAvailability::Available,
            tcp: TcpCapability::Supported,
        }]
    };
    for name in [
        "allowed.example.test",
        "service.example.test",
        "cdn.own.example.test",
    ] {
        let candidate = publication
            .content()
            .gateway_candidate(
                publication.compiled(),
                identity(),
                name,
                NonZeroU16::new(443).unwrap(),
                observation(),
            )
            .unwrap();
        assert_eq!(candidate.outbound(), id(50));
        assert_eq!(candidate.action().profile(), id(2));
    }
    for name in [
        "dep-only.example.test",
        "own.example.test",
        "service.example.test.attacker.test",
        "blocked.example.test",
    ] {
        assert!(
            publication
                .content()
                .gateway_candidate(
                    publication.compiled(),
                    identity(),
                    name,
                    NonZeroU16::new(443).unwrap(),
                    observation()
                )
                .is_err()
        );
    }
    let mut wrong = identity();
    wrong.group = id(99);
    assert!(
        publication
            .content()
            .gateway_candidate(
                publication.compiled(),
                wrong,
                "service.example.test",
                NonZeroU16::new(443).unwrap(),
                observation()
            )
            .is_err()
    );
    assert!(
        publication
            .content()
            .gateway_candidate(
                publication.compiled(),
                identity(),
                "service.example.test",
                NonZeroU16::new(80).unwrap(),
                observation()
            )
            .is_err()
    );
}
