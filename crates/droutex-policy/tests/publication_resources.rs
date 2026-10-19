use droutex_policy::{
    CatalogIssue as CI, CatalogLicense as License, CatalogPin, CatalogProvenance as Provenance,
    CatalogRevision as Revision, CatalogRevisionFacts as Facts, CatalogValidationError as CE,
    CatalogValidationState as State, PreparedPrivateRecord as Record, PrivateRecordIssue as RI,
    PrivateRecordValidationError as RE, PrivateRecordValue as Value,
    validate_catalog_dependencies as catalogs, validate_private_records as records,
};
use std::{error::Error, str::FromStr};

fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn pin(n: usize) -> CatalogPin {
    CatalogPin {
        revision: Revision::Service(id(n)),
        hash: [7; 32],
    }
}
fn fact(n: usize, dependencies: Vec<CatalogPin>) -> Facts {
    Facts {
        revision: pin(n).revision,
        actual_hash: [7; 32],
        license: License::Approved,
        provenance: Provenance::Recorded,
        validation: State::Valid,
        dependencies,
    }
}
fn cat_error(issue: CI, n: usize) -> CE {
    CE::Revision {
        issue,
        revision: pin(n).revision,
    }
}
fn record(n: usize, profile: usize, owner: &str, ttl: u32, value: Value) -> Record {
    Record {
        id: id(n),
        profile: id(profile),
        owner: owner.parse().unwrap(),
        ttl,
        value,
    }
}
fn alias(n: usize, p: usize, name: &str, target: &str) -> Record {
    record(n, p, name, 300, Value::Cname(target.parse().unwrap()))
}
fn a(n: usize, p: usize, name: &str, ttl: u32) -> Record {
    record(n, p, name, ttl, Value::Ipv4("192.0.2.1".parse().unwrap()))
}

#[test]
fn selected_catalogs_require_actual_hash_license_provenance_and_validation() {
    let root = pin(1);
    assert_eq!(catalogs(&[root], &[]), Err(cat_error(CI::Missing, 1)));
    for change in 0..6 {
        let mut f = fact(1, vec![]);
        let issue = match change {
            0 => {
                f.actual_hash = [8; 32];
                CI::HashMismatch
            }
            1 => {
                f.license = License::Unapproved;
                CI::License
            }
            2 => {
                f.license = License::Unavailable;
                CI::License
            }
            3 => {
                f.provenance = Provenance::Unavailable;
                CI::Provenance
            }
            4 => {
                f.validation = State::Invalid;
                CI::Invalid
            }
            _ => {
                f.validation = State::Unavailable;
                CI::Invalid
            }
        };
        assert_eq!(catalogs(&[root], &[f]), Err(cat_error(issue, 1)));
    }
    assert!(catalogs(&[root, root], &[fact(1, vec![])]).is_ok());
    assert_eq!(
        catalogs(&[root], &[fact(1, vec![]), fact(1, vec![])]),
        Err(cat_error(CI::DuplicateRevision, 1))
    );
    // An invalid unselected revision grants nothing and cannot disable another
    // selection. Kind is part of identity even when the UUID bytes are equal.
    let mut unrelated = fact(2, vec![]);
    unrelated.license = License::Unapproved;
    assert!(catalogs(&[root], &[unrelated, fact(1, vec![])]).is_ok());
    assert_eq!(
        catalogs(
            &[CatalogPin {
                revision: Revision::Filter(id(1)),
                hash: [7; 32]
            }],
            &[fact(1, vec![])]
        ),
        Err(CE::Revision {
            issue: CI::Missing,
            revision: Revision::Filter(id(1))
        })
    );
}

#[test]
fn dependencies_check_every_pin_and_reordering_never_hides_a_bad_reference() {
    for reverse in [false, true] {
        let mut inventory = vec![
            fact(1, vec![pin(2), pin(3)]),
            fact(2, vec![pin(3)]),
            fact(3, vec![]),
        ];
        if reverse {
            inventory.reverse();
            for f in &mut inventory {
                f.dependencies.reverse()
            }
        }
        assert!(catalogs(&[pin(1)], &inventory).is_ok());
        inventory.retain(|f| f.revision != pin(3).revision);
        assert_eq!(
            catalogs(&[pin(1)], &inventory),
            Err(cat_error(CI::Missing, 3))
        );
    }
    let bad = CatalogPin {
        hash: [9; 32],
        ..pin(3)
    };
    let inventory = [
        fact(1, vec![pin(2), pin(3)]),
        fact(2, vec![bad]),
        fact(3, vec![]),
    ];
    assert_eq!(
        catalogs(&[pin(1)], &inventory),
        Err(cat_error(CI::HashMismatch, 3))
    );
    let mut inventory = vec![
        fact(1, vec![pin(2)]),
        fact(2, vec![pin(3)]),
        fact(3, vec![pin(2)]),
    ];
    let error = catalogs(&[pin(1)], &inventory).unwrap_err();
    assert!(matches!(error, CE::Cycle { .. }));
    inventory.reverse();
    assert_eq!(catalogs(&[pin(1)], &inventory), Err(error));
    assert!(matches!(
        catalogs(&[pin(1)], &[fact(1, vec![pin(1)])]),
        Err(CE::Cycle { .. })
    ));
}

#[test]
fn catalog_graph_and_inputs_share_the_existing_bound_and_deep_chains_are_iterative() {
    // 5000 facts +4999 dependency pins +1 root =10000 entries, no new limit.
    let chain: Vec<_> = (1..=5000)
        .map(|n| fact(n, if n < 5000 { vec![pin(n + 1)] } else { vec![] }))
        .collect();
    assert!(catalogs(&[pin(1)], &chain).is_ok());
    assert_eq!(catalogs(&[pin(1), pin(1)], &chain), Err(CE::EntryLimit));
    let cycle: Vec<_> = (1..5000)
        .map(|n| fact(n, vec![pin(if n < 4999 { n + 1 } else { 1 })]))
        .collect();
    assert!(matches!(catalogs(&[pin(1)], &cycle), Err(CE::Cycle { .. })));
}

#[test]
fn canonical_rrsets_allow_multiple_addresses_equivalents_and_independent_ttls() {
    let mut input = vec![
        a(10, 1, "BÜCHER.Example.", 0),
        a(11, 1, "xn--bcher-kva.example", 0),
        record(
            12,
            1,
            "bücher.example",
            u32::MAX,
            Value::Ipv6("2001:db8::1".parse().unwrap()),
        ),
        record(
            13,
            1,
            "bücher.example",
            0,
            Value::Ipv4("192.0.2.2".parse().unwrap()),
        ),
        alias(14, 1, "alias.example", "BÜCHER.example."),
        alias(15, 1, "ALIAS.example.", "xn--bcher-kva.example"),
    ];
    assert!(records(&input).is_ok());
    input.reverse();
    assert!(records(&input).is_ok());
    input.push(a(16, 1, "bücher.example", 1));
    assert!(matches!(
        records(&input),
        Err(RE::Record { issue: RI::Ttl, .. })
    ));
    assert!(records(&[a(10, 1, "same.example", 10), a(11, 2, "same.example", 20)]).is_ok());
}

#[test]
fn private_alias_conflicts_cycles_and_ids_are_stable_and_do_not_echo_names() {
    for input in [
        vec![
            alias(10, 1, "name.example", "target.example"),
            a(11, 1, "NAME.example.", 300),
        ],
        vec![
            a(10, 1, "name.example", 300),
            alias(11, 1, "name.example", "target.example"),
        ],
    ] {
        assert!(matches!(
            records(&input),
            Err(RE::Record {
                issue: RI::CnameData,
                ..
            })
        ));
    }
    assert!(matches!(
        records(&[
            alias(10, 1, "name.example", "one.example"),
            alias(11, 1, "NAME.example", "two.example")
        ]),
        Err(RE::Record {
            issue: RI::CnameTarget,
            ..
        })
    ));
    assert!(matches!(
        records(&[a(10, 1, "one.example", 300), a(10, 2, "two.example", 300)]),
        Err(RE::Record {
            issue: RI::DuplicateId,
            ..
        })
    ));
    let mut cycle = vec![
        alias(10, 1, "BÜCHER.example", "other.example"),
        alias(11, 1, "other.example", "xn--bcher-kva.example"),
    ];
    let error = records(&cycle).unwrap_err();
    assert!(matches!(error,RE::Record {issue:RI::Cycle,profile,..} if profile==id(1)));
    cycle.reverse();
    assert_eq!(records(&cycle), Err(error));
    assert!(!format!("{error} {error:?}").contains("example"));
    assert!(error.source().is_none());
    assert!(matches!(
        records(&[alias(10, 1, "SELF.example.", "self.example")]),
        Err(RE::Record {
            issue: RI::Cycle,
            ..
        })
    ));
}

#[test]
fn absent_targets_are_external_and_other_profiles_cannot_create_or_resolve_a_cycle() {
    assert!(
        records(&[
            alias(10, 1, "one.example", "two.example"),
            alias(11, 2, "two.example", "one.example")
        ])
        .is_ok()
    );
    assert!(records(&[alias(10, 1, "one.example", "absent.example")]).is_ok());
    let mut long: Vec<_> = (0..10000)
        .map(|n| {
            if n == 9999 {
                a(100 + n, 1, &format!("n{n}.example"), 300)
            } else {
                alias(
                    100 + n,
                    1,
                    &format!("n{n}.example"),
                    &format!("n{}.example", n + 1),
                )
            }
        })
        .collect();
    assert!(records(&long).is_ok());
    long[9999] = alias(10099, 1, "n9999.example", "n0.example");
    assert!(matches!(
        records(&long),
        Err(RE::Record {
            issue: RI::Cycle,
            ..
        })
    ));
    long.push(a(20000, 1, "extra.example", 300));
    assert_eq!(records(&long), Err(RE::EntryLimit));
}
