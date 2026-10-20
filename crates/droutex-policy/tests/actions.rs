use droutex_common::{
    AdministrativeState as State, DnsUpstreamId, OutboundAvailability as Availability,
    ProxyOutboundId, ProxyOutboundKind as Kind,
};
use droutex_policy::{
    ActionInventoryError as Inventory, ActionRefusal as Refusal, ActionResolver, DecisionPoint,
    DnsIntent, GatewayReadiness, MAX_POLICY_ENTRIES, OutboundFacts, PreparedPrivateRecord,
    PrivateRecordValue, RequestTransport, TcpCapability, TcpIntent, local_answer_intent,
};
use std::{error::Error, str::FromStr};

fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn fact(n: usize, kind: Kind) -> OutboundFacts {
    OutboundFacts {
        id: id(n),
        state: State::Enabled,
        kind,
        availability: Availability::Available,
        tcp: TcpCapability::Supported,
    }
}

#[test]
fn both_native_kinds_require_positive_facts_and_actual_tcp() {
    // Each row removes one prerequisite, independently of the other rows.
    let cases = [
        (
            State::Enabled,
            Availability::Available,
            TcpCapability::Supported,
            None,
        ),
        (
            State::Disabled,
            Availability::Available,
            TcpCapability::Supported,
            Some(Refusal::DisabledOutbound(id(1))),
        ),
        (
            State::Enabled,
            Availability::Unavailable,
            TcpCapability::Supported,
            Some(Refusal::UnavailableOutbound(id(1))),
        ),
        (
            State::Enabled,
            Availability::Unknown,
            TcpCapability::Supported,
            Some(Refusal::UnknownAvailability(id(1))),
        ),
        (
            State::Enabled,
            Availability::Available,
            TcpCapability::Unsupported,
            Some(Refusal::UnsupportedCapability(id(1))),
        ),
        (
            State::Enabled,
            Availability::Available,
            TcpCapability::Unknown,
            Some(Refusal::UnknownCapability(id(1))),
        ),
    ];
    for kind in [Kind::GatewayDirect, Kind::HttpConnectTls] {
        for (state, availability, tcp, refused) in cases {
            let resolver = ActionResolver::new(
                vec![],
                vec![OutboundFacts {
                    state,
                    availability,
                    tcp,
                    ..fact(1, kind)
                }],
            )
            .unwrap();
            let expected = match (refused, kind) {
                (Some(reason), _) => TcpIntent::Deny(reason),
                (None, Kind::GatewayDirect) => TcpIntent::GatewayDirect(id(1)),
                (None, Kind::HttpConnectTls) => TcpIntent::ProtectedProxy(id(1)),
            };
            assert_eq!(
                resolver.tcp(id(1), RequestTransport::Tcp),
                expected,
                "{state:?} {availability:?} {tcp:?} {kind:?}"
            );
            assert_eq!(
                resolver.dns_gateway(id(1), GatewayReadiness::CompatibleTcp),
                refused.map_or(DnsIntent::Gateway(id(1)), DnsIntent::Refuse)
            );
            for transport in [RequestTransport::Udp, RequestTransport::Unknown] {
                assert_eq!(
                    resolver.tcp(id(1), transport),
                    TcpIntent::Deny(Refusal::UnsupportedTransport)
                );
            }
            for readiness in [GatewayReadiness::Unknown, GatewayReadiness::Unavailable] {
                assert_eq!(
                    resolver.dns_gateway(id(1), readiness),
                    DnsIntent::Refuse(Refusal::GatewayNotReady)
                );
            }
        }
    }
}

#[test]
fn no_inventory_or_missing_ids_never_imply_available_routes() {
    let resolver = ActionResolver::new(vec![id(90)], vec![]).unwrap();
    assert_eq!(resolver.dns_origin(id(90)), DnsIntent::Resolve(id(90)));
    assert_eq!(
        resolver.dns_origin(id(91)),
        DnsIntent::Refuse(Refusal::UnknownUpstream(id(91)))
    );
    assert_eq!(
        resolver.dns_gateway(id(1), GatewayReadiness::CompatibleTcp),
        DnsIntent::Refuse(Refusal::UnknownOutbound(id(1)))
    );
    assert_eq!(
        resolver.tcp(id(1), RequestTransport::Tcp),
        TcpIntent::Deny(Refusal::UnknownOutbound(id(1)))
    );
    assert_eq!(DnsIntent::Block.decision_point(), None);
    assert_eq!(resolver.dns_origin(id(91)).decision_point(), None);
}

#[test]
fn protected_failure_denies_even_when_explicit_direct_is_available() {
    let resolver = ActionResolver::new(
        vec![id(90)],
        vec![fact(1, Kind::HttpConnectTls), fact(2, Kind::GatewayDirect)],
    )
    .unwrap();
    let protected = resolver.tcp(id(1), RequestTransport::Tcp);
    let direct = resolver.tcp(id(2), RequestTransport::Tcp);
    assert_eq!(
        protected.decision_point(),
        Some(DecisionPoint::ProtectedProxyExit)
    );
    assert_eq!(direct.decision_point(), Some(DecisionPoint::GatewayVps));
    assert_eq!(
        resolver.dns_origin(id(90)).decision_point(),
        Some(DecisionPoint::ClientOrigin)
    );
    // DNS gateway ingress and protected proxy exit are different decision points.
    assert_eq!(
        resolver
            .dns_gateway(id(1), GatewayReadiness::CompatibleTcp)
            .decision_point(),
        Some(DecisionPoint::GatewayVps)
    );
    for (intent, n) in [(protected, 1), (direct, 2)] {
        let failed = intent.on_connection_failure();
        assert_eq!(failed, TcpIntent::Deny(Refusal::ConnectionFailed(id(n))));
        assert_eq!(failed.decision_point(), None);
        assert_eq!(failed.on_connection_failure(), failed);
    }
    let denied = TcpIntent::Deny(Refusal::PolicyDenied);
    assert_eq!(denied.on_connection_failure(), denied);
    // The other outbound remains eligible only if explicitly selected.
    assert_eq!(
        resolver.tcp(id(2), RequestTransport::Tcp),
        TcpIntent::GatewayDirect(id(2))
    );
}

#[test]
fn inventory_rejects_duplicates_deterministically_and_bounds_total_before_indexing() {
    let a = fact(1, Kind::GatewayDirect);
    let b = fact(2, Kind::HttpConnectTls);
    for outbounds in [vec![b, a, b, a], vec![a, b, a, b]] {
        assert_eq!(
            ActionResolver::new(vec![], outbounds).unwrap_err(),
            Inventory::DuplicateOutbound(id(1))
        );
    }
    for upstreams in [
        vec![id(2), id(1), id(2), id(1)],
        vec![id(1), id(2), id(1), id(2)],
    ] {
        assert_eq!(
            ActionResolver::new(upstreams, vec![]).unwrap_err(),
            Inventory::DuplicateUpstream(id(1))
        );
    }
    let upstreams: Vec<DnsUpstreamId> = (1..MAX_POLICY_ENTRIES).map(id).collect();
    let resolver = ActionResolver::new(upstreams, vec![a]).unwrap();
    assert_eq!(
        resolver.dns_origin(id(MAX_POLICY_ENTRIES - 1)),
        DnsIntent::Resolve(id(MAX_POLICY_ENTRIES - 1))
    );
    assert_eq!(
        ActionResolver::new((1..=MAX_POLICY_ENTRIES).map(id).collect(), vec![a]).unwrap_err(),
        Inventory::EntryLimit
    );
    assert_eq!(
        ActionResolver::new(vec![], vec![a; MAX_POLICY_ENTRIES + 1]).unwrap_err(),
        Inventory::EntryLimit
    );
    // IDs can coincide between typed resource namespaces without a collision.
    assert!(ActionResolver::new(vec![id(1)], vec![a]).is_ok());
}

#[test]
fn local_answers_preserve_borrowed_typed_data_and_do_not_cross_profile_or_owner() {
    let values = [
        PrivateRecordValue::Ipv4("192.0.2.29".parse().unwrap()),
        PrivateRecordValue::Ipv6("2001:db8::29".parse().unwrap()),
        PrivateRecordValue::Cname("private-target.example".parse().unwrap()),
    ];
    for value in values {
        let record = PreparedPrivateRecord {
            id: id(20),
            profile: id(1),
            owner: "BÜCHER.Example.".parse().unwrap(),
            ttl: 300,
            value,
        };
        let query = "xn--bcher-kva.example".parse().unwrap();
        let intent = local_answer_intent(&record, id(1), &query).unwrap();
        assert!(std::ptr::eq(intent.value(), &record.value));
        assert_eq!(
            (intent.profile(), intent.record_id(), intent.ttl()),
            (id(1), id(20), 300)
        );
        assert_eq!(
            local_answer_intent(&record, id(2), &query).unwrap_err(),
            Refusal::PrivateProfileMismatch
        );
        assert_eq!(
            local_answer_intent(
                &record,
                id(1),
                &"child.xn--bcher-kva.example".parse().unwrap()
            )
            .unwrap_err(),
            Refusal::PrivateNameMismatch
        );
        let diagnostic = format!("{intent:?}");
        for confidential in [
            "bcher",
            "BÜCHER",
            "private-target",
            "192.0.2.29",
            "2001:db8",
        ] {
            assert!(!diagnostic.contains(confidential));
        }
    }
}

#[test]
fn diagnostics_report_only_sanitized_reasons_and_inventory_counts() {
    let resolver = ActionResolver::new(vec![id(90)], vec![fact(1, Kind::HttpConnectTls)]).unwrap();
    assert!(!format!("{resolver:?}").contains(&id::<ProxyOutboundId>(1).to_string()));
    for error in [
        Refusal::ConnectionFailed(id(1)),
        Refusal::PrivateNameMismatch,
    ] {
        assert!(error.source().is_none());
        assert_eq!(error.to_string(), "action refused or unavailable");
    }
    for error in [
        Inventory::EntryLimit,
        Inventory::DuplicateUpstream(id(90)),
        Inventory::DuplicateOutbound(id(1)),
    ] {
        assert!(error.source().is_none());
        assert_eq!(error.to_string(), "invalid action inventory");
    }
}
