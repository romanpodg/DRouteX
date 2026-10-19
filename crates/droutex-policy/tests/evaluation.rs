use droutex_common::{
    AdministrativeState as State, DnsEndpointAssignment, DnsProfileBinding, GatewayNetworkIdentity,
};
use droutex_policy::{
    CanonicalDomain, DomainMatchMode as Mode, DomainMatcher, DomainRule, EvaluationError as Eval,
    GatewayProfileAssignment, MAX_POLICY_ENTRIES, PolicyBuildError as Build, PolicyScope,
    PolicySet, PreparedProfile, RuleMetadata, RuleSource as Source, TcpPortRange, TcpPortScope,
    TcpRule,
};
use std::{error::Error, num::NonZeroU16, str::FromStr};

fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn custom(n: usize) -> Source {
    Source::Custom(id(n))
}
fn service(n: usize, revision: usize) -> Source {
    Source::Service {
        rule: id(n),
        revision: id(revision),
    }
}
fn rule(source: Source, priority: i32, mode: Mode, domain: &str, action: u8) -> DomainRule<u8> {
    DomainRule::new(
        RuleMetadata {
            source,
            state: State::Enabled,
            priority,
        },
        DomainMatcher::new(domain.parse().unwrap(), mode),
        action,
    )
}
fn profile(n: usize, dns: Vec<DomainRule<u8>>, tcp: Vec<TcpRule<u8>>) -> PreparedProfile<u8, u8> {
    PreparedProfile {
        id: id(n),
        dns_default: 0,
        dns_rules: dns,
        tcp_default: 0,
        tcp_rules: tcp,
    }
}
fn endpoint(n: usize, p: usize) -> DnsEndpointAssignment {
    DnsEndpointAssignment {
        endpoint: id(n),
        profile: id(p),
        state: State::Enabled,
        device: None,
    }
}
fn binding() -> DnsProfileBinding {
    DnsProfileBinding {
        endpoint: id(10),
        profile: id(1),
    }
}
fn identity(group: usize) -> GatewayNetworkIdentity {
    GatewayNetworkIdentity {
        grant: id(30),
        group: id(group),
        listener: id(40),
        source: "192.0.2.1".parse().unwrap(),
    }
}
fn policy(dns: Vec<DomainRule<u8>>, tcp: Vec<TcpRule<u8>>) -> PolicySet<u8, u8> {
    PolicySet::new(
        vec![profile(1, dns, tcp)],
        vec![endpoint(10, 1)],
        vec![GatewayProfileAssignment {
            group: id(20),
            profile: id(1),
        }],
    )
    .unwrap()
}
fn query() -> CanonicalDomain {
    "child.example.com".parse().unwrap()
}
fn port(n: u16) -> NonZeroU16 {
    NonZeroU16::new(n).unwrap()
}
fn range(first: u16, last: u16) -> TcpPortScope {
    TcpPortScope::Range(TcpPortRange::new(first, last).unwrap())
}
fn selected(p: &PolicySet<u8, u8>) -> (u8, Option<Source>) {
    let result = p.evaluate_dns(&binding(), &query()).unwrap();
    (*result.action(), result.source())
}
fn each_order(n: usize, mut visit: impl FnMut(&[usize])) {
    fn walk(at: usize, order: &mut [usize], visit: &mut dyn FnMut(&[usize])) {
        if at == order.len() {
            visit(order);
            return;
        }
        for i in at..order.len() {
            order.swap(at, i);
            walk(at + 1, order, visit);
            order.swap(at, i);
        }
    }
    walk(0, &mut (0..n).collect::<Vec<_>>(), &mut visit);
}

#[test]
fn fixed_layers_override_extreme_priorities_independently_of_input_order() {
    each_order(4, |order| {
        let rules = order
            .iter()
            .map(|index| match index {
                0 => rule(
                    custom(50),
                    i32::MIN,
                    Mode::DomainAndSubdomains,
                    "example.com",
                    1,
                ),
                1 => rule(
                    service(51, 100),
                    i32::MAX,
                    Mode::Exact,
                    "child.example.com",
                    2,
                ),
                2 => rule(
                    Source::Filter(id(101)),
                    i32::MAX,
                    Mode::Exact,
                    "child.example.com",
                    3,
                ),
                _ => rule(custom(52), i32::MAX, Mode::Exact, "unrelated.example", 4),
            })
            .collect();
        assert_eq!(selected(&policy(rules, vec![])), (1, Some(custom(50))));
    });
    assert_eq!(
        selected(&policy(
            vec![
                rule(
                    Source::Filter(id(101)),
                    i32::MAX,
                    Mode::Exact,
                    "child.example.com",
                    3
                ),
                rule(
                    service(51, 100),
                    i32::MIN,
                    Mode::DomainAndSubdomains,
                    "example.com",
                    2
                )
            ],
            vec![]
        )),
        (2, Some(service(51, 100)))
    );
}

#[test]
fn priority_then_label_count_then_explicit_match_mode_selects() {
    let cases = [
        (
            100,
            Mode::DomainAndSubdomains,
            "example.com",
            50,
            Mode::Exact,
            "child.example.com",
            1,
        ),
        (
            0,
            Mode::DomainAndSubdomains,
            "example.com",
            0,
            Mode::DomainAndSubdomains,
            "child.example.com",
            2,
        ),
        (
            0,
            Mode::DomainAndSubdomains,
            "example.com",
            0,
            Mode::SubdomainsOnly,
            "example.com",
            2,
        ),
        (
            0,
            Mode::DomainAndSubdomains,
            "child.example.com",
            0,
            Mode::Exact,
            "child.example.com",
            2,
        ),
        (
            i32::MIN,
            Mode::Exact,
            "child.example.com",
            i32::MAX,
            Mode::DomainAndSubdomains,
            "example.com",
            2,
        ),
    ];
    for (p1, m1, d1, p2, m2, d2, expected) in cases {
        each_order(2, |order| {
            let rules = order
                .iter()
                .map(|i| {
                    if *i == 0 {
                        rule(custom(60), p1, m1, d1, 1)
                    } else {
                        rule(custom(61), p2, m2, d2, 2)
                    }
                })
                .collect();
            assert_eq!(selected(&policy(rules, vec![])).0, expected);
        });
    }
}

#[test]
fn uuid_order_breaks_only_equivalent_effect_ties_and_preserves_revision_provenance() {
    each_order(3, |order| {
        let rules = order
            .iter()
            .map(|i| {
                rule(
                    service(70, 103 - *i),
                    0,
                    Mode::Exact,
                    "child.example.com",
                    8,
                )
            })
            .collect();
        assert_eq!(
            selected(&policy(rules, vec![])),
            (8, Some(service(70, 101)))
        );
    });
    each_order(3, |order| {
        let rules = order
            .iter()
            .map(|i| rule(custom(80 - *i), 0, Mode::Exact, "child.example.com", 8))
            .collect();
        assert_eq!(selected(&policy(rules, vec![])), (8, Some(custom(78))));
    });
}

#[test]
fn contradictory_top_effects_refuse_with_stable_source_ids_in_every_permutation() {
    each_order(4, |order| {
        let rules = order
            .iter()
            .map(|i| match i {
                0 => rule(custom(80), 0, Mode::Exact, "child.example.com", 1),
                1 => rule(custom(81), 0, Mode::Exact, "child.example.com", 2),
                2 => rule(custom(82), 0, Mode::Exact, "child.example.com", 3),
                _ => rule(custom(79), -1, Mode::Exact, "child.example.com", 4),
            })
            .collect();
        assert_eq!(
            policy(rules, vec![])
                .evaluate_dns(&binding(), &query())
                .unwrap_err(),
            Eval::Ambiguous {
                first: custom(80),
                second: custom(81)
            }
        );
    });
    // Even duplicate source identity may not conceal contradictory effects.
    let p = policy(
        vec![
            rule(custom(80), 0, Mode::Exact, "child.example.com", 1),
            rule(custom(80), 0, Mode::Exact, "child.example.com", 2),
        ],
        vec![],
    );
    assert_eq!(
        p.evaluate_dns(&binding(), &query()).unwrap_err(),
        Eval::Ambiguous {
            first: custom(80),
            second: custom(80)
        }
    );
}

#[test]
fn lower_key_conflicts_do_not_hide_a_later_unambiguous_winner() {
    each_order(3, |order| {
        let rules = order
            .iter()
            .map(|i| {
                rule(
                    custom(90 + *i),
                    if *i == 2 { 1 } else { 0 },
                    Mode::Exact,
                    "child.example.com",
                    *i as u8,
                )
            })
            .collect();
        assert_eq!(selected(&policy(rules, vec![])), (2, Some(custom(92))));
    });
}

#[test]
fn disabled_and_nonmatching_rules_do_not_replace_the_profile_fallback() {
    let disabled = DomainRule::new(
        RuleMetadata {
            source: custom(91),
            state: State::Disabled,
            priority: i32::MAX,
        },
        DomainMatcher::new("child.example.com".parse().unwrap(), Mode::Exact),
        9,
    );
    let p = policy(
        vec![
            disabled,
            rule(custom(92), 0, Mode::Exact, "badchild.example.com", 9),
        ],
        vec![],
    );
    let result = p.evaluate_dns(&binding(), &query()).unwrap();
    assert_eq!((*result.action(), result.source()), (0, None));
    assert_eq!(result.profile(), id(1));
    assert_eq!(result.scope(), PolicyScope::DnsEndpoint(id(10)));
}

#[test]
fn canonical_ace_queries_and_arbitrary_descendant_depth_use_d37_semantics() {
    let p = policy(
        vec![rule(
            custom(1),
            0,
            Mode::SubdomainsOnly,
            "BÜCHER.Example.",
            7,
        )],
        vec![],
    );
    for depth in 0..16 {
        let query = format!("{}XN--BCHER-KVA.EXAMPLE.", "NODE.".repeat(depth))
            .parse()
            .unwrap();
        assert_eq!(
            *p.evaluate_dns(&binding(), &query).unwrap().action(),
            if depth == 0 { 0 } else { 7 }
        );
    }
    for name in ["badxn--bcher-kva.example", "xn--bcher-kva.example.attacker"] {
        assert_eq!(
            *p.evaluate_dns(&binding(), &name.parse().unwrap())
                .unwrap()
                .action(),
            0
        );
    }
}

#[test]
fn tcp_ports_gate_eligibility_without_changing_priority_or_guessing_metadata() {
    for (first, last) in [(0, 1), (1, 0), (443, 80)] {
        assert_eq!(TcpPortRange::new(first, last), Err(Build::InvalidPortRange));
    }
    assert!(NonZeroU16::new(0).is_none());
    let p = policy(
        vec![],
        vec![
            TcpRule::new(
                rule(custom(1), 1, Mode::DomainAndSubdomains, "example.com", 1),
                range(80, 81),
            ),
            TcpRule::new(
                rule(custom(1), 1, Mode::DomainAndSubdomains, "example.com", 1),
                range(443, 443),
            ),
            TcpRule::new(
                rule(custom(2), 0, Mode::Exact, "child.example.com", 2),
                TcpPortScope::Any,
            ),
        ],
    );
    for (number, expected) in [
        (1, 2),
        (79, 2),
        (80, 1),
        (81, 1),
        (82, 2),
        (443, 1),
        (444, 2),
        (65535, 2),
    ] {
        assert_eq!(
            *p.evaluate_tcp(&identity(20), &query(), port(number))
                .unwrap()
                .action(),
            expected
        );
    }
    let p = policy(
        vec![],
        vec![TcpRule::new(
            rule(custom(1), 0, Mode::Exact, "child.example.com", 1),
            range(1, 65535),
        )],
    );
    for number in [1, 65535] {
        assert_eq!(
            *p.evaluate_tcp(&identity(20), &query(), port(number))
                .unwrap()
                .action(),
            1
        );
    }
    assert_eq!(
        *p.evaluate_tcp(
            &identity(20),
            &"unrelated.example".parse().unwrap(),
            port(443)
        )
        .unwrap()
        .action(),
        0
    );
}

#[test]
fn overlapping_port_ranges_do_not_invent_a_narrowest_range_tiebreak() {
    each_order(2, |order| {
        let rules = order
            .iter()
            .map(|i| {
                TcpRule::new(
                    rule(
                        custom(100 + *i),
                        0,
                        Mode::Exact,
                        "child.example.com",
                        *i as u8,
                    ),
                    if *i == 0 {
                        range(1, 65535)
                    } else {
                        range(443, 443)
                    },
                )
            })
            .collect();
        let p = policy(vec![], rules);
        assert_eq!(
            p.evaluate_tcp(&identity(20), &query(), port(443))
                .unwrap_err(),
            Eval::Ambiguous {
                first: custom(100),
                second: custom(101)
            }
        );
        assert_eq!(
            *p.evaluate_tcp(&identity(20), &query(), port(80))
                .unwrap()
                .action(),
            0
        );
    });
}

#[test]
fn endpoint_and_gateway_profile_scopes_remain_independent_of_dns_history() {
    let mut a = profile(1, vec![], vec![]);
    a.dns_default = 1;
    a.tcp_default = 2;
    let mut b = profile(2, vec![], vec![]);
    b.dns_default = 3;
    b.tcp_default = 4;
    let p = PolicySet::new(
        vec![b, a],
        vec![endpoint(10, 1), endpoint(11, 2)],
        vec![
            GatewayProfileAssignment {
                group: id(20),
                profile: id(1),
            },
            GatewayProfileAssignment {
                group: id(21),
                profile: id(2),
            },
        ],
    )
    .unwrap();
    for ep in [10, 11, 10, 11] {
        let binding = DnsProfileBinding {
            endpoint: id(ep),
            profile: id(if ep == 10 { 1 } else { 2 }),
        };
        assert_eq!(
            *p.evaluate_dns(&binding, &query()).unwrap().action(),
            if ep == 10 { 1 } else { 3 }
        );
        let tcp = p.evaluate_tcp(&identity(20), &query(), port(443)).unwrap();
        assert_eq!(
            (*tcp.action(), tcp.profile(), tcp.scope()),
            (2, id(1), PolicyScope::GatewayGroup(id(20)))
        );
    }
    assert_eq!(
        *p.evaluate_tcp(&identity(21), &query(), port(443))
            .unwrap()
            .action(),
        4
    );
    assert_eq!(
        p.evaluate_dns(
            &DnsProfileBinding {
                endpoint: id(10),
                profile: id(2)
            },
            &query()
        )
        .unwrap_err(),
        Eval::EndpointBindingMismatch
    );
    assert_eq!(
        p.evaluate_dns(
            &DnsProfileBinding {
                endpoint: id(99),
                profile: id(1)
            },
            &query()
        )
        .unwrap_err(),
        Eval::UnknownEndpoint
    );
    assert_eq!(
        p.evaluate_tcp(&identity(99), &query(), port(443))
            .unwrap_err(),
        Eval::UnknownGatewayGroup
    );
    let mut disabled = endpoint(10, 1);
    disabled.state = State::Disabled;
    let p = PolicySet::new(vec![profile(1, vec![], vec![])], vec![disabled], vec![]).unwrap();
    assert_eq!(
        p.evaluate_dns(&binding(), &query()).unwrap_err(),
        Eval::UnavailableEndpoint
    );
}

#[test]
fn duplicate_or_dangling_associations_are_never_merged_or_guessed() {
    assert_eq!(
        PolicySet::new(
            vec![profile(1, vec![], vec![]), profile(1, vec![], vec![])],
            vec![],
            vec![]
        )
        .unwrap_err(),
        Build::DuplicateProfile(id(1))
    );
    assert_eq!(
        PolicySet::new(
            vec![profile(1, vec![], vec![])],
            vec![endpoint(10, 1), endpoint(10, 1)],
            vec![]
        )
        .unwrap_err(),
        Build::DuplicateEndpoint(id(10))
    );
    let g = GatewayProfileAssignment {
        group: id(20),
        profile: id(1),
    };
    assert_eq!(
        PolicySet::new(vec![profile(1, vec![], vec![])], vec![], vec![g, g]).unwrap_err(),
        Build::DuplicateGroup(id(20))
    );
    assert_eq!(
        PolicySet::new(
            vec![profile(1, vec![], vec![])],
            vec![endpoint(10, 2)],
            vec![]
        )
        .unwrap_err(),
        Build::UnknownProfile(id(2))
    );
    assert_eq!(
        PolicySet::new(
            vec![profile(1, vec![], vec![])],
            vec![],
            vec![GatewayProfileAssignment {
                group: id(20),
                profile: id(2)
            }]
        )
        .unwrap_err(),
        Build::UnknownProfile(id(2))
    );
    let empty = PolicySet::<u8, u8>::new(vec![], vec![], vec![]).unwrap();
    assert_eq!(
        empty.evaluate_dns(&binding(), &query()).unwrap_err(),
        Eval::UnknownEndpoint
    );
}

#[test]
fn aggregate_budget_counts_profiles_bindings_disabled_and_tcp_entries() {
    // One Profile, one Endpoint, one gateway association leave 9997 rule entries.
    let rules = (0..MAX_POLICY_ENTRIES - 3)
        .map(|_| rule(custom(1), 0, Mode::Exact, "example.com", 1))
        .collect();
    let p = policy(rules, vec![]);
    assert_eq!(
        *p.evaluate_dns(&binding(), &"example.com".parse().unwrap())
            .unwrap()
            .action(),
        1
    );
    let rules = (0..MAX_POLICY_ENTRIES - 2)
        .map(|_| rule(custom(1), 0, Mode::Exact, "example.com", 1))
        .collect();
    assert_eq!(
        PolicySet::new(
            vec![profile(1, rules, vec![])],
            vec![endpoint(10, 1)],
            vec![GatewayProfileAssignment {
                group: id(20),
                profile: id(1)
            }]
        )
        .unwrap_err(),
        Build::EntryLimit
    );
    let tcp = (0..MAX_POLICY_ENTRIES)
        .map(|_| {
            TcpRule::new(
                rule(custom(1), 0, Mode::Exact, "example.com", 1),
                TcpPortScope::Any,
            )
        })
        .collect();
    assert_eq!(
        PolicySet::new(vec![profile(1, vec![], tcp)], vec![], vec![]).unwrap_err(),
        Build::EntryLimit
    );
    let disabled = (0..MAX_POLICY_ENTRIES)
        .map(|_| {
            DomainRule::new(
                RuleMetadata {
                    source: custom(1),
                    state: State::Disabled,
                    priority: 0,
                },
                DomainMatcher::new("example.com".parse().unwrap(), Mode::Exact),
                1,
            )
        })
        .collect();
    assert_eq!(
        PolicySet::new(vec![profile(1, disabled, vec![])], vec![], vec![]).unwrap_err(),
        Build::EntryLimit
    );
}

#[test]
fn opaque_effects_and_diagnostics_never_echo_confidential_action_data() {
    let secret = "not-a-real-secret-sensitive-effect";
    let p = PolicySet::new(
        vec![PreparedProfile {
            id: id(1),
            dns_default: secret.to_owned(),
            dns_rules: vec![],
            tcp_default: (),
            tcp_rules: vec![],
        }],
        vec![endpoint(10, 1)],
        vec![],
    )
    .unwrap();
    let result = p.evaluate_dns(&binding(), &query()).unwrap();
    assert!(!format!("{result:?} {p:?}").contains(secret));
    let errors = [
        Eval::UnknownEndpoint,
        Eval::UnavailableEndpoint,
        Eval::EndpointBindingMismatch,
        Eval::UnknownGatewayGroup,
        Eval::UnknownProfile,
        Eval::Ambiguous {
            first: custom(1),
            second: custom(2),
        },
    ];
    for error in errors {
        assert!(error.source().is_none());
        assert!(!error.to_string().contains(secret));
    }
    for error in [
        Build::EntryLimit,
        Build::DuplicateProfile(id(1)),
        Build::DuplicateEndpoint(id(10)),
        Build::DuplicateGroup(id(20)),
        Build::UnknownProfile(id(99)),
        Build::InvalidPortRange,
    ] {
        assert!(error.source().is_none());
        assert!(!error.to_string().contains(secret));
    }
}
