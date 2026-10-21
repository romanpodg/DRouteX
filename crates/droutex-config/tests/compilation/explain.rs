use super::*;
use droutex_policy::{GatewayReadiness, RequestTransport, TcpCapability};
use std::net::TcpListener;
#[test]
fn contradictory_tcp_quic_inputs_cannot_predict_success() {
    let store = catalogs();
    let p = publication(&document(), &store).unwrap();
    let request = SimulationRequest::Tcp {
        identity: identity(),
        domain: "allowed.example.test",
        port: NonZeroU16::new(443).unwrap(),
        transport: RequestTransport::Tcp,
        context: DiagnosticContext {
            ech: Evidence::Unknown,
            quic: Evidence::Present,
        },
    };
    assert_eq!(
        p.content()
            .explain_selection(p.compiled(), request, observed())
            .unwrap_err(),
        SimulationError::Inconsistent
    );
}
fn observed() -> Vec<OutboundObservation> {
    vec![
        OutboundObservation {
            id: id(50),
            kind: ProxyOutboundKind::GatewayDirect,
            availability: OutboundAvailability::Available,
            tcp: TcpCapability::Supported,
        },
        OutboundObservation {
            id: id(51),
            kind: ProxyOutboundKind::HttpConnectTls,
            availability: OutboundAvailability::Available,
            tcp: TcpCapability::Supported,
        },
    ]
}
fn dns(domain: &str) -> SimulationRequest<'_> {
    SimulationRequest::Dns {
        binding: binding(),
        domain,
        readiness: GatewayReadiness::CompatibleTcp,
        context: DiagnosticContext::default(),
    }
}
fn tcp(
    domain: &str,
    identity: GatewayNetworkIdentity,
    transport: RequestTransport,
) -> SimulationRequest<'_> {
    SimulationRequest::Tcp {
        identity,
        domain,
        port: NonZeroU16::new(443).unwrap(),
        transport,
        context: DiagnosticContext {
            ech: Evidence::Present,
            quic: if transport == RequestTransport::Tcp {
                Evidence::Absent
            } else {
                Evidence::Present
            },
        },
    }
}
fn run(p: &CompletePublication, request: SimulationRequest<'_>) -> Value {
    serde_json::to_value(
        p.content()
            .explain_selection(p.compiled(), request, observed())
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn catalog_precedence_contexts_private_visibility_and_reordered_outputs_match_golden() {
    let store = catalogs();
    let mut doc = document();
    doc["privateRecords"].as_array_mut().unwrap().push(json!({
        "id":text(72),"profile":text(2),"name":"private.example.test","ttl":30,
        "record":{"kind":"a","address":"192.0.2.200"}
    }));
    let p = publication(&doc, &store).unwrap();
    let dns_service = run(&p, dns("SERVICE.example.test."));
    assert_eq!(dns_service["selected_source"]["kind"], "service");
    assert_eq!(dns_service["resolved_intent"]["code"], "disabled_outbound");
    assert_eq!(
        dns_service["selected_source"]["resource_hash"],
        ContentHash::of(
            store
                .get(CatalogRevision::Service(id(201)))
                .unwrap()
                .resource_bytes()
        )
        .to_string()
    );
    let blocked = run(&p, dns("blocked.example.test"));
    assert_eq!(blocked["selected_source"]["kind"], "filter");
    assert_eq!(blocked["resolved_intent"]["kind"], "dns_block");
    let private = run(&p, dns("PRIVATE.example.test."));
    assert_eq!(private["private_response_precedence"], "pending_p3");
    assert_eq!(private["private_owners"].as_array().unwrap().len(), 1);
    let v4 = run(
        &p,
        tcp("allowed.example.test", identity(), RequestTransport::Tcp),
    );
    assert_eq!(v4["profile"], text(2));
    assert_eq!(v4["resolved_intent"]["kind"], "gateway_direct");
    let udp = run(
        &p,
        tcp("allowed.example.test", identity(), RequestTransport::Udp),
    );
    assert_eq!(udp["resolved_intent"]["code"], "unsupported_transport");
    let mut doc6 = document();
    doc6["gatewayGrants"][0]["source"] = "2001:db8::7".into();
    let p6 = publication(&doc6, &store).unwrap();
    let ipv6 = GatewayNetworkIdentity {
        source: "2001:db8::7".parse().unwrap(),
        ..identity()
    };
    let v6 = run(
        &p6,
        tcp("allowed.example.test", ipv6, RequestTransport::Tcp),
    );
    assert!(
        v6["limitations"]
            .as_array()
            .unwrap()
            .contains(&Value::from("ipv4_authorization_not_established"))
    );
    assert!(
        v4["limitations"]
            .as_array()
            .unwrap()
            .contains(&Value::from("ipv6_authorization_not_established"))
    );
    let matrix = json!({"dns_service":dns_service,"dns_filter":blocked,"dns_private":private,"tcp_ipv4_ech_nat":v4,"tcp_udp":udp,"tcp_ipv6":v6});
    for v in matrix.as_object().unwrap().values() {
        assert_eq!(v["inspected_entries"], v["emitted_entries"]);
        assert_eq!(
            v["emitted_entries"].as_u64().unwrap() as usize,
            v["trace"].as_array().unwrap().len()
        );
    }
    let raw = serde_json::to_string(&matrix).unwrap();
    for secret in [
        "192.0.2.10",
        "proxy.test",
        "203.0.113.7",
        "2001:db8::7",
        "owner:synthetic-test",
        "local:compiler-fixture",
    ] {
        assert!(!raw.contains(secret), "leaked {secret}");
    }
    let mut reversed = doc;
    for key in ["profiles", "listeners", "outbounds", "privateRecords"] {
        reversed[key].as_array_mut().unwrap().reverse();
    }
    let reordered = publication(&reversed, &store).unwrap();
    assert_eq!(
        run(&reordered, dns("service.example.test")),
        matrix["dns_service"]
    );
    assert_eq!(
        matrix,
        serde_json::from_str::<Value>(include_str!(
            "../../../../api/schemas/explain/selection-examples.json"
        ))
        .unwrap()
    );
}
#[test]
fn static_scope_all_dimensions_and_absent_observations_refuse_without_partial_output() {
    let store = catalogs();
    let p = publication(&document(), &store).unwrap();
    let valid = identity();
    for wrong in [
        GatewayNetworkIdentity {
            grant: id(999),
            ..valid
        },
        GatewayNetworkIdentity {
            group: id(999),
            ..valid
        },
        GatewayNetworkIdentity {
            listener: id(999),
            ..valid
        },
        GatewayNetworkIdentity {
            source: "203.0.113.8".parse().unwrap(),
            ..valid
        },
        GatewayNetworkIdentity {
            source: "2001:db8::7".parse().unwrap(),
            ..valid
        },
    ] {
        assert_eq!(
            p.content()
                .explain_selection(
                    p.compiled(),
                    tcp("allowed.example.test", wrong, RequestTransport::Tcp),
                    observed()
                )
                .unwrap_err(),
            SimulationError::Scope
        );
    }
    for domain in ["dep-only.example.test", "outside.example.test"] {
        assert_eq!(
            p.content()
                .explain_selection(
                    p.compiled(),
                    tcp(domain, valid, RequestTransport::Tcp),
                    observed()
                )
                .unwrap_err(),
            SimulationError::Scope
        );
    }
    assert_eq!(
        p.content()
            .explain_selection(
                p.compiled(),
                SimulationRequest::tcp(valid, "allowed.example.test", NonZeroU16::new(80).unwrap()),
                observed()
            )
            .unwrap_err(),
        SimulationError::Scope
    );
    let mut changed = document();
    changed["gatewayGrants"][0]["outbounds"] = json!([text(51)]);
    let excluded = publication(&changed, &store).unwrap();
    assert_eq!(
        excluded
            .content()
            .explain_selection(
                excluded.compiled(),
                tcp("allowed.example.test", valid, RequestTransport::Tcp),
                observed()
            )
            .unwrap_err(),
        SimulationError::Scope
    );
    let unknown = p
        .content()
        .explain_selection(
            p.compiled(),
            tcp("allowed.example.test", valid, RequestTransport::Tcp),
            vec![],
        )
        .unwrap();
    let mut deny_doc = document();
    deny_doc["profiles"][1]["defaultTrafficAction"] = json!({"kind":"deny"});
    let deny = publication(&deny_doc, &store).unwrap();
    assert_eq!(
        run(
            &deny,
            tcp("allowed.example.test", valid, RequestTransport::Tcp)
        )["resolved_intent"]["code"],
        "policy_denied"
    );
    assert_eq!(
        deny.content()
            .explain_selection(
                deny.compiled(),
                tcp(
                    "allowed.example.test",
                    GatewayNetworkIdentity {
                        source: "203.0.113.8".parse().unwrap(),
                        ..valid
                    },
                    RequestTransport::Tcp
                ),
                observed()
            )
            .unwrap_err(),
        SimulationError::Scope
    );
    assert_eq!(
        serde_json::to_value(unknown).unwrap()["resolved_intent"]["code"],
        "unknown_availability"
    );
}
#[test]
fn assumed_available_proxy_simulation_does_not_contact_a_passive_network_trap() {
    // Required negative proof only: a passive loopback trap observes any accidental
    // connect; simulation has no connector/probe interface and performs no request.
    let trap = TcpListener::bind("127.0.0.1:0").unwrap();
    trap.set_nonblocking(true).unwrap();
    let mut doc = document();
    doc["outbounds"][1]["enabled"] = true.into();
    doc["outbounds"][1]["transport"]["host"] = "127.0.0.1".into();
    doc["outbounds"][1]["transport"]["port"] = trap.local_addr().unwrap().port().into();
    doc["profiles"][1]["defaultTrafficAction"]["outbound"] = text(51).into();
    doc["gatewayGrants"][0]["outbounds"] = json!([text(51)]);
    let store = catalogs();
    let p = publication(&doc, &store).unwrap();
    let result = run(
        &p,
        tcp("allowed.example.test", identity(), RequestTransport::Tcp),
    );
    assert_eq!(result["resolved_intent"]["kind"], "protected_proxy");
    assert_eq!(
        trap.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("127.0.0.1")
    );
}
#[test]
fn intent_assumptions_readiness_and_private_record_types_are_explicit() {
    let store = catalogs();
    let mut doc = document();
    doc["outbounds"][1]["enabled"] = true.into();
    doc["privateRecords"].as_array_mut().unwrap().push(json!({"id":text(73),"profile":text(1),"name":"v6-private.example.test","ttl":30,"record":{"kind":"aaaa","address":"2001:db8::42"}}));
    let p = publication(&doc, &store).unwrap();
    assert_eq!(
        run(&p, dns("service.example.test"))["resolved_intent"]["kind"],
        "gateway"
    );
    let not_ready = serde_json::to_value(
        p.content()
            .explain_selection(
                p.compiled(),
                SimulationRequest::dns(binding(), "service.example.test"),
                observed(),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(not_ready["resolved_intent"]["code"], "gateway_not_ready");
    for (name, kind, forbidden) in [
        ("alias.example.test", "cname", "private.example.test"),
        ("v6-private.example.test", "aaaa", "2001:db8::42"),
    ] {
        let result = run(&p, dns(name));
        assert_eq!(result["private_owners"][0]["record_type"], kind);
        assert!(
            !serde_json::to_string(&result["private_owners"])
                .unwrap()
                .contains(forbidden)
        );
        assert_eq!(result["private_response_precedence"], "pending_p3");
    }
    for (availability, capability, code) in [
        (
            OutboundAvailability::Unavailable,
            TcpCapability::Supported,
            "unavailable_outbound",
        ),
        (
            OutboundAvailability::Unknown,
            TcpCapability::Supported,
            "unknown_availability",
        ),
        (
            OutboundAvailability::Available,
            TcpCapability::Unsupported,
            "unsupported_capability",
        ),
        (
            OutboundAvailability::Available,
            TcpCapability::Unknown,
            "unknown_capability",
        ),
    ] {
        let mut observations = observed();
        observations[0].availability = availability;
        observations[0].tcp = capability;
        let result = serde_json::to_value(
            p.content()
                .explain_selection(
                    p.compiled(),
                    SimulationRequest::tcp(
                        identity(),
                        "allowed.example.test",
                        NonZeroU16::new(443).unwrap(),
                    ),
                    observations,
                )
                .unwrap(),
        )
        .unwrap();
        assert_eq!(result["resolved_intent"]["code"], code);
        assert!(result["predicted_decision_point"].is_null());
    }
}
