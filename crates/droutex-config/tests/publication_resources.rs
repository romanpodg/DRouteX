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
