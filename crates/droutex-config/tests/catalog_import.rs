use droutex_common::{FilterListRevisionId, MAX_LOCAL_INPUT_BYTES};
use droutex_config::{
    catalog::{
        CatalogFormat as Format, CatalogImportError as Error, CatalogInput,
        CatalogLicenseApproval as Approval, CatalogMetadata, CatalogStore,
    },
    snapshot::{ContentHash, ResourcePin, SnapshotResource},
};
use droutex_policy::{CatalogRevision as Revision, DomainMatchMode as Mode, MAX_POLICY_ENTRIES};
use serde_json::{Value, json};
use std::{error::Error as _, str::FromStr};

const DOMAINS: &[u8] = include_bytes!("../../../tests/fixtures/catalogs/domains.txt");
const SERVICE: &[u8] = include_bytes!("../../../tests/fixtures/catalogs/service-v1.json");
const GOLDEN: &[u8] = include_bytes!("../../../tests/fixtures/catalogs/filter-resource-v1.json");

fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn filter(n: usize) -> Revision {
    Revision::Filter(id(n))
}
fn service(n: usize) -> Revision {
    Revision::Service(id(n))
}
fn metadata() -> CatalogMetadata {
    CatalogMetadata {
        source: "local:test-fixture".into(),
        owner: "DRouteX".into(),
        license: "Apache-2.0".into(),
        notice: "Original DRouteX synthetic test fixtures.".into(),
    }
}
fn input(revision: Revision, bytes: &[u8]) -> CatalogInput<'_> {
    CatalogInput {
        revision,
        format: match revision {
            Revision::Filter(_) => Format::Domains(Mode::DomainAndSubdomains),
            Revision::Service(_) => Format::ServiceJson,
        },
        metadata: metadata(),
        bytes,
    }
}
fn approval(revision: Revision, bytes: &[u8]) -> Approval {
    Approval::new(
        revision,
        ContentHash::of(bytes),
        "Apache-2.0".into(),
        "owner:test-fixture-approval".into(),
    )
    .unwrap()
}
fn base() -> CatalogStore {
    let mut s = CatalogStore::default();
    s.import_batch(
        vec![input(filter(300), DOMAINS)],
        vec![approval(filter(300), DOMAINS)],
    )
    .unwrap();
    s
}
fn fixture() -> Value {
    serde_json::from_slice(SERVICE).unwrap()
}

#[test]
fn actual_source_and_canonical_hashes_match_independent_golden_and_snapshot_pin() {
    let mut store = CatalogStore::default();
    let summary = store
        .import_batch(
            vec![input(filter(300), DOMAINS)],
            vec![approval(filter(300), DOMAINS)],
        )
        .unwrap();
    assert_eq!(
        (
            summary.supplied_revisions,
            summary.raw_entries,
            summary.canonical_entries
        ),
        (1, 3, 2)
    );
    let revision = store.get(filter(300)).unwrap();
    assert_eq!(revision.resource_bytes(), GOLDEN);
    assert_eq!(revision.source_bytes(), DOMAINS);
    assert_eq!(
        revision.source_hash().to_string(),
        "2a93ea740d4dd8e372743a7e30ffd90e3e0ab12ffde593eb487fd2dd7ff13bbb"
    );
    assert_eq!(
        ContentHash::of(revision.resource_bytes()).to_string(),
        "c51d5eada3cfa3a9faf8238531646ac00fd9693fa7abe9c56bea2e12648d309b"
    );
    assert_ne!(
        revision.source_hash(),
        ContentHash::of(revision.resource_bytes())
    );
    let pin = ResourcePin::from_bytes(
        SnapshotResource::Filter(id::<FilterListRevisionId>(300)),
        revision.resource_bytes(),
    )
    .unwrap();
    assert_eq!(pin.hash().as_bytes(), &revision.pin().hash);
    assert_eq!(revision.facts().actual_hash, revision.pin().hash);
    assert_eq!(revision.metadata().notice, metadata().notice);
    assert_eq!(revision.approval_evidence(), "owner:test-fixture-approval");
    assert_eq!(
        revision.format(),
        Format::Domains(Mode::DomainAndSubdomains)
    );
}

#[test]
fn actual_service_dependencies_and_explicit_conditions_are_order_independent() {
    let mut expected = None;
    for reversed in [false, true] {
        let mut inputs = vec![input(filter(300), DOMAINS), input(service(301), SERVICE)];
        let mut approvals = vec![
            approval(filter(300), DOMAINS),
            approval(service(301), SERVICE),
        ];
        if reversed {
            inputs.reverse();
            approvals.reverse();
        }
        let mut store = CatalogStore::default();
        store.import_batch(inputs, approvals).unwrap();
        let revision = store.get(service(301)).unwrap();
        assert_eq!(
            revision.dependencies(),
            &[store.get(filter(300)).unwrap().pin()]
        );
        let conditions = revision.conditions().entries();
        for (name, apex, child) in [
            ("assets.example", false, true),
            ("cdn.example", true, true),
            ("video.example", true, false),
        ] {
            let matcher = conditions
                .iter()
                .find(|m| m.domain().as_str() == name)
                .unwrap();
            assert_eq!(matcher.matches(&name.parse().unwrap()), apex);
            assert_eq!(
                matcher.matches(&format!("x.{name}").parse().unwrap()),
                child
            );
        }
        if let Some(expected) = &expected {
            assert_eq!(revision.resource_bytes(), expected);
        } else {
            expected = Some(revision.resource_bytes().to_vec());
        }
        let debug = format!("{store:?} {revision:?}");
        assert!(!debug.contains("video.example"));
        assert!(!debug.contains("local:test-fixture"));
    }
}

#[test]
fn identities_never_rebind_and_identical_imports_are_idempotent_without_latest_selection() {
    let mut store = base();
    let original = store.get(filter(300)).unwrap().resource_bytes().to_vec();
    store
        .import_batch(
            vec![input(filter(300), DOMAINS)],
            vec![approval(filter(300), DOMAINS)],
        )
        .unwrap();
    assert_eq!(store.revisions().count(), 1);
    let changed=b"# different bytes, equivalent normalized conditions\ncdn--edge.example\nxn--bcher-kva.example\n";
    assert_eq!(
        store.import_batch(
            vec![input(filter(300), changed)],
            vec![approval(filter(300), changed)]
        ),
        Err(Error::ImmutableRevision(filter(300)))
    );
    store
        .import_batch(
            vec![input(filter(302), changed)],
            vec![approval(filter(302), changed)],
        )
        .unwrap();
    assert_eq!(store.revisions().count(), 2);
    assert_eq!(store.get(filter(300)).unwrap().resource_bytes(), original);
    assert_ne!(
        store.get(filter(300)).unwrap().pin().hash,
        store.get(filter(302)).unwrap().pin().hash
    );
    assert_eq!(
        store.get(filter(300)).unwrap().conditions().entries(),
        store.get(filter(302)).unwrap().conditions().entries()
    );
    // No update removes the old immutable identity, even after another is added.
    assert_eq!(
        store.import_batch(
            vec![input(filter(300), changed)],
            vec![approval(filter(300), changed)]
        ),
        Err(Error::ImmutableRevision(filter(300)))
    );
}

#[test]
fn failed_multi_revision_updates_leave_previous_data_and_all_new_candidates_unpublished() {
    let mut store = base();
    let original = store.get(filter(300)).unwrap().resource_bytes().to_vec();
    let new = b"new.example\n";
    for target in [filter(999), service(301)] {
        let mut value = fixture();
        value["dependencies"][0]["kind"] = match target {
            Revision::Filter(_) => "filter",
            Revision::Service(_) => "service",
        }
        .into();
        value["dependencies"][0]["revision"] = match target {
            Revision::Filter(i) => i.to_string(),
            Revision::Service(i) => i.to_string(),
        }
        .into();
        value["dependencies"][0]["hash"] = "00".repeat(32).into();
        let bad = serde_json::to_vec(&value).unwrap();
        let result = store.import_batch(
            vec![input(filter(303), new), input(service(301), &bad)],
            vec![approval(service(301), &bad), approval(filter(303), new)],
        );
        assert!(matches!(result, Err(Error::Dependencies(_))));
        assert_eq!(store.revisions().count(), 1);
        assert!(store.get(filter(303)).is_none());
        assert_eq!(store.get(filter(300)).unwrap().resource_bytes(), original);
    }
    for invalid in [
        b"# empty update".as_slice(),
        b"broken..example",
        b"https://bad.example",
    ] {
        assert!(
            store
                .import_batch(
                    vec![input(filter(300), invalid)],
                    vec![approval(filter(300), invalid)]
                )
                .is_err()
        );
        assert_eq!(store.get(filter(300)).unwrap().resource_bytes(), original);
    }
    assert_eq!(store.import_batch(vec![], vec![]), Err(Error::Empty));
}

#[test]
fn approvals_must_match_exact_revision_source_hash_and_license_and_cannot_come_from_payload() {
    let mut store = base();
    for approved in [
        vec![],
        vec![approval(filter(301), DOMAINS)],
        vec![approval(filter(300), b"different.example")],
        vec![
            Approval::new(
                filter(300),
                ContentHash::of(DOMAINS),
                "MIT".into(),
                "fixture-evidence".into(),
            )
            .unwrap(),
        ],
        vec![
            approval(filter(300), DOMAINS),
            approval(filter(300), DOMAINS),
        ],
    ] {
        assert!(
            store
                .import_batch(vec![input(filter(300), DOMAINS)], approved)
                .is_err()
        );
        assert_eq!(store.revisions().count(), 1);
    }
    for evidence in ["", "\t", "unsafe\0claim"] {
        assert!(
            Approval::new(
                filter(300),
                ContentHash::of(DOMAINS),
                "Apache-2.0".into(),
                evidence.into()
            )
            .is_err()
        );
    }
    let mut payload = fixture();
    payload["approved"] = true.into();
    let bytes = serde_json::to_vec(&payload).unwrap();
    assert_eq!(
        store.import_batch(
            vec![input(service(301), &bytes)],
            vec![approval(service(301), &bytes)]
        ),
        Err(Error::Shape)
    );
}

#[test]
fn strict_service_shape_types_duplicates_unsupported_actions_and_pins_are_rejected() {
    let mut invalid = vec![];
    for (field, value) in [
        ("schemaVersion", json!(2)),
        ("schemaVersion", json!(1.0)),
        ("schemaVersion", json!("1")),
        ("kind", json!("filter")),
        ("entries", json!([])),
        ("entries", Value::Null),
        ("dependencies", Value::Null),
        ("profile", json!("untrusted")),
        ("action", json!({"kind":"direct"})),
    ] {
        let mut data = fixture();
        data[field] = value;
        invalid.push(serde_json::to_vec(&data).unwrap());
    }
    for (field, value) in [
        ("mode", json!("wildcard")),
        ("mode", Value::Null),
        ("domain", json!("*.bad.example")),
        ("domain", json!("xn--invalid-")),
        ("domain", json!(1)),
    ] {
        let mut data = fixture();
        data["entries"][0][field] = value;
        invalid.push(serde_json::to_vec(&data).unwrap());
    }
    for (field, value) in [
        ("kind", json!("node")),
        ("revision", json!("not-an-id")),
        ("hash", json!("00")),
        ("hash", json!("zz".repeat(32))),
    ] {
        let mut data = fixture();
        data["dependencies"][0][field] = value;
        invalid.push(serde_json::to_vec(&data).unwrap());
    }
    let valid_text = std::str::from_utf8(SERVICE).unwrap().trim_end();
    invalid.push(
        format!(
            "{},\"schemaVersion\":1}}",
            &valid_text[..valid_text.len() - 1]
        )
        .into_bytes(),
    );
    invalid.push(
        valid_text
            .replacen(
                "\"domain\": \"video.example\"",
                "\"domain\": \"video.example\", \"domain\": \"other.example\"",
                1,
            )
            .into_bytes(),
    );
    let data = fixture();
    invalid.push(
        serde_json::to_vec(&json!([
            1,
            "service",
            data["entries"],
            data["dependencies"]
        ]))
        .unwrap(),
    );
    let mut positional = fixture();
    positional["entries"][0] = json!(["video.example", "exact"]);
    invalid.push(serde_json::to_vec(&positional).unwrap());
    invalid.extend([
        b"[]".to_vec(),
        b"null".to_vec(),
        b"{} {}".to_vec(),
        b"{\"schemaVersion\":1,\"schemaVersion\":1}".to_vec(),
        b"\xff".to_vec(),
    ]);
    let mut nested = b"[".repeat(1000);
    nested.extend(b"]".repeat(1000));
    invalid.push(nested);
    for bytes in invalid {
        let mut store = base();
        let error = store
            .import_batch(
                vec![input(service(301), &bytes)],
                vec![approval(service(301), &bytes)],
            )
            .unwrap_err();
        assert!(store.get(service(301)).is_none());
        assert_eq!(store.revisions().count(), 1);
        assert!(!format!("{error:?} {error}").contains("bad.example"));
    }
    let mut with_bom = b"\xef\xbb\xbf".to_vec();
    with_bom.extend_from_slice(SERVICE);
    let mut store = base();
    store
        .import_batch(
            vec![input(service(301), &with_bom)],
            vec![approval(service(301), &with_bom)],
        )
        .unwrap();
}

#[test]
fn json_raw_entries_share_the_batch_budget_before_normalization_or_deduplication() {
    let mut store = CatalogStore::default();
    let entry = json!({"domain":"same.example","mode":"exact"});
    let mut bundle = json!({"schemaVersion":1,"kind":"service","entries":vec![entry;MAX_POLICY_ENTRIES-1],"dependencies":[]});
    let bytes = serde_json::to_vec(&bundle).unwrap();
    let summary = store
        .import_batch(
            vec![input(service(1), &bytes)],
            vec![approval(service(1), &bytes)],
        )
        .unwrap();
    assert_eq!(
        (summary.raw_entries, summary.canonical_entries),
        (MAX_POLICY_ENTRIES - 1, 1)
    );
    bundle["entries"]
        .as_array_mut()
        .unwrap()
        .push(json!({"domain":"extra.example","mode":"exact"}));
    bundle["entries"][0]["domain"] = "broken..example".into();
    let oversized = serde_json::to_vec(&bundle).unwrap();
    assert!(matches!(
        store.import_batch(
            vec![input(service(2), &oversized)],
            vec![approval(service(2), &oversized)]
        ),
        Err(Error::Document { .. })
    ));
    assert_eq!(store.revisions().count(), 1);
}

#[test]
fn preflight_charges_raw_duplicates_revision_count_and_combined_metadata_bytes() {
    let mut store = CatalogStore::default();
    let exact = "same.example\n".repeat(MAX_POLICY_ENTRIES - 1);
    store
        .import_batch(
            vec![input(filter(1), exact.as_bytes())],
            vec![approval(filter(1), exact.as_bytes())],
        )
        .unwrap();
    let too_many = exact + "same.example\n";
    assert!(
        store
            .import_batch(
                vec![input(filter(2), too_many.as_bytes())],
                vec![approval(filter(2), too_many.as_bytes())]
            )
            .is_err()
    );
    let m = metadata();
    let overhead = m.source.len()
        + m.owner.len()
        + m.license.len()
        + m.notice.len()
        + "Apache-2.0".len()
        + "owner:test-fixture-approval".len();
    let mut bytes = b"exact.example\n#".to_vec();
    bytes.resize(MAX_LOCAL_INPUT_BYTES - overhead, b'x');
    store
        .import_batch(
            vec![input(filter(3), &bytes)],
            vec![approval(filter(3), &bytes)],
        )
        .unwrap();
    bytes.push(b'x');
    assert_eq!(
        store.import_batch(
            vec![input(filter(4), &bytes)],
            vec![approval(filter(4), &bytes)]
        ),
        Err(Error::Size)
    );
    // 65 full-size inputs share one borrowed buffer: byte preflight must refuse
    // before allocating 65 owned copies or normalizing any domains.
    bytes.pop();
    let inputs = (100..165).map(|n| input(filter(n), &bytes)).collect();
    let approvals = (100..165).map(|n| approval(filter(n), &bytes)).collect();
    assert_eq!(store.import_batch(inputs, approvals), Err(Error::Size));
    assert_eq!(store.revisions().count(), 2);
}

#[test]
fn retained_revisions_share_the_existing_entry_budget_and_diagnostics_have_typed_sources() {
    let mut store = CatalogStore::default();
    for n in [1, 2] {
        let data = (0..4999)
            .map(|i| format!("n{n}-{i}.example\n"))
            .collect::<String>();
        store
            .import_batch(
                vec![input(filter(n), data.as_bytes())],
                vec![approval(filter(n), data.as_bytes())],
            )
            .unwrap();
    }
    assert_eq!(store.revisions().count(), 2);
    assert_eq!(
        store.import_batch(
            vec![input(filter(3), b"one.example")],
            vec![approval(filter(3), b"one.example")]
        ),
        Err(Error::Size)
    );
    let error = store
        .import_batch(
            vec![input(filter(4), b"broken..example")],
            vec![approval(filter(4), b"broken..example")],
        )
        .unwrap_err();
    assert!(error.source().is_some());
    assert!(Error::License.source().is_none());
}

#[test]
fn service_errors_keep_sanitized_entry_and_dependency_indices() {
    for (index, field, value) in [
        (1, "domain", json!("confidential..example")),
        (2, "mode", json!("unsupported")),
        (0, "domain", json!(7)),
    ] {
        let mut payload = fixture();
        payload["entries"][index][field] = value;
        let bytes = serde_json::to_vec(&payload).unwrap();
        let mut store = base();
        let error = store
            .import_batch(
                vec![input(service(301), &bytes)],
                vec![approval(service(301), &bytes)],
            )
            .unwrap_err();
        assert_eq!(error, Error::ServiceEntry { index });
        assert!(!format!("{error:?} {error}").contains("confidential"));
        assert!(error.source().is_none());
    }
    for (field, value) in [
        ("hash", json!("bad")),
        ("revision", json!("bad")),
        ("kind", json!("node")),
    ] {
        let mut payload = fixture();
        payload["dependencies"][0][field] = value;
        let bytes = serde_json::to_vec(&payload).unwrap();
        let mut store = base();
        assert_eq!(
            store.import_batch(
                vec![input(service(301), &bytes)],
                vec![approval(service(301), &bytes)]
            ),
            Err(Error::ServiceDependency { index: 0 })
        );
    }
}
