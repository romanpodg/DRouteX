use droutex_common::{
    AdministrativeState as State, DnsEndpointAssignment, DnsProfileBinding, GatewayNetworkIdentity,
};
use droutex_policy::{
    DomainMatchMode as Mode, DomainMatcher, DomainRule, EvaluationError, GatewayProfileAssignment,
    PolicySet, PreparedProfile, RuleMetadata, RuleSource as Source, TcpPortRange, TcpPortScope,
    TcpRule, TraceDisposition as Why,
};
use std::{num::NonZeroU16, str::FromStr};
#[test]
fn same_source_with_distinct_states_has_stable_trace_order() {
    for reverse in [false, true] {
        let mut rules = vec![
            rule(Source::Custom(id(10)), 0, "same.test", Mode::Exact, 1, true),
            rule(
                Source::Custom(id(10)),
                0,
                "same.test",
                Mode::Exact,
                1,
                false,
            ),
        ];
        if reverse {
            rules.reverse();
        }
        let p = PolicySet::<u8, u8>::new(
            vec![PreparedProfile {
                id: id(1),
                dns_default: 0,
                dns_rules: rules,
                tcp_default: 0,
                tcp_rules: vec![],
            }],
            vec![DnsEndpointAssignment {
                endpoint: id(3),
                profile: id(1),
                state: State::Enabled,
                device: None,
            }],
            vec![],
        )
        .unwrap();
        let trace = p
            .explain_dns(&binding(), &"same.test".parse().unwrap())
            .unwrap();
        assert_eq!(
            trace
                .entries()
                .iter()
                .map(|e| e.disposition())
                .collect::<Vec<_>>(),
            vec![Why::Selected, Why::Disabled]
        );
    }
}
#[test]
fn trace_emits_every_entry_within_the_existing_aggregate_limit() {
    let build = |count| {
        let rules = (1..=count)
            .map(|n| {
                rule(
                    Source::Custom(id(n)),
                    0,
                    "full.example.test",
                    Mode::Exact,
                    1,
                    true,
                )
            })
            .collect();
        PolicySet::<u8, u8>::new(
            vec![PreparedProfile {
                id: id(1),
                dns_default: 0,
                dns_rules: rules,
                tcp_default: 0,
                tcp_rules: vec![],
            }],
            vec![DnsEndpointAssignment {
                endpoint: id(3),
                profile: id(1),
                state: State::Enabled,
                device: None,
            }],
            vec![],
        )
    };
    let p = build(droutex_policy::MAX_POLICY_ENTRIES - 2).unwrap();
    let trace = p
        .explain_dns(&binding(), &"full.example.test".parse().unwrap())
        .unwrap();
    assert_eq!(trace.entries().len(), 9998);
    assert_eq!(
        trace
            .entries()
            .iter()
            .filter(|e| e.disposition() == Why::Selected)
            .count(),
        1
    );
    assert_eq!(
        trace
            .entries()
            .iter()
            .filter(|e| e.disposition() == Why::EquivalentTie)
            .count(),
        9997
    );
    assert!(matches!(
        build(droutex_policy::MAX_POLICY_ENTRIES - 1),
        Err(droutex_policy::PolicyBuildError::EntryLimit)
    ));
}
fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn rule(
    source: Source,
    priority: i32,
    domain: &str,
    mode: Mode,
    action: u8,
    enabled: bool,
) -> DomainRule<u8> {
    DomainRule::new(
        RuleMetadata {
            source,
            priority,
            state: if enabled {
                State::Enabled
            } else {
                State::Disabled
            },
        },
        DomainMatcher::new(domain.parse().unwrap(), mode),
        action,
    )
}
fn binding() -> DnsProfileBinding {
    DnsProfileBinding {
        endpoint: id(3),
        profile: id(1),
    }
}
fn identity() -> GatewayNetworkIdentity {
    GatewayNetworkIdentity {
        grant: id(5),
        group: id(4),
        listener: id(6),
        source: "203.0.113.7".parse().unwrap(),
    }
}
fn policy(reverse: bool, conflict: bool) -> PolicySet<u8, u8> {
    let mut dns = vec![
        rule(
            Source::Custom(id(10)),
            50,
            "example.test",
            Mode::DomainAndSubdomains,
            1,
            true,
        ),
        rule(
            Source::Custom(id(11)),
            49,
            "child.example.test",
            Mode::Exact,
            2,
            true,
        ),
        rule(
            Source::Custom(id(12)),
            50,
            "example.test",
            Mode::DomainAndSubdomains,
            if conflict { 2 } else { 1 },
            true,
        ),
        rule(
            Source::Custom(id(13)),
            100,
            "child.example.test",
            Mode::Exact,
            99,
            false,
        ),
        rule(
            Source::Custom(id(14)),
            100,
            "outside.test",
            Mode::Exact,
            8,
            true,
        ),
        rule(
            Source::Service {
                rule: id(20),
                revision: id(30),
            },
            999,
            "child.example.test",
            Mode::Exact,
            3,
            true,
        ),
        rule(
            Source::Filter(id(40)),
            999,
            "example.test",
            Mode::DomainAndSubdomains,
            4,
            true,
        ),
    ];
    let mut tcp = vec![
        TcpRule::new(
            rule(
                Source::Custom(id(50)),
                0,
                "child.example.test",
                Mode::Exact,
                8,
                true,
            ),
            TcpPortScope::Range(TcpPortRange::new(443, 443).unwrap()),
        ),
        TcpRule::new(
            rule(
                Source::Custom(id(51)),
                100,
                "example.test",
                Mode::DomainAndSubdomains,
                9,
                true,
            ),
            TcpPortScope::Range(TcpPortRange::new(80, 80).unwrap()),
        ),
        TcpRule::new(
            rule(
                Source::Custom(id(52)),
                0,
                "child.example.test",
                Mode::Exact,
                8,
                true,
            ),
            TcpPortScope::Any,
        ),
    ];
    if reverse {
        dns.reverse();
        tcp.reverse();
    }
    PolicySet::new(
        vec![
            PreparedProfile {
                id: id(1),
                dns_default: 0,
                dns_rules: dns,
                tcp_default: 100,
                tcp_rules: vec![],
            },
            PreparedProfile {
                id: id(2),
                dns_default: 200,
                dns_rules: vec![],
                tcp_default: 7,
                tcp_rules: tcp,
            },
        ],
        vec![DnsEndpointAssignment {
            endpoint: id(3),
            profile: id(1),
            state: State::Enabled,
            device: None,
        }],
        vec![GatewayProfileAssignment {
            group: id(4),
            profile: id(2),
        }],
    )
    .unwrap()
}
#[test]
fn actual_selector_trace_has_expected_reasons_and_permutation_order() {
    for reverse in [false, true] {
        let p = policy(reverse, false);
        let query = "child.example.test".parse().unwrap();
        let trace = p.explain_dns(&binding(), &query).unwrap();
        assert_eq!(
            trace.selection(),
            &p.evaluate_dns(&binding(), &query).unwrap()
        );
        assert_eq!(*trace.selection().action(), 1);
        assert_eq!(trace.selection().source(), Some(Source::Custom(id(10))));
        assert_eq!(
            trace
                .entries()
                .iter()
                .map(|e| (e.source(), e.disposition()))
                .collect::<Vec<_>>(),
            vec![
                (Source::Custom(id(13)), Why::Disabled),
                (Source::Custom(id(14)), Why::DomainMismatch),
                (Source::Custom(id(10)), Why::Selected),
                (Source::Custom(id(12)), Why::EquivalentTie),
                (Source::Custom(id(11)), Why::LowerRank),
                (
                    Source::Service {
                        rule: id(20),
                        revision: id(30)
                    },
                    Why::LowerRank
                ),
                (Source::Filter(id(40)), Why::LowerRank),
            ]
        );
        assert!(trace.entries().iter().all(|e| e.port_matches().is_none()));
        assert!(!format!("{trace:?}").contains("example"));
        let fallback = p
            .explain_dns(&binding(), &"unmatched.test".parse().unwrap())
            .unwrap();
        assert_eq!(fallback.selection().source(), None);
        assert_eq!(*fallback.selection().action(), 0);
        assert_eq!(fallback.entries().len(), 7);
    }
}
#[test]
fn tcp_trace_uses_actual_port_and_independent_profile() {
    for reverse in [false, true] {
        let p = policy(reverse, false);
        let query = "child.example.test".parse().unwrap();
        let identity = identity();
        let trace = p
            .explain_tcp(&identity, &query, NonZeroU16::new(443).unwrap())
            .unwrap();
        assert_eq!(
            trace.selection(),
            &p.evaluate_tcp(&identity, &query, NonZeroU16::new(443).unwrap())
                .unwrap()
        );
        assert_eq!(trace.selection().profile(), id(2));
        assert_eq!(*trace.selection().action(), 8);
        assert_eq!(
            trace
                .entries()
                .iter()
                .map(|e| (e.source(), e.disposition(), e.port_matches()))
                .collect::<Vec<_>>(),
            vec![
                (Source::Custom(id(51)), Why::PortMismatch, Some(false)),
                (Source::Custom(id(50)), Why::Selected, Some(true)),
                (Source::Custom(id(52)), Why::EquivalentTie, Some(true)),
            ]
        );
        let port80 = p
            .explain_tcp(&identity, &query, NonZeroU16::new(80).unwrap())
            .unwrap();
        assert_eq!(*port80.selection().action(), 9);
    }
}
#[test]
fn refused_scope_and_conflicts_never_return_a_partial_trace() {
    let p = policy(false, false);
    let query = "child.example.test".parse().unwrap();
    assert_eq!(
        p.explain_dns(
            &DnsProfileBinding {
                endpoint: id(999),
                ..binding()
            },
            &query
        )
        .unwrap_err(),
        EvaluationError::UnknownEndpoint
    );
    assert_eq!(
        p.explain_dns(
            &DnsProfileBinding {
                profile: id(2),
                ..binding()
            },
            &query
        )
        .unwrap_err(),
        EvaluationError::EndpointBindingMismatch
    );
    assert_eq!(
        p.explain_tcp(
            &GatewayNetworkIdentity {
                group: id(999),
                ..identity()
            },
            &query,
            NonZeroU16::new(443).unwrap()
        )
        .unwrap_err(),
        EvaluationError::UnknownGatewayGroup
    );
    let p = policy(false, true);
    assert_eq!(
        p.explain_dns(&binding(), &query).unwrap_err(),
        p.evaluate_dns(&binding(), &query).unwrap_err()
    );
    assert!(matches!(
        p.explain_dns(&binding(), &query),
        Err(EvaluationError::Ambiguous { .. })
    ));
}
