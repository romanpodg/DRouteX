use droutex_common::{
    AdministrativeState, DnsEndpointAssignment, DnsProfileBinding, GatewayNetworkIdentity,
    OutboundAvailability, ProxyOutboundKind,
};
use droutex_config::{
    ActionPreparationError as Preparation, DnsAction, InputFormat, InvalidActionScope,
    OutboundObservation, ReferenceCatalog, RuleAction, TrafficAction, parse_configuration,
    resolve_dns_action, resolve_tcp_action,
};
use droutex_policy::{
    ActionInventoryError, ActionRefusal, ActionResolver, DnsIntent, DomainMatchMode, DomainMatcher,
    DomainRule, GatewayProfileAssignment, GatewayReadiness, MAX_POLICY_ENTRIES, PolicySet,
    PreparedProfile, RequestTransport, RuleMetadata, RuleSource, TcpCapability, TcpIntent,
    TcpPortScope, TcpRule,
};
use serde_json::Value;
use std::{borrow::Borrow, error::Error, num::NonZeroU16, str::FromStr};

fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn refs() -> ReferenceCatalog {
    ReferenceCatalog {
        devices: [id(120)].into(),
        upstreams: [id(90)].into(),
        secrets: [id(99)].into(),
        filter_revisions: [id(101)].into(),
        service_revisions: [id(100)].into(),
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/configuration-v1.json")).unwrap()
}
fn checked(value: &Value) -> droutex_config::CheckedConfiguration {
    parse_configuration(
        &serde_json::to_vec(value).unwrap(),
        InputFormat::Json,
        &refs(),
    )
    .unwrap()
}
fn observations() -> Vec<OutboundObservation> {
    vec![
        OutboundObservation {
            id: id(70),
            kind: ProxyOutboundKind::GatewayDirect,
            availability: OutboundAvailability::Available,
            tcp: TcpCapability::Supported,
        },
        OutboundObservation {
            id: id(71),
            kind: ProxyOutboundKind::HttpConnectTls,
            availability: OutboundAvailability::Available,
            tcp: TcpCapability::Supported,
        },
    ]
}

#[test]
fn observations_cannot_enable_disabled_config_or_substitute_a_kind() {
    let mut input = fixture();
    input["outbounds"][1]["enabled"] = false.into();
    let config = checked(&input);
    let before = serde_json::to_vec(config.document()).unwrap();
    let mut observed = observations();
    for _ in 0..2 {
        let resolver = config
            .prepare_action_resolver(&refs(), observed.clone())
            .unwrap();
        assert_eq!(
            resolver.tcp(id(71), RequestTransport::Tcp),
            TcpIntent::Deny(ActionRefusal::DisabledOutbound(id(71)))
        );
        assert_eq!(
            resolver.tcp(id(70), RequestTransport::Tcp),
            TcpIntent::GatewayDirect(id(70))
        );
        observed.reverse();
    }
    let mut wrong_kind = observations();
    wrong_kind[1].kind = ProxyOutboundKind::GatewayDirect;
    assert_eq!(
        config
            .prepare_action_resolver(&refs(), wrong_kind)
            .unwrap_err(),
        Preparation::KindMismatch(id(71))
    );
    assert_eq!(serde_json::to_vec(config.document()).unwrap(), before);
}

#[test]
fn missing_stale_duplicate_and_unbounded_observations_fail_closed() {
    let config = checked(&fixture());
    let unknown = config.prepare_action_resolver(&refs(), vec![]).unwrap();
    for n in [70, 71] {
        assert_eq!(
            unknown.tcp(id(n), RequestTransport::Tcp),
            TcpIntent::Deny(ActionRefusal::UnknownAvailability(id(n)))
        );
    }
    let mut removed = refs();
    removed.upstreams.clear();
    assert!(matches!(
        config.prepare_action_resolver(&removed, observations()),
        Err(Preparation::Configuration(_))
    ));
    let mut removed = refs();
    removed.secrets.clear();
    assert!(matches!(
        config.prepare_action_resolver(&removed, observations()),
        Err(Preparation::Configuration(_))
    ));
    let mut unknown_id = observations();
    unknown_id[1].id = id(72);
    assert_eq!(
        config
            .prepare_action_resolver(&refs(), unknown_id)
            .unwrap_err(),
        Preparation::UnknownObservation(id(72))
    );
    let one = observations()[0];
    let mut duplicate = vec![observations()[1], one, one];
    for _ in 0..2 {
        assert_eq!(
            config
                .prepare_action_resolver(&refs(), duplicate.clone())
                .unwrap_err(),
            Preparation::DuplicateObservation(id(70))
        );
        duplicate.reverse();
    }
    assert_eq!(
        config
            .prepare_action_resolver(&refs(), vec![one; MAX_POLICY_ENTRIES + 1])
            .unwrap_err(),
        Preparation::Inventory(ActionInventoryError::EntryLimit)
    );
    let mut many = refs();
    many.upstreams = (1..MAX_POLICY_ENTRIES - 1).map(id).collect();
    assert!(
        config
            .prepare_action_resolver(&many, observations())
            .is_ok()
    );
    many.upstreams = (1..MAX_POLICY_ENTRIES).map(id).collect();
    // 9999 current references plus two declared outbounds exceed the combined budget.
    assert_eq!(
        config
            .prepare_action_resolver(&many, observations())
            .unwrap_err(),
        Preparation::Inventory(ActionInventoryError::EntryLimit)
    );
}

fn rule<A>(n: usize, action: A) -> DomainRule<A> {
    DomainRule::new(
        RuleMetadata {
            source: RuleSource::Custom(id(n)),
            state: AdministrativeState::Enabled,
            priority: 0,
        },
        DomainMatcher::new("route.example".parse().unwrap(), DomainMatchMode::Exact),
        action,
    )
}
fn endpoint(n: usize, p: usize) -> DnsEndpointAssignment {
    DnsEndpointAssignment {
        endpoint: id(n),
        profile: id(p),
        state: AdministrativeState::Enabled,
        device: None,
    }
}
fn identity() -> GatewayNetworkIdentity {
    // Already-trusted test identity: this test does not authenticate it.
    GatewayNetworkIdentity {
        grant: id(30),
        group: id(40),
        listener: id(34),
        source: "192.0.2.1".parse().unwrap(),
    }
}
fn verify_separation<A: Borrow<DnsAction> + Eq, B: Borrow<TrafficAction> + Eq>(
    profiles: Vec<PreparedProfile<A, B>>,
    resolver: &ActionResolver,
) {
    let policy = PolicySet::new(
        profiles,
        vec![endpoint(10, 1), endpoint(12, 2)],
        vec![GatewayProfileAssignment {
            group: id(40),
            profile: id(2),
        }],
    )
    .unwrap();
    let binding = DnsProfileBinding {
        endpoint: id(10),
        profile: id(1),
    };
    let route = "route.example".parse().unwrap();
    let other = "other.example".parse().unwrap();
    let dns = resolve_dns_action(
        resolver,
        &policy.evaluate_dns(&binding, &route).unwrap(),
        GatewayReadiness::CompatibleTcp,
    )
    .unwrap();
    assert_eq!(
        (dns.endpoint(), dns.profile(), dns.source(), dns.intent()),
        (
            id(10),
            id(1),
            Some(RuleSource::Custom(id(80))),
            DnsIntent::Gateway(id(71))
        )
    );
    let bypass = resolve_dns_action(
        resolver,
        &policy.evaluate_dns(&binding, &other).unwrap(),
        GatewayReadiness::Unknown,
    )
    .unwrap();
    assert_eq!(
        (bypass.source(), bypass.intent()),
        (None, DnsIntent::Resolve(id(90)))
    );
    let blocked = resolve_dns_action(
        resolver,
        &policy
            .evaluate_dns(
                &DnsProfileBinding {
                    endpoint: id(12),
                    profile: id(2),
                },
                &route,
            )
            .unwrap(),
        GatewayReadiness::CompatibleTcp,
    )
    .unwrap();
    assert_eq!(blocked.intent(), DnsIntent::Block);
    let tcp = resolve_tcp_action(
        resolver,
        &policy
            .evaluate_tcp(&identity(), &route, NonZeroU16::new(443).unwrap())
            .unwrap(),
        RequestTransport::Tcp,
    )
    .unwrap();
    assert_eq!(
        (tcp.group(), tcp.profile(), tcp.source(), tcp.intent()),
        (
            id(40),
            id(2),
            Some(RuleSource::Custom(id(81))),
            TcpIntent::ProtectedProxy(id(71))
        )
    );
    let failed = tcp.on_connection_failure();
    assert_eq!(
        (failed.group(), failed.profile(), failed.source()),
        (tcp.group(), tcp.profile(), tcp.source())
    );
    assert_eq!(
        failed.intent(),
        TcpIntent::Deny(ActionRefusal::ConnectionFailed(id(71)))
    );
    let selection = policy
        .evaluate_tcp(&identity(), &other, NonZeroU16::new(443).unwrap())
        .unwrap();
    assert_eq!(
        resolve_tcp_action(resolver, &selection, RequestTransport::Tcp)
            .unwrap()
            .intent(),
        TcpIntent::GatewayDirect(id(70))
    );
    assert_eq!(
        resolve_tcp_action(resolver, &selection, RequestTransport::Udp)
            .unwrap()
            .intent(),
        TcpIntent::Deny(ActionRefusal::UnsupportedTransport)
    );
    // Re-evaluating DNS from another Profile cannot mutate the independent group.
    assert_eq!(
        resolve_tcp_action(resolver, &selection, RequestTransport::Tcp)
            .unwrap()
            .profile(),
        id(2)
    );
}

#[test]
fn owned_actions_keep_dns_and_gateway_profile_source_and_failure_context_separate() {
    let config = checked(&fixture());
    let resolver = config
        .prepare_action_resolver(&refs(), observations())
        .unwrap();
    verify_separation(
        vec![
            PreparedProfile {
                id: id(1),
                dns_default: DnsAction::Resolve { upstream: id(90) },
                dns_rules: vec![rule(80, DnsAction::Gateway { outbound: id(71) })],
                tcp_default: TrafficAction::Deny {},
                tcp_rules: vec![],
            },
            PreparedProfile {
                id: id(2),
                dns_default: DnsAction::Block {},
                dns_rules: vec![],
                tcp_default: TrafficAction::Outbound { outbound: id(70) },
                tcp_rules: vec![TcpRule::new(
                    rule(81, TrafficAction::Outbound { outbound: id(71) }),
                    TcpPortScope::Any,
                )],
            },
        ],
        &resolver,
    );
}

#[test]
fn borrowed_wire_actions_keep_the_same_semantics_without_rewriting_config() {
    let config = checked(&fixture());
    let before = serde_json::to_vec(config.document()).unwrap();
    let resolver = config
        .prepare_action_resolver(&refs(), observations())
        .unwrap();
    let first = &config.document().profiles[0];
    let second = &config.document().profiles[1];
    let RuleAction::Traffic(proxy) = &first.custom_rules[1].action else {
        panic!("fixture traffic")
    };
    let gateway = DnsAction::Gateway { outbound: id(71) };
    verify_separation(
        vec![
            PreparedProfile {
                id: id(1),
                dns_default: &first.default_dns_action,
                dns_rules: vec![rule(80, &gateway)],
                tcp_default: &second.default_traffic_action,
                tcp_rules: vec![],
            },
            PreparedProfile {
                id: id(2),
                dns_default: &second.default_dns_action,
                dns_rules: vec![],
                tcp_default: &first.default_traffic_action,
                tcp_rules: vec![TcpRule::new(rule(81, proxy), TcpPortScope::Any)],
            },
        ],
        &resolver,
    );
    assert_eq!(serde_json::to_vec(config.document()).unwrap(), before);
}

#[test]
fn wrong_decision_scope_is_rejected_even_if_both_policy_planes_use_the_same_type() {
    let resolver = ActionResolver::new(vec![], vec![]).unwrap();
    let dns = PolicySet::new(
        vec![PreparedProfile {
            id: id(1),
            dns_default: DnsAction::Block {},
            dns_rules: vec![],
            tcp_default: DnsAction::Block {},
            tcp_rules: vec![],
        }],
        vec![endpoint(10, 1)],
        vec![GatewayProfileAssignment {
            group: id(40),
            profile: id(1),
        }],
    )
    .unwrap();
    let query = "example.com".parse().unwrap();
    let selection = dns
        .evaluate_tcp(&identity(), &query, NonZeroU16::new(443).unwrap())
        .unwrap();
    assert_eq!(
        resolve_dns_action(&resolver, &selection, GatewayReadiness::CompatibleTcp),
        Err(InvalidActionScope)
    );
    let tcp = PolicySet::new(
        vec![PreparedProfile {
            id: id(1),
            dns_default: TrafficAction::Deny {},
            dns_rules: vec![],
            tcp_default: TrafficAction::Deny {},
            tcp_rules: vec![],
        }],
        vec![endpoint(10, 1)],
        vec![],
    )
    .unwrap();
    let selection = tcp
        .evaluate_dns(
            &DnsProfileBinding {
                endpoint: id(10),
                profile: id(1),
            },
            &query,
        )
        .unwrap();
    assert_eq!(
        resolve_tcp_action(&resolver, &selection, RequestTransport::Tcp),
        Err(InvalidActionScope)
    );
    // A correctly scoped policy denial remains denial even for an unsupported request.
    let denial = PolicySet::new(
        vec![PreparedProfile {
            id: id(1),
            dns_default: (),
            dns_rules: vec![],
            tcp_default: TrafficAction::Deny {},
            tcp_rules: vec![],
        }],
        vec![],
        vec![GatewayProfileAssignment {
            group: id(40),
            profile: id(1),
        }],
    )
    .unwrap();
    let selection = denial
        .evaluate_tcp(&identity(), &query, NonZeroU16::new(443).unwrap())
        .unwrap();
    assert_eq!(
        resolve_tcp_action(&resolver, &selection, RequestTransport::Unknown)
            .unwrap()
            .intent(),
        TcpIntent::Deny(ActionRefusal::PolicyDenied)
    );
}

#[test]
fn unsupported_wire_transports_cannot_become_native_actions_and_errors_are_redacted() {
    for kind in ["node", "tunnel", "engine", "socks5", "udp"] {
        let mut input = fixture();
        input["outbounds"][1]["transport"]["kind"] = kind.into();
        assert!(
            parse_configuration(
                &serde_json::to_vec(&input).unwrap(),
                InputFormat::Json,
                &refs()
            )
            .is_err(),
            "{kind}"
        );
    }
    let config = checked(&fixture());
    let mut removed = refs();
    removed.secrets.clear();
    let error = config
        .prepare_action_resolver(&removed, observations())
        .unwrap_err();
    assert!(error.source().is_some());
    let resolver = config
        .prepare_action_resolver(&refs(), observations())
        .unwrap();
    let debug = format!("{resolver:?} {error:?} {error}");
    assert!(!debug.contains("proxy.example"));
    assert!(!debug.contains("credentials"));
    for error in [
        Preparation::UnknownObservation(id(71)),
        Preparation::KindMismatch(id(71)),
        Preparation::DuplicateObservation(id(71)),
    ] {
        assert!(error.source().is_none());
    }
    assert!(InvalidActionScope.source().is_none());
}
