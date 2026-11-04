use super::*;

#[test]
fn upstream_group_count_is_bounded_before_building_the_compiled_map() {
    let store = catalogs();
    let mut inputs = data(&store);
    for n in 300..363 {
        inputs.push(ResourceData {
            resource: SnapshotResource::Upstream(id(n)),
            bytes: UPSTREAM,
        });
    }
    assert!(
        CompletePublication::new(
            checked(&document()),
            vec![],
            &inputs,
            &store.revisions().collect::<Vec<_>>()
        )
        .is_ok()
    );
    inputs.push(ResourceData {
        resource: SnapshotResource::Upstream(id(363)),
        bytes: UPSTREAM,
    });
    assert!(matches!(
        CompletePublication::new(
            checked(&document()),
            vec![],
            &inputs,
            &store.revisions().collect::<Vec<_>>()
        ),
        Err(CompilationError::Size)
    ));
}

#[test]
fn cp_and_authenticated_dp_decode_actual_upstream_bytes_without_changing_their_pins() {
    let store = catalogs();
    let published = publication(&document(), &store).unwrap();
    let group = published.compiled().upstream(id(60)).unwrap();
    assert_eq!(group.members()[0].address().to_string(), "1.1.1.1:53");
    let inputs = data(&store);
    let trusted = TrustedResources::new(
        inputs
            .iter()
            .map(|r| ResourcePin::from_bytes(r.resource, r.bytes).unwrap())
            .collect(),
    )
    .unwrap();
    let (key, trust) = key();
    let encoded = published.encode(snapshot_metadata(), &key).unwrap();
    let verified = snapshot::verify_snapshot(
        &encoded,
        &trust,
        &ReplayState::first_provisioning(id(80)),
        &trusted,
    )
    .unwrap();
    let compiled = verified.compile(&inputs).unwrap();
    assert_eq!(compiled.upstream(id(60)), Some(group));
    assert!(compiled.upstream(id(999)).is_none());
    let mut changed = inputs;
    changed
        .iter_mut()
        .find(|r| matches!(r.resource, SnapshotResource::Upstream(_)))
        .unwrap()
        .bytes = b"different descriptor";
    assert!(matches!(
        verified.compile(&changed),
        Err(CompilationError::Resource)
    ));
}

#[test]
fn even_a_signed_and_hash_matching_opaque_or_private_upstream_is_refused_by_dp() {
    let store = catalogs();
    let (key, trust) = key();
    for invalid in [b"old opaque upstream".as_slice(), br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"udp_tcp","address":"127.0.0.1:53"}]}"#] {
        let mut inputs = data(&store);
        inputs.iter_mut().find(|r| matches!(r.resource, SnapshotResource::Upstream(_))).unwrap().bytes = invalid;
        assert!(matches!(CompletePublication::new(checked(&document()), vec![], &inputs, &store.revisions().collect::<Vec<_>>()), Err(CompilationError::Upstream(_, _))));
        let trusted = TrustedResources::new(inputs.iter().map(|r| ResourcePin::from_bytes(r.resource, r.bytes).unwrap()).collect()).unwrap();
        // Low-level codec can sign arbitrary data; DP compilation must still enforce D53.
        let content = SnapshotContent::new(checked(&document()), vec![], &trusted).unwrap();
        let encoded = snapshot::encode_snapshot(&content, snapshot_metadata(), &key).unwrap();
        let verified = snapshot::verify_snapshot(&encoded, &trust, &ReplayState::first_provisioning(id(80)), &trusted).unwrap();
        assert!(matches!(verified.compile(&inputs), Err(CompilationError::Upstream(_, _))));
    }
}

#[test]
fn origin_resolver_is_explicit_snapshot_pinned_and_independent_of_dns_profile() {
    let store = catalogs();
    let mut doc = document();
    doc["schemaVersion"] = json!(2);
    for table in ["legacyGrants", "gatewayGrants"] {
        for grant in doc[table].as_array_mut().unwrap() {
            grant["lease"] = json!({"issuedAt":1000,"expiresAt":2000,"cause":"manual"});
        }
    }
    doc["originUpstream"] = json!(text(60));
    doc["profiles"][0]["defaultDnsAction"] = json!({"kind":"block"});
    doc["profiles"][0]["customRules"] = json!([]);
    let publication = publication(&doc, &store).unwrap();
    assert_eq!(
        publication
            .content()
            .configuration()
            .document()
            .origin_upstream,
        Some(id(60))
    );
    assert_eq!(
        publication.compiled().upstream(id(60)).unwrap().members()[0]
            .address()
            .to_string(),
        "1.1.1.1:53"
    );
    let inputs = data(&store);
    let trust_resources = TrustedResources::new(
        inputs
            .iter()
            .map(|r| ResourcePin::from_bytes(r.resource, r.bytes).unwrap())
            .collect(),
    )
    .unwrap();
    let (key, trust) = key();
    let signed = publication.encode(snapshot_metadata(), &key).unwrap();
    let verified = snapshot::verify_snapshot(
        &signed,
        &trust,
        &ReplayState::first_provisioning(id(80)),
        &trust_resources,
    )
    .unwrap();
    assert_eq!(
        verified
            .content()
            .configuration()
            .document()
            .origin_upstream,
        Some(id(60))
    );
    assert!(
        verified
            .compile(&inputs)
            .unwrap()
            .upstream(id(60))
            .is_some()
    );
    let mut missing = data(&store);
    missing.retain(|r| !matches!(r.resource, SnapshotResource::Upstream(_)));
    assert!(
        CompletePublication::new(
            checked(&doc),
            vec![],
            &missing,
            &store.revisions().collect::<Vec<_>>()
        )
        .is_err(),
        "unpinned origin resolver must fail publication"
    );
    let mut bad = doc.clone();
    bad["originUpstream"] = json!(text(999));
    assert!(
        parse_configuration(
            &serde_json::to_vec(&bad).unwrap(),
            InputFormat::Json,
            &references()
        )
        .is_err()
    );
}
