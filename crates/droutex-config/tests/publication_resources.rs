use droutex_config::{
    InputFormat, PublicationResourceError as Error, ReferenceCatalog, parse_configuration,
};
use droutex_policy::{
    CatalogIssue, CatalogLicense, CatalogPin, CatalogProvenance, CatalogRevision,
    CatalogRevisionFacts, CatalogValidationError, CatalogValidationState, PrivateRecordIssue,
    PrivateRecordValidationError,
};
use serde_json::json;
use std::str::FromStr;

fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn text(n: usize) -> String {
    id::<droutex_common::ProfileId>(n).to_string()
}

#[test]
fn existence_only_catalogs_and_old_reference_checks_are_not_publication_validation() {
    let input = json!({"schemaVersion":1,"profiles":[{"id":text(1),"name":"p",
        "defaultDnsAction":{"kind":"resolve","upstream":text(90)},"defaultTrafficAction":{"kind":"deny"},
        "serviceRules":[{"id":text(80),"enabled":false,"priority":0,"revision":text(70),"action":{"plane":"dns","action":{"kind":"block"}}}]}]});
    let refs = ReferenceCatalog {
        upstreams: [id(90)].into(),
        service_revisions: [id(70)].into(),
        ..ReferenceCatalog::default()
    };
    let checked = parse_configuration(
        &serde_json::to_vec(&input).unwrap(),
        InputFormat::Json,
        &refs,
    )
    .unwrap();
    let source = serde_json::to_vec(checked.document()).unwrap();
    let revision = CatalogRevision::Service(id(70));
    let pin = CatalogPin {
        revision,
        hash: [7; 32],
    };
    assert_eq!(
        checked.validate_publication_resources(&refs, &[], &[]),
        Err(Error::MissingCatalogPin(revision))
    );
    let mut fact = CatalogRevisionFacts {
        revision,
        actual_hash: [7; 32],
        license: CatalogLicense::Unapproved,
        provenance: CatalogProvenance::Recorded,
        validation: CatalogValidationState::Valid,
        dependencies: vec![],
    };
    assert_eq!(
        checked.validate_publication_resources(&refs, &[pin], &[]),
        Err(Error::Catalog(CatalogValidationError::Revision {
            issue: CatalogIssue::Missing,
            revision
        }))
    );
    assert_eq!(
        checked.validate_publication_resources(&refs, &[pin], std::slice::from_ref(&fact)),
        Err(Error::Catalog(CatalogValidationError::Revision {
            issue: CatalogIssue::License,
            revision
        }))
    );
    fact.license = CatalogLicense::Approved;
    assert!(
        checked
            .validate_publication_resources(&refs, &[pin], std::slice::from_ref(&fact))
            .is_ok()
    );
    let removed = ReferenceCatalog {
        service_revisions: [id(70)].into(),
        ..ReferenceCatalog::default()
    };
    assert!(matches!(
        checked.validate_publication_resources(&removed, &[pin], &[fact]),
        Err(Error::Configuration(_))
    ));
    assert_eq!(serde_json::to_vec(checked.document()).unwrap(), source);
}

#[test]
fn canonical_alias_cycle_passes_old_shape_checks_but_cannot_pass_publication_resources() {
    let mut input = json!({"schemaVersion":1,"profiles":[{"id":text(1),"name":"p","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"deny"}}],
        "privateRecords":[{"id":text(10),"profile":text(1),"name":"BÜCHER.example.","ttl":300,"record":{"kind":"cname","target":"other.example"}},
        {"id":text(11),"profile":text(1),"name":"other.example","ttl":300,"record":{"kind":"cname","target":"xn--bcher-kva.example"}}]});
    let refs = ReferenceCatalog::default();
    let checked = parse_configuration(
        &serde_json::to_vec(&input).unwrap(),
        InputFormat::Json,
        &refs,
    )
    .unwrap();
    assert!(matches!(
        checked.validate_publication_resources(&refs, &[], &[]),
        Err(Error::PrivateRecord(PrivateRecordValidationError::Record {
            issue: PrivateRecordIssue::Cycle,
            ..
        }))
    ));
    input["privateRecords"][1]["record"]["target"] = json!("external.example");
    let checked = parse_configuration(
        &serde_json::to_vec(&input).unwrap(),
        InputFormat::Json,
        &refs,
    )
    .unwrap();
    assert!(
        checked
            .validate_publication_resources(&refs, &[], &[])
            .is_ok()
    );
}

#[test]
fn full_configuration_requires_filter_and_gateway_pins_and_checks_both_address_families() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/configuration-v1.json")).unwrap();
    input["profiles"][0]["filterLists"][0]["enabled"] = json!(false);
    input["gatewayGrants"][0]["services"] = json!([text(102)]);
    let refs = ReferenceCatalog {
        devices: [id(120)].into(),
        upstreams: [id(90)].into(),
        secrets: [id(99)].into(),
        filter_revisions: [id(101)].into(),
        service_revisions: [id(100), id(102)].into(),
    };
    let revisions = [
        CatalogRevision::Filter(id(101)),
        CatalogRevision::Service(id(100)),
        CatalogRevision::Service(id(102)),
    ];
    let pins: Vec<_> = revisions
        .iter()
        .map(|&revision| CatalogPin {
            revision,
            hash: [7; 32],
        })
        .collect();
    let facts: Vec<_> = revisions
        .iter()
        .map(|&revision| CatalogRevisionFacts {
            revision,
            actual_hash: [7; 32],
            license: CatalogLicense::Approved,
            provenance: CatalogProvenance::Recorded,
            validation: CatalogValidationState::Valid,
            dependencies: vec![],
        })
        .collect();
    let checked = parse_configuration(
        &serde_json::to_vec(&input).unwrap(),
        InputFormat::Json,
        &refs,
    )
    .unwrap();
    let before = serde_json::to_vec(checked.document()).unwrap();
    assert!(
        checked
            .validate_publication_resources(&refs, &pins, &facts)
            .is_ok()
    );
    for missing in revisions {
        let incomplete: Vec<_> = pins
            .iter()
            .copied()
            .filter(|p| p.revision != missing)
            .collect();
        let error = checked
            .validate_publication_resources(&refs, &incomplete, &facts)
            .unwrap_err();
        assert_eq!(error, Error::MissingCatalogPin(missing));
        assert!(!format!("{error} {error:?}").contains("internal"));
    }
    assert_eq!(serde_json::to_vec(checked.document()).unwrap(), before);
    input["privateRecords"].as_array_mut().unwrap().push(json!({
        "id":text(114),"profile":text(1),"name":"IPV6.internal.","ttl":61,
        "record":{"kind":"aaaa","address":"fd00::2"}
    }));
    let checked = parse_configuration(
        &serde_json::to_vec(&input).unwrap(),
        InputFormat::Json,
        &refs,
    )
    .unwrap();
    assert!(matches!(
        checked.validate_publication_resources(&refs, &pins, &facts),
        Err(Error::PrivateRecord(PrivateRecordValidationError::Record {
            issue: PrivateRecordIssue::Ttl,
            ..
        }))
    ));
}
