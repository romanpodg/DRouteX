use super::*;
use droutex_config::{
    InputFormat, ReferenceCatalog, parse_configuration,
    snapshot::{
        ContentHash, ResourcePin, SnapshotContent, SnapshotSequence, TrustedResources,
        encode_snapshot,
    },
};
use droutex_security::{SigningKey, SigningKeyId};
use std::str::FromStr;
mod admission_tests;
mod explain_tests;

fn typed<T: FromStr>(n: usize) -> T
where
    T::Err: fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn signer(second: bool) -> (SigningKey, [u8; 32]) {
    // Published RFC8032 seeds, never operational credentials.
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
    let public = hex(public).try_into().unwrap();
    let key = SigningKey::from_pkcs8(
        SigningKeyId::from_bytes([if second { 2 } else { 1 }; 16]).unwrap(),
        hex(&format!("302e020100300506032b657004220420{seed}")),
        &public,
    )
    .unwrap();
    (key, public)
}
fn trust(second: bool) -> SnapshotTrust {
    let (key, public) = signer(second);
    SnapshotTrust::new(vec![(key.id(), public)]).unwrap()
}
fn resource() -> SnapshotResource {
    SnapshotResource::Device(typed(99))
}

fn compiled_candidate(
    sequence: u64,
    conflict: bool,
    private_record: bool,
    malformed_catalog: bool,
) -> (Vec<u8>, Artifacts) {
    use droutex_config::catalog::{
        CatalogFormat, CatalogInput, CatalogLicenseApproval, CatalogMetadata, CatalogStore,
    };
    // Parse/approve a real local catalog, then exercise the complete CP builder
    // through signed bytes and actual DP Generation construction/recovery.
    let revision = SnapshotResource::Filter(typed(92)).try_into().unwrap();
    let source = b"catalog.test\n";
    let mut catalogs = CatalogStore::default();
    catalogs
        .import_batch(
            vec![CatalogInput {
                revision,
                format: CatalogFormat::Domains(
                    droutex_config::DomainMatch {
                        mode: droutex_config::MatchMode::Suffix,
                        value: "catalog.test".into(),
                    }
                    .normalized()
                    .unwrap()
                    .mode(),
                ),
                metadata: CatalogMetadata {
                    source: "local:core-compile-fixture".into(),
                    owner: "DRouteX".into(),
                    license: "Apache-2.0".into(),
                    notice: "Original synthetic fixture.".into(),
                },
                bytes: source,
            }],
            vec![
                CatalogLicenseApproval::new(
                    revision,
                    ContentHash::of(source),
                    "Apache-2.0".into(),
                    "owner:synthetic-test".into(),
                )
                .unwrap(),
            ],
        )
        .unwrap();
    let document = serde_json::json!({"schemaVersion":1,"profiles":[{
        "id":typed::<droutex_common::ProfileId>(77).to_string(),"name":"compiled generation",
        "defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"deny"},
        "filterLists":[{"revision":typed::<droutex_common::FilterListRevisionId>(92).to_string(),"enabled":true,"priority":0}],
        "customRules":if conflict { serde_json::json!([
            {"id":typed::<droutex_common::RuleId>(100).to_string(),"enabled":true,"priority":0,
             "matcher":{"mode":"exact","value":"private.test"},"action":{"plane":"traffic","action":{"kind":"deny"}}},
            {"id":typed::<droutex_common::RuleId>(101).to_string(),"enabled":true,"priority":0,
             "matcher":{"mode":"exact","value":"private.test"},"action":{"plane":"traffic","action":{"kind":"outbound","outbound":typed::<droutex_common::ProxyOutboundId>(90).to_string()}}}
        ]) } else { serde_json::json!([]) }}],
        "listeners":[{"id":typed::<droutex_common::ListenerId>(79).to_string(),"name":"legacy diagnostic fixture","enabled":true,"bind":"127.0.0.1:5353","transport":"dns_udp"}],
        "endpoints":[{"id":typed::<droutex_common::EndpointId>(78).to_string(),"name":"private DNS metadata","enabled":true,"profile":typed::<droutex_common::ProfileId>(77).to_string(),"access":[{"transport":"udp","listener":typed::<droutex_common::ListenerId>(79).to_string(),"grants":[typed::<droutex_common::LegacyNetworkGrantId>(81).to_string()]}]}],
        "networkGroups":[{"id":typed::<droutex_common::NetworkGroupId>(80).to_string(),"name":"private network metadata","trafficProfile":typed::<droutex_common::ProfileId>(77).to_string()}],
        "legacyGrants":[{"id":typed::<droutex_common::LegacyNetworkGrantId>(81).to_string(),"source":"203.0.113.7","listener":typed::<droutex_common::ListenerId>(79).to_string(),"group":typed::<droutex_common::NetworkGroupId>(80).to_string(),"endpoint":typed::<droutex_common::EndpointId>(78).to_string()}],
        "outbounds":[{"id":typed::<droutex_common::ProxyOutboundId>(90).to_string(),"name":"explicit direct","enabled":true,"transport":{"kind":"vps_direct"}}],
        "privateRecords":if private_record { serde_json::json!([{"id":typed::<droutex_common::PrivateDnsRecordId>(91).to_string(),
            "profile":typed::<droutex_common::ProfileId>(77).to_string(),"name":"retained.test","ttl":60,"record":{"kind":"a","address":"192.0.2.7"}}]) } else { serde_json::json!([]) }});
    let artifacts = Artifacts::new(if malformed_catalog {
        vec![(
            SnapshotResource::Filter(typed(92)),
            b"not canonical catalog JSON".to_vec(),
        )]
    } else {
        vec![(
            SnapshotResource::Filter(typed(92)),
            catalogs.get(revision).unwrap().resource_bytes().to_vec(),
        )]
    })
    .unwrap();
    let configuration = parse_configuration(
        &serde_json::to_vec(&document).unwrap(),
        InputFormat::Json,
        &ReferenceCatalog {
            filter_revisions: [typed(92)].into(),
            ..Default::default()
        },
    )
    .unwrap();
    let metadata = SnapshotMetadata {
        node: typed(1),
        snapshot: typed(2),
        source: typed(3),
        sequence: SnapshotSequence::new(sequence).unwrap(),
    };
    let encoded = if conflict || malformed_catalog {
        // Deliberate low-level signing for negative authenticated-input probes.
        let content =
            SnapshotContent::new(configuration, vec![], &artifacts.trusted().unwrap()).unwrap();
        encode_snapshot(&content, metadata, &signer(false).0).unwrap()
    } else {
        droutex_config::CompletePublication::new(
            configuration,
            vec![],
            &artifacts.data(),
            &catalogs.revisions().collect::<Vec<_>>(),
        )
        .unwrap()
        .encode(metadata, &signer(false).0)
        .unwrap()
    };
    (encoded, artifacts)
}

#[test]
fn complete_compilation_gates_fresh_and_retained_candidates_without_advancing_lkg() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    let (bytes, artifacts) = compiled_candidate(1, false, false, false);
    deployment
        .activate(bytes, artifacts, &Cancellation::default())
        .unwrap();
    let old = deployment.pin().unwrap();
    for malformed in [false, true] {
        let (bytes, artifacts) = compiled_candidate(2, !malformed, false, malformed);
        assert_eq!(
            failure(deployment.activate(bytes, artifacts, &Cancellation::default())),
            ErrorCode::Compilation
        );
        let current = deployment.pin().unwrap();
        assert_eq!(current.metadata().sequence.get(), 1);
        assert!(std::ptr::eq(
            old.compiled_configuration(),
            current.compiled_configuration()
        ));
    }
    let (bytes, artifacts) = compiled_candidate(2, false, true, false);
    deployment
        .activate(bytes, artifacts, &Cancellation::default())
        .unwrap();
    assert!(old.compiled_configuration().private_records().is_empty());
    assert_eq!(
        deployment
            .pin()
            .unwrap()
            .compiled_configuration()
            .private_records()[0]
            .owner
            .as_str(),
        "retained.test"
    );
    // A correctly authenticated retained snapshot still must compile. Construct
    // its independently trusted identity here, rather than failing on a bad hash.
    let (good, artifacts) = compiled_candidate(1, false, false, false);
    let generation =
        Generation::fresh(good, artifacts, &trust(false), &Journal::empty(typed(1))).unwrap();
    let descriptor = Descriptor::new(&generation, 0);
    let (bad, artifacts) = compiled_candidate(1, false, false, true);
    let mut wire = serde_json::to_value(descriptor).unwrap();
    wire["hash"] = serde_json::json!(ContentHash::of(&bad).as_bytes());
    let descriptor = serde_json::from_value(wire).unwrap();
    assert!(
        matches!(Generation::restore(bad,artifacts,&trust(false),&descriptor),Err(e) if e.code()==ErrorCode::Compilation)
    );
}
fn candidate(sequence: u64, second: bool, value: &[u8]) -> (Vec<u8>, Artifacts) {
    let pins =
        TrustedResources::new(vec![ResourcePin::from_bytes(resource(), value).unwrap()]).unwrap();
    let document = serde_json::json!({"schemaVersion":1, "profiles":[{
        "id": typed::<droutex_common::ProfileId>(77).to_string(),
        "name": String::from_utf8_lossy(value), "defaultDnsAction":{"kind":"block"},
        "defaultTrafficAction":{"kind":"deny"}}]});
    let content = SnapshotContent::new(
        parse_configuration(
            &serde_json::to_vec(&document).unwrap(),
            InputFormat::Json,
            &ReferenceCatalog::default(),
        )
        .unwrap(),
        vec![],
        &pins,
    )
    .unwrap();
    let metadata = SnapshotMetadata {
        node: typed(1),
        snapshot: typed(2),
        source: typed(3),
        sequence: SnapshotSequence::new(sequence).unwrap(),
    };
    (
        encode_snapshot(&content, metadata, &signer(second).0).unwrap(),
        Artifacts::new(vec![(resource(), value.to_vec())]).unwrap(),
    )
}
fn apply(
    deployment: &Deployment,
    sequence: u64,
    second: bool,
    value: &[u8],
) -> Result<SnapshotMetadata> {
    let (bytes, artifacts) = candidate(sequence, second, value);
    deployment.activate(bytes, artifacts, &Cancellation::default())
}
fn failure<T>(result: Result<T>) -> ErrorCode {
    match result {
        Ok(_) => panic!("unexpected success"),
        Err(error) => error.code(),
    }
}
fn assert_version(deployment: &Deployment, sequence: u64, bytes: &[u8]) {
    let decision = deployment.pin().unwrap();
    assert_eq!(decision.metadata().sequence.get(), sequence);
    assert_eq!(decision.artifact(resource()).unwrap(), bytes);
    assert_eq!(
        decision.dns_configuration().document().profiles[0]
            .name
            .as_bytes(),
        bytes
    );
    assert_eq!(
        decision.gateway_configuration().document().profiles[0]
            .name
            .as_bytes(),
        bytes
    );
    assert!(std::ptr::eq(
        decision.dns_configuration(),
        decision.gateway_configuration()
    ));
}

#[test]
fn shared_generation_and_previous_reader_backpressure() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    assert_eq!(failure(deployment.pin()), ErrorCode::Unavailable);
    apply(&deployment, 1, false, b"old").unwrap();
    let old_dns_decision = deployment.pin().unwrap();
    apply(&deployment, 2, false, b"new").unwrap();
    assert_eq!(old_dns_decision.artifact(resource()).unwrap(), b"old");
    assert!(std::ptr::eq(
        old_dns_decision.dns_configuration(),
        old_dns_decision.gateway_configuration()
    ));
    assert_version(&deployment, 2, b"new");
    assert_eq!(
        failure(apply(&deployment, 3, false, b"third")),
        ErrorCode::Busy
    );
    assert_version(&deployment, 2, b"new");
    drop(old_dns_decision);
    apply(&deployment, 3, false, b"third").unwrap();
    assert_version(&deployment, 3, b"third");
}

#[test]
fn replay_gap_rotation_and_fresh_rollback_keep_one_floor() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    apply(&deployment, 7, false, b"old").unwrap(); // Gaps are permitted by D11.
    for sequence in [1, 6, 7] {
        assert_eq!(
            failure(apply(&deployment, sequence, false, b"replay")),
            ErrorCode::Snapshot(snapshot::SnapshotErrorCode::Replay)
        );
    }
    let (first, p1) = signer(false);
    let (second, p2) = signer(true);
    assert!(
        deployment
            .replace_trust(SnapshotTrust::new(vec![(first.id(), p1), (second.id(), p2)]).unwrap())
            .unwrap()
    );
    apply(&deployment, 9, true, b"changed").unwrap();
    assert!(deployment.replace_trust(trust(true)).unwrap());
    assert_eq!(
        failure(apply(&deployment, 10, false, b"revoked")),
        ErrorCode::Snapshot(snapshot::SnapshotErrorCode::Authentication)
    );
    apply(&deployment, 10, true, b"old").unwrap(); // Rollback content requires a new signature/sequence.
    apply(&deployment, u64::MAX, true, b"last").unwrap();
    assert_eq!(
        failure(apply(&deployment, u64::MAX, true, b"overflow")),
        ErrorCode::Snapshot(snapshot::SnapshotErrorCode::Replay)
    );
    assert_version(&deployment, u64::MAX, b"last");
}

#[test]
fn refusal_preserves_active_and_revocation_stops_new_pins() {
    let deployment = Deployment::in_memory(typed(1), trust(false));
    apply(&deployment, 1, false, b"good").unwrap();
    let captured = deployment.pin().unwrap();
    let (mut bytes, artifacts) = candidate(2, false, b"bad");
    bytes[140] ^= 1;
    assert_eq!(
        failure(deployment.activate(bytes, artifacts, &Cancellation::default())),
        ErrorCode::Snapshot(snapshot::SnapshotErrorCode::Authentication)
    );
    let (bytes, _) = candidate(2, false, b"signed");
    assert!(
        deployment
            .activate(
                bytes,
                Artifacts::new(vec![]).unwrap(),
                &Cancellation::default()
            )
            .is_err()
    );
    let (bytes, artifacts) = candidate(2, false, b"cancelled");
    let cancelled = Cancellation::default();
    cancelled.cancel();
    assert_eq!(
        failure(deployment.activate(bytes, artifacts, &cancelled)),
        ErrorCode::Cancelled
    );
    assert_version(&deployment, 1, b"good");
    assert!(!deployment.replace_trust(trust(true)).unwrap());
    assert_eq!(failure(deployment.pin()), ErrorCode::Unavailable);
    assert_eq!(captured.artifact(resource()).unwrap(), b"good");
    apply(&deployment, 2, true, b"recovered").unwrap();
    assert_version(&deployment, 2, b"recovered");
}

#[test]
fn concurrent_consumers_keep_their_captured_generation() {
    let deployment = Arc::new(Deployment::in_memory(typed(1), trust(false)));
    apply(&deployment, 1, false, b"one").unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let reader = {
        let deployment = Arc::clone(&deployment);
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            let decision = deployment.pin().unwrap();
            let dns = ContentHash::of(decision.artifact(resource()).unwrap());
            barrier.wait();
            barrier.wait();
            let gateway = ContentHash::of(decision.artifact(resource()).unwrap());
            assert_eq!(dns, gateway);
            assert_eq!(decision.metadata().sequence.get(), 1);
        })
    };
    barrier.wait();
    apply(&deployment, 2, false, b"two").unwrap();
    barrier.wait();
    reader.join().unwrap();
    assert_version(&deployment, 2, b"two");
}

#[test]
fn wrong_scope_extra_duplicate_and_budgeted_artifacts_refuse() {
    let deployment = Deployment::in_memory(typed(7), trust(false));
    assert_eq!(
        failure(apply(&deployment, 1, false, b"wrong")),
        ErrorCode::Snapshot(snapshot::SnapshotErrorCode::Scope)
    );
    assert_eq!(
        failure(Artifacts::new(vec![
            (resource(), vec![]),
            (resource(), vec![])
        ])),
        ErrorCode::Resource
    );
    assert_eq!(
        failure(Artifacts::new(vec![(
            resource(),
            vec![0; MAX_RESOURCE_BYTES + 1]
        )])),
        ErrorCode::Size
    );
    let deployment = Deployment::in_memory(typed(1), trust(false));
    let (bytes, _) = candidate(1, false, b"signed");
    let extra = Artifacts::new(vec![
        (resource(), b"signed".to_vec()),
        (SnapshotResource::Device(typed(100)), vec![]),
    ])
    .unwrap();
    assert_eq!(
        failure(deployment.activate(bytes, extra, &Cancellation::default())),
        ErrorCode::Resource
    );
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
#[test]
fn filesystem_never_silently_becomes_ephemeral() {
    assert_eq!(
        failure(Deployment::open(
            Path::new("/absent"),
            typed(1),
            trust(false)
        )),
        ErrorCode::UnsupportedPlatform
    );
    assert_eq!(
        failure(Deployment::provision(
            Path::new("/absent"),
            Path::new("/cp"),
            Path::new("/backup"),
            typed(1),
            trust(false)
        )),
        ErrorCode::UnsupportedPlatform
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(super) fn checkpoint(phase: &str) -> Result<()> {
    linux::checkpoint(phase)
}
