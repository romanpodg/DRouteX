use super::*;
use droutex_common::{OutboundAvailability, ProxyOutboundKind};
use droutex_config::{OutboundObservation, ResourceData, snapshot::SnapshotResource};
use droutex_gateway::{listener, origin::resolver::OriginResolver};
use droutex_policy::TcpCapability;

fn publish(owner: &Deployment, doc: &Value, sequence: u64) {
    let resource = br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"udp_tcp","address":"1.1.1.1:53"}]}"#.to_vec();
    let checked = parse_configuration(
        &serde_json::to_vec(doc).unwrap(),
        InputFormat::Json,
        &ReferenceCatalog {
            upstreams: [typed(60)].into(),
            ..Default::default()
        },
    )
    .unwrap();
    let publication = CompletePublication::new(
        checked,
        vec![],
        &[ResourceData {
            resource: SnapshotResource::Upstream(typed(60)),
            bytes: &resource,
        }],
        &[],
    )
    .unwrap();
    let envelope = publication
        .encode(
            SnapshotMetadata {
                node: typed(1),
                snapshot: typed(2),
                source: typed(3),
                sequence: SnapshotSequence::new(sequence).unwrap(),
            },
            &keys().0,
        )
        .unwrap();
    owner
        .activate(
            envelope,
            Artifacts::new(vec![(SnapshotResource::Upstream(typed(60)), resource)]).unwrap(),
            &Cancellation::default(),
        )
        .unwrap();
}

async fn query_udp(address: SocketAddr, qtype: u16) -> Message {
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let mut query = QUERY.to_vec();
    let offset = query.len() - 4;
    query[offset..offset + 2].copy_from_slice(&qtype.to_be_bytes());
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        client.send_to(&query, address).await.unwrap();
        let mut bytes = [0; 1232];
        if let Ok(Ok((size, _))) =
            timeout(Duration::from_millis(100), client.recv_from(&mut bytes)).await
        {
            return Message::from_vec(&bytes[..size]).unwrap();
        }
        assert!(Instant::now() < until, "no DNS response");
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum AddressCase {
    Listener,
    Unknown,
    TrustedAlias,
}

async fn live_gateway_controls_dns_answers(
    change_generation: bool,
    ipv6: bool,
    stop_inventory: bool,
    wrong_port: bool,
    address_case: AddressCase,
) {
    let udp = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ingress = TcpListener::bind(if ipv6 { "[::1]:0" } else { "127.0.0.1:0" })
        .await
        .unwrap();
    let udp_address = udp.local_addr().unwrap();
    let tcp_address = tcp.local_addr().unwrap();
    let listener_port = ingress.local_addr().unwrap().port();
    let gateway_port = if wrong_port {
        if listener_port == u16::MAX {
            listener_port - 1
        } else {
            listener_port + 1
        }
    } else {
        listener_port
    };
    let mut doc = document(udp_address, tcp_address);
    let lease = doc["legacyGrants"][0]["lease"].clone();
    doc["profiles"][0]["defaultDnsAction"] = json!({"kind":"gateway","outbound":text(90)});
    doc["profiles"].as_array_mut().unwrap().push(json!({"id":text(11),"name":"TCP","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"outbound","outbound":text(90)}}));
    doc["networkGroups"][0]["trafficProfile"] = json!(text(11));
    doc["listeners"].as_array_mut().unwrap().push(json!({"id":text(22),"name":"Gateway","enabled":true,"bind":ingress.local_addr().unwrap().to_string(),"transport":"gateway_tcp"}));
    doc["gatewayGrants"] = json!([{"id":text(42),"source":"127.0.0.1","listener":text(22),"group":text(50),"targets":[{"mode":"exact","value":"example.test"}],"ports":[{"from":gateway_port,"to":gateway_port}],"outbounds":[text(90)],"lease":lease}]);
    doc["outbounds"] =
        json!([{"id":text(90),"name":"Direct","enabled":true,"transport":{"kind":"vps_direct"}}]);
    doc["originUpstream"] = json!(text(60));
    let ipv4 = if address_case == AddressCase::Listener {
        "127.0.0.1"
    } else {
        "127.0.0.2"
    };
    let ipv6_address = if address_case == AddressCase::Listener {
        "::1"
    } else {
        "::2"
    };
    doc["smartDnsRoutes"] = json!([{"endpoint":text(30),"grant":text(42),"domain":"example.test","port":gateway_port,"outbound":text(90),"addresses":[ipv4, ipv6_address],"ttl":30}]);
    let node = DnsNode::new(Deployment::in_memory(typed(1), keys().1));
    publish(node.deployment(), &doc, 1);
    let owner = Arc::clone(node.deployment());
    let (gateway_stop, shutdown) = watch::channel(false);
    let (ready, receiver) = oneshot::channel();
    let aliases = if address_case == AddressCase::TrustedAlias {
        vec![if ipv6 { ipv6_address } else { ipv4 }.parse().unwrap()]
    } else {
        vec![]
    };
    let resolver = Arc::new(OriginResolver::new(aliases).unwrap());
    let gateway = tokio::spawn(listener::serve_preview_ready(
        Arc::clone(&owner),
        vec![listener::ListenerSocket {
            id: typed(22),
            socket: ingress,
        }],
        shutdown,
        Arc::clone(&resolver),
        vec![OutboundObservation {
            id: typed(90),
            kind: ProxyOutboundKind::GatewayDirect,
            availability: OutboundAvailability::Available,
            tcp: TcpCapability::Supported,
        }],
        ready,
    ));
    let readiness = timeout(Duration::from_secs(3), receiver)
        .await
        .unwrap()
        .unwrap();
    let (dns_stop, shutdown) = watch::channel(false);
    let dns = start_runtime(
        std::future::ready(()),
        node.serve_with_gateway(
            vec![
                ListenerSocket::Udp {
                    id: typed(20),
                    socket: udp,
                },
                ListenerSocket::Tcp {
                    id: typed(21),
                    socket: tcp,
                },
            ],
            vec![],
            shutdown,
            readiness.clone(),
        ),
    )
    .await;
    let supported = if ipv6 { 28 } else { 1 };
    let unsupported = if ipv6 { 1 } else { 28 };
    let expected = if ipv6 {
        RData::AAAA(hickory_proto::rr::rdata::AAAA(
            ipv6_address.parse().unwrap(),
        ))
    } else {
        RData::A(A(ipv4.parse::<Ipv4Addr>().unwrap()))
    };
    let reply = query_udp(udp_address, supported).await;
    if wrong_port || address_case == AddressCase::Unknown {
        assert_eq!(reply.metadata.response_code, ResponseCode::ServFail);
        gateway_stop.send_replace(true);
        timeout(Duration::from_secs(3), gateway)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        dns_stop.send_replace(true);
        timeout(Duration::from_secs(7), dns)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        return;
    }
    assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
    assert_eq!(reply.answers[0].data, expected);
    assert_eq!(reply.answers[0].ttl, 30);
    assert!(!reply.metadata.authentic_data);
    let other = Arc::new(Deployment::in_memory(typed(1), keys().1));
    publish(&other, &doc, 1);
    let other_refresh = Arc::new(crate::clock::Refresh::new());
    let (other_stop, other_shutdown) = watch::channel(false);
    let other_clock = tokio::spawn(Arc::clone(&other_refresh).run(
        Arc::clone(&other),
        Arc::new(crate::budget::Budget::new()),
        other_shutdown,
    ));
    sleep(Duration::from_millis(150)).await;
    let other_auth = runtime::fixture_authorize(&other, &other_refresh, typed(20), udp_address)
        .await
        .unwrap();
    let other_binding = other_auth
        .smart_dns_binding("example.test", typed(90))
        .unwrap();
    assert!(
        !readiness.matches(&other_binding),
        "matching metadata from another owner must not grant readiness"
    );
    drop(other_auth);
    other_stop.send_replace(true);
    other_clock.await.unwrap().unwrap();
    for kind in [unsupported, 16, 65] {
        assert_eq!(
            query_udp(udp_address, kind).await.metadata.response_code,
            ResponseCode::ServFail
        );
    }
    sleep(Duration::from_millis(150)).await;
    let mut stream = TcpStream::connect(tcp_address).await.unwrap();
    let mut tcp_query = QUERY.to_vec();
    let qtype = tcp_query.len() - 4;
    tcp_query[qtype..qtype + 2].copy_from_slice(&supported.to_be_bytes());
    frame(&mut stream, &tcp_query).await;
    assert_eq!(read_reply(&mut stream).await.answers[0].data, expected);
    if stop_inventory {
        resolver.shutdown_inventory().await.unwrap();
        assert_eq!(
            query_udp(udp_address, supported)
                .await
                .metadata
                .response_code,
            ResponseCode::ServFail
        );
    }
    if change_generation {
        publish(&owner, &doc, 2);
        assert_eq!(
            query_udp(udp_address, supported)
                .await
                .metadata
                .response_code,
            ResponseCode::ServFail
        );
    }
    if !change_generation && !stop_inventory {
        sleep(Duration::from_millis(150)).await;
        let refresh = Arc::new(crate::clock::Refresh::new());
        let auth = runtime::fixture_authorize(&owner, &refresh, typed(20), udp_address)
            .await
            .unwrap();
        let reply = wire::Reply {
            message: wire::response(&Message::from_vec(QUERY).unwrap(), ResponseCode::NoError),
            expires: None,
            gateway: Some(wire::GatewayCheck {
                name: "example.test".into(),
                outbound: typed(90),
                readiness,
            }),
        };
        let entered = Notify::new();
        let release = Notify::new();
        let sent = AtomicUsize::new(0);
        let operation = async {
            entered.notify_one();
            release.notified().await;
            sent.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };
        let send = runtime::submit_reply(
            &auth,
            &reply,
            &refresh,
            Instant::now() + Duration::from_secs(2),
            operation,
        );
        let withdraw = async {
            entered.notified().await;
            gateway_stop.send_replace(true);
            release.notify_one();
        };
        let (result, ()) = tokio::join!(send, withdraw);
        assert!(
            result.is_err(),
            "withdrawn gateway must prevent a pending submission"
        );
        assert_eq!(sent.load(Ordering::SeqCst), 0);
    } else {
        gateway_stop.send_replace(true);
    }
    timeout(Duration::from_secs(3), gateway)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        query_udp(udp_address, supported)
            .await
            .metadata
            .response_code,
        ResponseCode::ServFail
    );
    dns_stop.send_replace(true);
    timeout(Duration::from_secs(7), dns)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_gateway_generation_change_withdraws_dns_answers() {
    live_gateway_controls_dns_answers(true, false, false, false, AddressCase::Listener).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_gateway_shutdown_withdraws_dns_answers() {
    live_gateway_controls_dns_answers(false, false, false, false, AddressCase::Listener).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_ipv6_gateway_only_advertises_its_address_family() {
    live_gateway_controls_dns_answers(true, true, false, false, AddressCase::Listener).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inventory_shutdown_withdraws_gateway_dns_answers() {
    live_gateway_controls_dns_answers(false, false, true, false, AddressCase::Listener).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mismatched_listener_port_never_produces_a_gateway_address() {
    live_gateway_controls_dns_answers(false, false, false, true, AddressCase::Listener).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unregistered_gateway_address_never_produces_a_dns_answer() {
    live_gateway_controls_dns_answers(false, false, false, false, AddressCase::Unknown).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_declared_gateway_alias_can_be_advertised() {
    live_gateway_controls_dns_answers(true, false, false, false, AddressCase::TrustedAlias).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unregistered_ipv6_gateway_address_never_produces_a_dns_answer() {
    live_gateway_controls_dns_answers(false, true, false, false, AddressCase::Unknown).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_declared_ipv6_gateway_alias_can_be_advertised() {
    live_gateway_controls_dns_answers(true, true, false, false, AddressCase::TrustedAlias).await;
}
