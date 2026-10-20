use droutex_common::{
    AdministrativeState, DnsEndpointAssignment, DnsProfileBinding, GatewayNetworkIdentity,
};
use droutex_policy::{
    CanonicalDomain, DomainMatchMode as Mode, DomainMatcher, DomainRule, GatewayProfileAssignment,
    PolicyPlane, PolicySet, PreparedProfile, RuleMetadata, RuleSource, TcpPortRange,
    TcpPortScope as Ports, TcpRule,
};
use std::{num::NonZeroU16, str::FromStr};

fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn rule(n: usize, priority: i32, name: &str, mode: Mode, effect: u8) -> DomainRule<u8> {
    DomainRule::new(
        RuleMetadata {
            source: RuleSource::Custom(id(n)),
            state: AdministrativeState::Enabled,
            priority,
        },
        DomainMatcher::new(name.parse().unwrap(), mode),
        effect,
    )
}
fn policy(dns: Vec<DomainRule<u8>>, tcp: Vec<TcpRule<u8>>) -> PolicySet<u8, u8> {
    PolicySet::new(
        vec![PreparedProfile {
            id: id(1),
            dns_default: 0,
            dns_rules: dns,
            tcp_default: 0,
            tcp_rules: tcp,
        }],
        vec![DnsEndpointAssignment {
            endpoint: id(10),
            profile: id(1),
            state: AdministrativeState::Enabled,
            device: None,
        }],
        vec![GatewayProfileAssignment {
            group: id(20),
            profile: id(1),
        }],
    )
    .unwrap()
}
fn range(a: u16, b: u16) -> Ports {
    Ports::Range(TcpPortRange::new(a, b).unwrap())
}
fn binding() -> DnsProfileBinding {
    DnsProfileBinding {
        endpoint: id(10),
        profile: id(1),
    }
}
fn identity() -> GatewayNetworkIdentity {
    GatewayNetworkIdentity {
        grant: id(30),
        group: id(20),
        listener: id(40),
        source: "192.0.2.1".parse().unwrap(),
    }
}

#[test]
fn symbolic_result_agrees_with_direct_evaluation_on_complete_fixture_quotients() {
    // This finite condition grammar partitions ALL names into the named bases,
    // their listed child branches, or an unlisted branch (c/deep below). It has
    // no predicates on further labels. The ports below cover every interval cell.
    // The oracle uses the separately tested D38 query evaluator, not trie internals.
    let grammar = [
        ("example.test", Mode::Exact),
        ("example.test", Mode::DomainAndSubdomains),
        ("example.test", Mode::SubdomainsOnly),
        ("a.example.test", Mode::Exact),
        ("a.example.test", Mode::SubdomainsOnly),
        ("a.example.test", Mode::DomainAndSubdomains),
        ("b.a.example.test", Mode::Exact),
    ];
    let names: Vec<CanonicalDomain> = [
        "example.test",
        "a.example.test",
        "b.example.test",
        "c.example.test",
        "b.a.example.test",
        "c.a.example.test",
        "deep.b.a.example.test",
        "deep.c.a.example.test",
        "deep.deep.a.example.test",
        "badexample.test",
        "example.test.attacker",
    ]
    .iter()
    .map(|s| s.parse().unwrap())
    .collect();
    for seed in 0..256 {
        let build = |reverse: bool| {
            let mut dns = Vec::new();
            let mut tcp = Vec::new();
            let order: Vec<_> = if reverse {
                (0..6).rev().collect()
            } else {
                (0..6).collect()
            };
            for i in order {
                let (name, mode) = grammar[(seed + i * i) % grammar.len()];
                let source = match (seed + i) % 3 {
                    0 => RuleSource::Custom(id(100 + i)),
                    1 => RuleSource::Service {
                        rule: id(100 + i),
                        revision: id(200 + i),
                    },
                    _ => RuleSource::Filter(id(200 + i)),
                };
                let r = DomainRule::new(
                    RuleMetadata {
                        source,
                        state: if seed % 17 == i {
                            AdministrativeState::Disabled
                        } else {
                            AdministrativeState::Enabled
                        },
                        priority: ((seed >> (i % 4)) & 3) as i32 - 1,
                    },
                    DomainMatcher::new(name.parse().unwrap(), mode),
                    ((seed >> i) & 1) as u8,
                );
                if seed % 2 == 0 {
                    dns.push(r)
                } else {
                    tcp.push(TcpRule::new(
                        r,
                        match i % 3 {
                            0 => Ports::Any,
                            1 => range(80, 81),
                            _ => range(443, 443),
                        },
                    ))
                }
            }
            policy(dns, tcp)
        };
        let p = build(false);
        let expected = names.iter().any(|name| {
            if seed % 2 == 0 {
                p.evaluate_dns(&binding(), name).is_err()
            } else {
                [1, 79, 80, 81, 82, 442, 443, 444, 65535]
                    .iter()
                    .any(|&port| {
                        p.evaluate_tcp(&identity(), name, NonZeroU16::new(port).unwrap())
                            .is_err()
                    })
            }
        });
        assert_eq!(p.validate_conflicts().is_err(), expected, "seed {seed}");
        assert_eq!(
            p.validate_conflicts(),
            build(true).validate_conflicts(),
            "reorder {seed}"
        );
    }
}

#[test]
fn full_shadowing_is_permitted_but_partial_names_or_ports_do_not_hide_conflicts() {
    let base = || {
        vec![
            rule(100, 0, "example.test", Mode::DomainAndSubdomains, 1),
            rule(101, 0, "example.test", Mode::DomainAndSubdomains, 2),
        ]
    };
    let mut full = base();
    full.push(rule(102, 1, "example.test", Mode::Exact, 3));
    full.push(rule(103, 1, "example.test", Mode::SubdomainsOnly, 3));
    assert!(policy(full, vec![]).validate_conflicts().is_ok());
    let mut partial = base();
    partial.push(rule(102, 1, "example.test", Mode::Exact, 3));
    assert!(policy(partial, vec![]).validate_conflicts().is_err());
    for (start, ok) in [(201, true), (202, false)] {
        let p = policy(
            vec![],
            vec![
                TcpRule::new(rule(100, 0, "example.test", Mode::Exact, 1), range(80, 443)),
                TcpRule::new(rule(101, 0, "example.test", Mode::Exact, 2), range(80, 443)),
                TcpRule::new(rule(102, 1, "example.test", Mode::Exact, 3), range(80, 200)),
                TcpRule::new(
                    rule(103, 1, "example.test", Mode::Exact, 3),
                    range(start, 443),
                ),
            ],
        );
        assert_eq!(p.validate_conflicts().is_ok(), ok);
    }
    let p = policy(
        vec![],
        vec![
            TcpRule::new(
                rule(100, 0, "example.test", Mode::Exact, 1),
                range(65535, 65535),
            ),
            TcpRule::new(
                rule(101, 0, "example.test", Mode::Exact, 2),
                range(65535, 65535),
            ),
            TcpRule::new(
                rule(102, 1, "example.test", Mode::Exact, 3),
                range(1, 65534),
            ),
        ],
    );
    assert_eq!(p.validate_conflicts().unwrap_err().plane, PolicyPlane::Tcp);
}

fn long_name(bytes: usize) -> String {
    format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "a".repeat(63),
        "a".repeat(63),
        "a".repeat(bytes - 192)
    )
}
fn conflict(name: &str) -> Vec<DomainRule<u8>> {
    vec![
        rule(100, 0, name, Mode::SubdomainsOnly, 1),
        rule(101, 0, name, Mode::SubdomainsOnly, 2),
    ]
}

#[test]
fn finite_name_space_has_no_phantom_descendants_or_sample_based_absence() {
    for bytes in [252, 253] {
        assert!(
            policy(conflict(&long_name(bytes)), vec![])
                .validate_conflicts()
                .is_ok()
        );
    }
    let alphabet = "abcdefghijklmnopqrstuvwxyz0123456789-_";
    let name = long_name(251);
    let mut covered = conflict(&name);
    for (i, c) in alphabet.chars().enumerate() {
        covered.push(rule(
            200 + i,
            1,
            &format!("{c}.{name}"),
            Mode::DomainAndSubdomains,
            3,
        ));
    }
    assert!(policy(covered, vec![]).validate_conflicts().is_ok());
    let mut hole = conflict(&name);
    for (i, c) in alphabet.chars().filter(|&c| c != 'z').enumerate() {
        hole.push(rule(
            200 + i,
            1,
            &format!("{c}.{name}"),
            Mode::DomainAndSubdomains,
            3,
        ));
    }
    assert!(policy(hole, vec![]).validate_conflicts().is_err());
    // At250 bytes a full alphabet of one-byte children does not cover two-byte
    // labels. Cover the entire finite one/two-byte language to resolve the tie.
    let name = long_name(250);
    let mut covered = conflict(&name);
    for c in alphabet.chars() {
        covered.push(rule(
            200 + covered.len(),
            1,
            &format!("{c}.{name}"),
            Mode::DomainAndSubdomains,
            3,
        ));
    }
    assert!(policy(covered, vec![]).validate_conflicts().is_err());
    let mut covered = conflict(&name);
    for c in alphabet.chars() {
        covered.push(rule(
            200 + covered.len(),
            1,
            &format!("{c}.{name}"),
            Mode::DomainAndSubdomains,
            3,
        ));
        for d in alphabet.chars() {
            covered.push(rule(
                200 + covered.len(),
                1,
                &format!("{c}{d}.{name}"),
                Mode::DomainAndSubdomains,
                3,
            ));
        }
    }
    assert!(policy(covered, vec![]).validate_conflicts().is_ok());
}

#[test]
fn bidi_context_uses_the_qualified_name_boundary_and_profiles_and_planes_do_not_union() {
    let rtl: CanonicalDomain = "עברית".parse().unwrap();
    let mut name = rtl.as_str().to_owned();
    while 251 - name.len() > 64 {
        name = format!("{}.{}", "a".repeat(63), name);
    }
    name = format!("{}.{}", "a".repeat(251 - name.len() - 1), name);
    assert_eq!(name.len(), 251);
    let mut covered = conflict(&name);
    for c in "abcdefghijklmnopqrstuvwxyz0123456789-_".chars() {
        let child = format!("{c}.{name}");
        if child.parse::<CanonicalDomain>().is_ok() {
            covered.push(rule(200 + covered.len(), 1, &child, Mode::Exact, 3));
        }
    }
    assert!(policy(covered, vec![]).validate_conflicts().is_ok());
    let profiles = (1..=2)
        .map(|n| PreparedProfile {
            id: id(n),
            dns_default: 0,
            dns_rules: vec![rule(100 + n, 0, "BÜCHER.example.", Mode::Exact, n as u8)],
            tcp_default: 0,
            tcp_rules: vec![TcpRule::new(
                rule(
                    200 + n,
                    0,
                    "xn--bcher-kva.example",
                    Mode::Exact,
                    (n + 1) as u8,
                ),
                Ports::Any,
            )],
        })
        .collect();
    assert!(
        PolicySet::new(profiles, vec![], vec![])
            .unwrap()
            .validate_conflicts()
            .is_ok()
    );
}

#[test]
fn maximum_prepared_set_does_not_expand_ports_or_require_cloned_actions() {
    let rules = (0..9999)
        .map(|n| {
            TcpRule::new(
                rule(100 + n, 0, "example.test", Mode::DomainAndSubdomains, 1),
                Ports::Any,
            )
        })
        .collect();
    let set = PolicySet::new(
        vec![PreparedProfile {
            id: id(1),
            dns_default: 0,
            dns_rules: vec![],
            tcp_default: 0,
            tcp_rules: rules,
        }],
        vec![],
        vec![],
    )
    .unwrap();
    assert!(set.validate_conflicts().is_ok());
}
