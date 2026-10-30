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
