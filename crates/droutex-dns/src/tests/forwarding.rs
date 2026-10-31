use super::*;
use droutex_common::DnsUpstreamId;
use droutex_config::{ResourceData, snapshot::SnapshotResource, upstream::Transport};
use std::collections::BTreeSet;
#[path = "../../../../tests/dns-fixtures/records.rs"]
mod record_vectors;

fn apply_forwarding(
    owner: &Deployment,
    document: &Value,
    transports: &[Transport],
    fallback: &str,
    sequence: u64,
) {
    let ipv6 = document["listeners"][0]["bind"]
        .as_str()
        .unwrap()
        .parse::<SocketAddr>()
        .unwrap()
        .is_ipv6();
    let resource = serde_json::to_vec(
        &json!({"format":"droutex-dns-upstream-v1", "fallback":fallback,
        "members":transports.iter().enumerate().map(|(n, transport)| json!({
            "transport":match transport { Transport::UdpTcp=>"udp_tcp", Transport::Tcp=>"tcp" },
            "address":match (ipv6,n) {
                (false,0)=>"1.1.1.1:53", (false,_)=>"8.8.8.8:53",
                (true,0)=>"[2606:4700:4700::1111]:53", (true,_)=>"[2001:4860:4860::8888]:53",
            }
        })).collect::<Vec<_>>() }),
    )
    .unwrap();
    let checked = parse_configuration(
        &serde_json::to_vec(document).unwrap(),
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

async fn server(
    ip: IpAddr,
    destinations: Vec<(Transport, SocketAddr)>,
    fallback: &str,
    inventory: Arc<Mutex<Result<BTreeSet<IpAddr>>>>,
) -> Server {
    server_delayed(
        ip,
        destinations,
        fallback,
        inventory,
        Duration::ZERO,
        Duration::ZERO,
    )
    .await
}
async fn server_delayed(
    ip: IpAddr,
    destinations: Vec<(Transport, SocketAddr)>,
    fallback: &str,
    inventory: Arc<Mutex<Result<BTreeSet<IpAddr>>>>,
    cleanup_delay: Duration,
    tc_delay: Duration,
) -> Server {
    let udp = UdpSocket::bind((ip, 0)).await.unwrap();
    let tcp = TcpListener::bind((ip, 0)).await.unwrap();
    let udp_addr = udp.local_addr().unwrap();
    let tcp_addr = tcp.local_addr().unwrap();
    let mut doc = document(udp_addr, tcp_addr);
    doc["profiles"][0]["defaultDnsAction"] = json!({"kind":"resolve", "upstream":text(60)});
    let owner = Arc::new(Deployment::in_memory(typed(1), keys().1));
    apply_forwarding(
        &owner,
        &doc,
        &destinations
            .iter()
            .map(|(kind, _)| *kind)
            .collect::<Vec<_>>(),
        fallback,
        1,
    );
    let mut processor =
        crate::forward::fixture(vec![(typed::<DnsUpstreamId>(60), destinations)], inventory)
            .unwrap();
    processor.fixture_cleanup_delay(cleanup_delay);
    processor.fixture_tc_delay(tc_delay);
    let (stop, shutdown) = watch::channel(false);
    let handle = start_runtime(
        std::future::ready(()),
        runtime::serve(
            Arc::clone(&owner),
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
            shutdown,
            processor,
        ),
    )
    .await;
    // Replenish the existing real monotonic rate buckets after runtime startup.
    // No query has been submitted; this does not retry an upstream operation.
    sleep(Duration::from_millis(100)).await;
    Server {
        owner,
        doc,
        udp: udp_addr,
        tcp: tcp_addr,
        stop,
        handle,
    }
}

fn answer(query: &[u8], code: u8, truncated: bool) -> Vec<u8> {
    // Independent fixture bytes: locate the original question's root, excluding OPT.
    let mut end = 12;
    while query[end] != 0 {
        end += usize::from(query[end]) + 1;
    }
    end += 5;
    let mut response = query[..end].to_vec();
    response[2] = 0x85 | if truncated { 2 } else { 0 }; // QR,RD,AA; AD below is deliberately untrusted.
    response[3] = 0xa0 | code;
    response[6..12].fill(0);
    if code == 0 && !truncated {
        response[7] = 1;
        response
            .extend_from_slice(b"\xc0\x0c\x00\x01\x00\x01\x00\x00\x00\x3c\x00\x04\xcb\x00\x71\x09");
    }
    response
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn udp_forwarding_uses_fresh_query_and_preserves_only_clean_client_semantics() {
    let upstream = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let fixture = tokio::spawn(async move {
        let mut bytes = [0; 4097];
        let (length, peer) = timeout(Duration::from_secs(3), upstream.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        let request = wire::parse(&bytes[..length]).unwrap();
        assert!(request.metadata.recursion_desired);
        assert!(!request.metadata.checking_disabled);
        assert_eq!(request.edns.as_ref().unwrap().max_payload(), 1232);
        assert!(request.edns.as_ref().unwrap().options().as_ref().is_empty());
        upstream
            .send_to(&answer(&bytes[..length], 0, false), peer)
            .await
            .unwrap();
        assert!(
            timeout(Duration::from_millis(150), upstream.recv_from(&mut bytes))
                .await
                .is_err(),
            "upstream datagram was retransmitted"
        );
    });
    let server = server(
        v4(),
        vec![(Transport::UdpTcp, address)],
        "none",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
    )
    .await;
    let mut original = QUERY.to_vec();
    original[2] &= !1; // The upstream still receives RD=true.
    original[11] = 1;
    original.extend_from_slice(
        b"\x00\x00\x29\x10\x00\x00\x00\x00\x00\x00\x0b\x00\x08\x00\x07\x00\x01\x18\x00\xcb\x00\x71",
    );
    let reply = exchange_once(&server, &original)
        .await
        .expect("no forwarded response");
    let reply = Message::from_vec(&reply).unwrap();
    assert_eq!(reply.metadata.id, 1234);
    assert_eq!(reply.queries, wire::parse(QUERY).unwrap().queries);
    assert_eq!(reply.answers.len(), 1);
    assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
    assert!(reply.metadata.recursion_available);
    assert!(!reply.metadata.recursion_desired);
    assert!(!reply.metadata.authoritative);
    assert!(!reply.metadata.authentic_data);
    fixture.await.unwrap();
    assert_eq!(server.close().await.answered, 1);
}

async fn upstream_tcp_query(stream: &mut TcpStream) -> Vec<u8> {
    let mut prefix = [0; 2];
    timeout(Duration::from_secs(2), stream.read_exact(&mut prefix))
        .await
        .unwrap()
        .unwrap();
    let length = usize::from(u16::from_be_bytes(prefix));
    assert!(length <= 4096);
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await.unwrap();
    bytes
}
async fn upstream_tcp_answer(mut stream: TcpStream) {
    let query = upstream_tcp_query(&mut stream).await;
    let decoded = wire::parse(&query).unwrap();
    assert!(decoded.metadata.recursion_desired);
    assert_eq!(decoded.edns.as_ref().unwrap().max_payload(), 1232);
    let reply = answer(&query, 0, false);
    let prefix = (reply.len() as u16).to_be_bytes();
    stream.write_all(&prefix[..1]).await.unwrap();
    tokio::task::yield_now().await;
    stream.write_all(&prefix[1..]).await.unwrap();
    stream.write_all(&reply[..13]).await.unwrap();
    tokio::task::yield_now().await;
    stream.write_all(&reply[13..]).await.unwrap();
    let mut extra = [0; 1];
    assert_eq!(
        timeout(Duration::from_secs(2), stream.read(&mut extra))
            .await
            .unwrap()
            .unwrap(),
        0,
        "TCP owner did not close after one query"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_forwarding_joins_one_shot_upstream_before_returning_clean_answer() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let fixture = tokio::spawn(async move {
        let (stream, _) = timeout(Duration::from_secs(3), upstream.accept())
            .await
            .unwrap()
            .unwrap();
        upstream_tcp_answer(stream).await;
        assert!(
            timeout(Duration::from_millis(100), upstream.accept())
                .await
                .is_err(),
            "reconnected one-shot upstream"
        );
    });
    let server = server(
        v4(),
        vec![(Transport::Tcp, address)],
        "none",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
    )
    .await;
    let mut client = TcpStream::connect(server.tcp).await.unwrap();
    frame(&mut client, QUERY).await;
    let reply = read_reply(&mut client).await;
    assert_eq!(reply.metadata.id, 1234);
    assert_eq!(reply.answers.len(), 1);
    assert!(reply.metadata.recursion_available);
    assert!(!reply.metadata.authentic_data);
    assert!(!reply.metadata.authoritative);
    drop(client);
    fixture.await.unwrap();
    assert_eq!(server.close().await.answered, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validated_udp_tc_uses_tcp_at_same_pinned_target_without_datagram_retry() {
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp.local_addr().unwrap();
    let udp = UdpSocket::bind(address).await.unwrap();
    let datagram = tokio::spawn(async move {
        let mut bytes = [0; 4097];
        let (length, peer) = timeout(Duration::from_secs(3), udp.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        udp.send_to(&answer(&bytes[..length], 0, true), peer)
            .await
            .unwrap();
        assert!(
            timeout(Duration::from_millis(300), udp.recv_from(&mut bytes))
                .await
                .is_err()
        );
    });
    let stream = tokio::spawn(async move {
        let (stream, _) = timeout(Duration::from_secs(3), tcp.accept())
            .await
            .unwrap()
            .unwrap();
        upstream_tcp_answer(stream).await;
    });
    let server = server(
        v4(),
        vec![(Transport::UdpTcp, address)],
        "none",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
    )
    .await;
    let bytes = exchange_once(&server, QUERY).await.unwrap();
    let reply = Message::from_vec(&bytes).unwrap();
    assert_eq!(reply.answers.len(), 1);
    assert!(!reply.metadata.truncation);
    datagram.await.unwrap();
    stream.await.unwrap();
    assert_eq!(server.close().await.answered, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_after_actual_partial_write_preserves_the_original_io_progress() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    let auth = server
        .owner
        .prepare_legacy_dns(typed(20), v4())
        .unwrap()
        .consume()
        .unwrap();
    crate::forward::partial_write_checkpoint_fixture(
        auth.dns_io_check().unwrap(),
        Arc::clone(&server.owner),
    )
    .await;
    drop(auth);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_fallback_keeps_rcode_order_and_never_retries_negative_answers() {
    for (code, fallback, expected, second_used) in [
        (0, "on_failure", ResponseCode::NoError, false),
        (3, "on_failure", ResponseCode::NXDomain, false),
        (2, "on_failure", ResponseCode::NoError, true),
        (5, "on_failure", ResponseCode::NoError, true),
        (5, "none", ResponseCode::Refused, false),
        (2, "none", ResponseCode::ServFail, false),
    ] {
        let first = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let second = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let first_address = first.local_addr().unwrap();
        let second_address = second.local_addr().unwrap();
        let order = Arc::new(AtomicUsize::new(0));
        let first_order = Arc::clone(&order);
        let first = tokio::spawn(async move {
            let mut bytes = [0; 4097];
            let (length, peer) = timeout(Duration::from_secs(3), first.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(first_order.fetch_add(1, Ordering::Relaxed), 0);
            first
                .send_to(&answer(&bytes[..length], code, false), peer)
                .await
                .unwrap();
            assert!(
                timeout(Duration::from_millis(100), first.recv_from(&mut bytes))
                    .await
                    .is_err()
            );
        });
        let second_order = Arc::clone(&order);
        let second = tokio::spawn(async move {
            let mut bytes = [0; 4097];
            let received = timeout(Duration::from_millis(800), second.recv_from(&mut bytes)).await;
            if second_used {
                let (length, peer) = received.unwrap().unwrap();
                assert_eq!(second_order.fetch_add(1, Ordering::Relaxed), 1);
                second
                    .send_to(&answer(&bytes[..length], 0, false), peer)
                    .await
                    .unwrap();
            } else {
                assert!(
                    received.is_err(),
                    "negative/disabled fallback contacted another member"
                );
            }
        });
        let destinations = if fallback == "none" {
            vec![(Transport::UdpTcp, first_address)]
        } else {
            vec![
                (Transport::UdpTcp, first_address),
                (Transport::UdpTcp, second_address),
            ]
        };
        let server = server(
            v4(),
            destinations,
            fallback,
            Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        )
        .await;
        let bytes = exchange_once(&server, QUERY).await.unwrap();
        assert_eq!(
            Message::from_vec(&bytes).unwrap().metadata.response_code,
            expected,
            "code={code}, fallback={fallback}"
        );
        first.await.unwrap();
        second.await.unwrap();
        assert_eq!(
            order.load(Ordering::Relaxed),
            if second_used { 2 } else { 1 }
        );
        assert_eq!(server.close().await.answered, 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn current_self_inventory_and_inventory_failure_refuse_all_upstream_io() {
    for initial in [Ok([v4()].into()), Err(DnsError(ErrorCode::Unavailable))] {
        let first = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let second = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = server(
            v4(),
            vec![
                (Transport::UdpTcp, first.local_addr().unwrap()),
                (Transport::UdpTcp, second.local_addr().unwrap()),
            ],
            "on_failure",
            Arc::new(Mutex::new(initial)),
        )
        .await;
        let reply = exchange_once(&server, QUERY).await.unwrap();
        assert_eq!(
            Message::from_vec(&reply).unwrap().metadata.response_code,
            ResponseCode::ServFail
        );
        let mut bytes = [0; 4097];
        assert!(
            timeout(Duration::from_millis(100), first.recv_from(&mut bytes))
                .await
                .is_err()
        );
        assert!(
            timeout(Duration::from_millis(100), second.recv_from(&mut bytes))
                .await
                .is_err()
        );
        assert_eq!(server.close().await.answered, 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inventory_change_is_rechecked_before_the_next_admitted_send() {
    let upstream = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let inventory = Arc::new(Mutex::new(Ok(BTreeSet::new())));
    let server = server(
        v4(),
        vec![(Transport::UdpTcp, address)],
        "none",
        Arc::clone(&inventory),
    )
    .await;
    let fixture = tokio::spawn(async move {
        let mut bytes = [0; 4097];
        let (length, peer) = timeout(Duration::from_secs(3), upstream.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        upstream
            .send_to(&answer(&bytes[..length], 0, false), peer)
            .await
            .unwrap();
        upstream
    });
    let first = exchange_once(&server, QUERY).await.unwrap();
    assert_eq!(Message::from_vec(&first).unwrap().answers.len(), 1);
    let upstream = fixture.await.unwrap();
    *inventory.lock().unwrap() = Ok([v4()].into());
    let second = exchange_once(&server, QUERY).await.unwrap();
    assert_eq!(
        Message::from_vec(&second).unwrap().metadata.response_code,
        ResponseCode::ServFail
    );
    let mut bytes = [0; 4097];
    assert!(
        timeout(Duration::from_millis(100), upstream.recv_from(&mut bytes))
            .await
            .is_err()
    );
    assert_eq!(server.close().await.answered, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delayed_join_cannot_extend_member_response_acceptance_deadline() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let fixture = tokio::spawn(async move {
        let (stream, _) = timeout(Duration::from_secs(3), upstream.accept())
            .await
            .unwrap()
            .unwrap();
        upstream_tcp_answer(stream).await;
    });
    let server = server_delayed(
        v4(),
        vec![(Transport::Tcp, address)],
        "none",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        Duration::from_millis(550),
        Duration::ZERO,
    )
    .await;
    let bytes = exchange_once(&server, QUERY).await.unwrap();
    assert_eq!(
        Message::from_vec(&bytes).unwrap().metadata.response_code,
        ResponseCode::ServFail,
        "cleanup extended the original500ms member acceptance deadline"
    );
    fixture.await.unwrap();
    assert_eq!(server.close().await.answered, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_and_cancellation_wake_both_worker_and_tcp_background_roles() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    for cancel in [false, true] {
        let auth = server
            .owner
            .prepare_legacy_dns(typed(20), v4())
            .unwrap()
            .consume()
            .unwrap();
        crate::forward::shared_waiter_fixture(
            auth.dns_io_check().unwrap(),
            Arc::clone(&server.owner),
            cancel,
        )
        .await;
    }
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn extended_upstream_rcode_never_becomes_a_false_noerror_for_legacy_client() {
    for client_edns in [false, true] {
        let upstream = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = upstream.local_addr().unwrap();
        let fixture = tokio::spawn(async move {
            let mut bytes = [0; 4097];
            let (length, peer) = timeout(Duration::from_secs(3), upstream.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            let mut response = answer(&bytes[..length], 2, false); // No answer RRs.
            response[3] = 0x80; // Low RCODE=0; high RCODE=1 in the independent OPT.
            response[11] = 1;
            response.extend_from_slice(b"\x00\x00\x29\x04\xd0\x01\x00\x00\x00\x00\x00");
            upstream.send_to(&response, peer).await.unwrap();
        });
        let server = server(
            v4(),
            vec![(Transport::UdpTcp, address)],
            "none",
            Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        )
        .await;
        let mut request = QUERY.to_vec();
        if client_edns {
            request[11] = 1;
            request.extend_from_slice(b"\x00\x00\x29\x04\xd0\x00\x00\x00\x00\x00\x00");
        }
        let bytes = exchange_once(&server, &request).await.unwrap();
        let response = Message::from_vec(&bytes).unwrap();
        assert_eq!(
            u16::from(response.metadata.response_code),
            if client_edns { 16 } else { 2 },
            "extended RCODE was misrepresented, client_edns={client_edns}"
        );
        assert!(response.answers.is_empty());
        fixture.await.unwrap();
        assert_eq!(server.close().await.answered, 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_tc_continuation_uses_explicit_next_member_without_late_tcp() {
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp.local_addr().unwrap();
    let udp = UdpSocket::bind(address).await.unwrap();
    let next = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let next_address = next.local_addr().unwrap();
    let first = tokio::spawn(async move {
        let mut bytes = [0; 4097];
        let (length, peer) = timeout(Duration::from_secs(3), udp.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        udp.send_to(&answer(&bytes[..length], 0, true), peer)
            .await
            .unwrap();
        assert!(
            timeout(Duration::from_millis(800), tcp.accept())
                .await
                .is_err(),
            "TCP started after member deadline"
        );
    });
    let second = tokio::spawn(async move {
        let mut bytes = [0; 4097];
        let (length, peer) = timeout(Duration::from_secs(3), next.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        next.send_to(&answer(&bytes[..length], 0, false), peer)
            .await
            .unwrap();
    });
    let server = server_delayed(
        v4(),
        vec![
            (Transport::UdpTcp, address),
            (Transport::UdpTcp, next_address),
        ],
        "on_failure",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        Duration::ZERO,
        Duration::from_millis(550),
    )
    .await;
    let reply = exchange_once(&server, QUERY).await.unwrap();
    assert_eq!(
        Message::from_vec(&reply).unwrap().metadata.response_code,
        ResponseCode::NoError,
        "member timeout bypassed explicit fallback"
    );
    first.await.unwrap();
    second.await.unwrap();
    assert_eq!(server.close().await.answered, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_before_first_socket_poll_rechecks_changed_self_inventory() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    for tcp in [false, true] {
        let auth = server
            .owner
            .prepare_legacy_dns(typed(20), v4())
            .unwrap()
            .consume()
            .unwrap();
        crate::forward::inventory_checkpoint_fixture(
            auth.dns_io_check().unwrap(),
            Arc::clone(&server.owner),
            tcp,
        )
        .await;
    }
    server.close().await;
}

fn record_answer(query: &[u8], case: &record_vectors::RecordCase) -> Vec<u8> {
    let mut reply = answer(query, 2, false);
    reply[3] = 0x80;
    reply[7] = 1;
    reply.extend_from_slice(b"\xc0\x0c");
    reply.extend_from_slice(&case.code.to_be_bytes());
    reply.extend_from_slice(b"\x00\x01\x00\x00\x00\x3c");
    reply.extend_from_slice(&(case.data.len() as u16).to_be_bytes());
    reply.extend_from_slice(case.data);
    reply
}
async fn ordinary_records(ip: IpAddr, transport: Transport) {
    let udp = UdpSocket::bind((ip, 0)).await.unwrap();
    let tcp = TcpListener::bind((ip, 0)).await.unwrap();
    let address = match transport {
        Transport::UdpTcp => udp.local_addr().unwrap(),
        Transport::Tcp => tcp.local_addr().unwrap(),
    };
    let fixture = tokio::spawn(async move {
        for case in record_vectors::record_cases() {
            for _ in 0..2 {
                match transport {
                    Transport::UdpTcp => {
                        let mut bytes = [0; 4097];
                        let (length, peer) =
                            timeout(Duration::from_secs(3), udp.recv_from(&mut bytes))
                                .await
                                .unwrap()
                                .unwrap();
                        assert_eq!(
                            wire::parse(&bytes[..length]).unwrap().queries[0].query_type,
                            hickory_proto::rr::RecordType::from(case.code)
                        );
                        udp.send_to(&record_answer(&bytes[..length], &case), peer)
                            .await
                            .unwrap();
                    }
                    Transport::Tcp => {
                        let (mut socket, _) = timeout(Duration::from_secs(3), tcp.accept())
                            .await
                            .unwrap()
                            .unwrap();
                        let query = upstream_tcp_query(&mut socket).await;
                        assert_eq!(
                            wire::parse(&query).unwrap().queries[0].query_type,
                            hickory_proto::rr::RecordType::from(case.code)
                        );
                        let reply = record_answer(&query, &case);
                        socket.write_u16(reply.len() as u16).await.unwrap();
                        socket.write_all(&reply).await.unwrap();
                        let mut extra = [0; 1];
                        assert_eq!(
                            timeout(Duration::from_secs(2), socket.read(&mut extra))
                                .await
                                .unwrap()
                                .unwrap(),
                            0
                        );
                    }
                }
            }
        }
    });
    let server = server(
        ip,
        vec![(transport, address)],
        "none",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
    )
    .await;
    sleep(Duration::from_millis(200)).await; // Enough existing group rate credits for22 distinct requests.
    let mut stream = TcpStream::connect(server.tcp).await.unwrap();
    for case in record_vectors::record_cases() {
        let mut query = QUERY.to_vec();
        let offset = query.len() - 4;
        query[offset..offset + 2].copy_from_slice(&case.code.to_be_bytes());
        frame(&mut stream, &query).await;
        let tcp_reply = read_reply(&mut stream).await;
        let bytes = exchange_once(&server, &query).await.unwrap();
        assert!(bytes.len() <= 3 * query.len());
        let udp_reply = Message::from_vec(&bytes).unwrap();
        for reply in [tcp_reply, udp_reply] {
            assert_eq!(reply.metadata.id, 1234);
            assert_eq!(
                reply.metadata.response_code,
                ResponseCode::NoError,
                "{transport:?} type{}",
                case.code
            );
            assert_eq!(reply.queries, wire::parse(&query).unwrap().queries);
            assert_eq!(reply.answers.len(), 1, "{transport:?} type{}", case.code);
            assert_eq!(
                reply.answers[0].record_type(),
                hickory_proto::rr::RecordType::from(case.code)
            );
            assert_eq!(reply.answers[0].data.to_string().trim(), case.expected);
            assert!(!reply.metadata.authentic_data);
            assert!(!reply.metadata.authoritative);
            assert!(reply.metadata.recursion_available);
        }
    }
    drop(stream);
    fixture.await.unwrap();
    assert_eq!(server.close().await.answered, 22);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_records_use_udp_upstream_ipv4() {
    ordinary_records(v4(), Transport::UdpTcp).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_records_use_tcp_upstream_ipv4() {
    ordinary_records(v4(), Transport::Tcp).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_records_use_udp_upstream_ipv6() {
    ordinary_records("::1".parse().unwrap(), Transport::UdpTcp).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_records_use_tcp_upstream_ipv6() {
    ordinary_records("::1".parse().unwrap(), Transport::Tcp).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declared_max_question_count_is_refused_and_next_admission_still_works() {
    macro_rules! layout {
        ($type:ty) => {
            (std::mem::size_of::<$type>(), std::mem::align_of::<$type>())
        };
    }
    println!(
        "D53 nested layouts RecordType={:?}; EDNS tuple={:?}; SVCB tuple={:?}; result={:?}; NetError={:?}",
        layout!(hickory_proto::rr::RecordType),
        layout!((
            hickory_proto::rr::rdata::opt::EdnsCode,
            hickory_proto::rr::rdata::opt::EdnsOption
        )),
        layout!((
            hickory_proto::rr::rdata::svcb::SvcParamKey,
            hickory_proto::rr::rdata::svcb::SvcParamValue
        )),
        layout!(std::result::Result<hickory_proto::op::DnsResponse, hickory_net::NetError>),
        layout!(hickory_net::NetError)
    );
    assert_eq!(
        [
            layout!(hickory_proto::rr::RecordType),
            layout!((
                hickory_proto::rr::rdata::opt::EdnsCode,
                hickory_proto::rr::rdata::opt::EdnsOption
            )),
            layout!((
                hickory_proto::rr::rdata::svcb::SvcParamKey,
                hickory_proto::rr::rdata::svcb::SvcParamValue
            )),
            layout!(std::result::Result<hickory_proto::op::DnsResponse, hickory_net::NetError>),
            layout!(hickory_net::NetError),
        ],
        [(4, 2), (40, 8), (40, 8), (176, 8), (88, 8)],
        "recalculate the nested D53 allocation profile"
    );
    let sizes = [
        std::mem::size_of::<hickory_proto::op::Query>(),
        std::mem::size_of::<Record>(),
        std::mem::size_of::<Message>(),
        std::mem::size_of::<hickory_proto::op::DnsResponse>(),
    ];
    assert_eq!(
        [
            std::mem::align_of::<hickory_proto::op::Query>(),
            std::mem::align_of::<Record>(),
            std::mem::align_of::<Message>(),
            std::mem::align_of::<hickory_proto::op::DnsResponse>(),
        ],
        [8; 4],
        "recalculate pinned D53 layouts"
    );
    // Freeze the reviewed pinned x86_64 layouts on both required CI platforms.
    // Any change requires recalculating the untrusted u16 allocation envelope.
    assert_eq!(
        sizes,
        [88, 272, 152, 176],
        "recalculate the D53 library allocation profile"
    );
    println!(
        "D53 pinned x86_64 type sizes Query/Record/Message/DnsResponse={sizes:?}; one u16 question-vector requested bytes={}; eight slots={}",
        usize::from(u16::MAX) * sizes[0],
        8 * usize::from(u16::MAX) * sizes[0]
    );
    let upstream = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let fixture = tokio::spawn(async move {
        let mut bytes = [0; 4097];
        let (_, peer) = timeout(Duration::from_secs(3), upstream.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        let mut malformed = b"\x00\x00\x81\x80\xff\xff\x00\x00\x00\x00\x00\x00".to_vec();
        malformed[..2].copy_from_slice(&bytes[..2]);
        upstream.send_to(&malformed, peer).await.unwrap();
        let (length, peer) = timeout(Duration::from_secs(3), upstream.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        upstream
            .send_to(&answer(&bytes[..length], 0, false), peer)
            .await
            .unwrap();
    });
    let server = server(
        v4(),
        vec![(Transport::UdpTcp, address)],
        "none",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
    )
    .await;
    let first = exchange_once(&server, QUERY).await.unwrap();
    assert_eq!(
        Message::from_vec(&first).unwrap().metadata.response_code,
        ResponseCode::ServFail
    );
    let next = exchange_once(&server, QUERY).await.unwrap();
    assert_eq!(Message::from_vec(&next).unwrap().answers.len(), 1);
    fixture.await.unwrap();
    assert_eq!(server.close().await.answered, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generation_or_revocation_during_each_attempt_prevents_reply_and_fallback() {
    for transport in [Transport::UdpTcp, Transport::Tcp] {
        for active_member in 0..2 {
            for revoke in [false, true] {
                let first = UdpSocket::bind("127.0.0.1:0").await.unwrap();
                let udp = UdpSocket::bind("127.0.0.1:0").await.unwrap();
                let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let target = match transport {
                    Transport::UdpTcp => udp.local_addr().unwrap(),
                    Transport::Tcp => tcp.local_addr().unwrap(),
                };
                let destinations = if active_member == 0 {
                    vec![
                        (transport, target),
                        (Transport::UdpTcp, first.local_addr().unwrap()),
                    ]
                } else {
                    vec![
                        (Transport::UdpTcp, first.local_addr().unwrap()),
                        (transport, target),
                    ]
                };
                let server = server(
                    v4(),
                    destinations,
                    "on_failure",
                    Arc::new(Mutex::new(Ok(BTreeSet::new()))),
                )
                .await;
                let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
                client.send_to(QUERY, server.udp).await.unwrap();
                let mut bytes = [0; 4097];
                if active_member == 1 {
                    let (length, peer) =
                        timeout(Duration::from_secs(2), first.recv_from(&mut bytes))
                            .await
                            .unwrap()
                            .unwrap();
                    first
                        .send_to(&answer(&bytes[..length], 2, false), peer)
                        .await
                        .unwrap();
                }
                let (query, datagram, mut stream) = match transport {
                    Transport::UdpTcp => {
                        let (length, peer) =
                            timeout(Duration::from_secs(2), udp.recv_from(&mut bytes))
                                .await
                                .unwrap()
                                .unwrap();
                        (bytes[..length].to_vec(), Some(peer), None)
                    }
                    Transport::Tcp => {
                        let (mut stream, _) = timeout(Duration::from_secs(2), tcp.accept())
                            .await
                            .unwrap()
                            .unwrap();
                        let query = upstream_tcp_query(&mut stream).await;
                        // Suspend the actual framing read with half its length prefix received.
                        stream.write_all(&[0]).await.unwrap();
                        (query, None, Some(stream))
                    }
                };
                let mut replacement = server.doc.clone();
                if revoke {
                    replacement["listeners"][0]["enabled"] = json!(false);
                }
                let transports = if active_member == 0 {
                    [transport, Transport::UdpTcp]
                } else {
                    [Transport::UdpTcp, transport]
                };
                apply_forwarding(&server.owner, &replacement, &transports, "on_failure", 2);
                if let Some(peer) = datagram {
                    udp.send_to(&answer(&query, 0, false), peer).await.unwrap();
                } else {
                    let stream = stream.as_mut().unwrap();
                    let reply = answer(&query, 0, false);
                    // A concurrent close after revocation is an expected fail-closed result.
                    let _ = stream.write_all(&[reply.len() as u8]).await;
                    let _ = stream.write_all(&reply).await;
                    assert!(matches!(
                        timeout(Duration::from_secs(2), stream.read(&mut bytes)).await,
                        Ok(Ok(0)) | Ok(Err(_))
                    ));
                }
                assert!(
                    timeout(Duration::from_millis(200), client.recv_from(&mut bytes))
                        .await
                        .is_err(),
                    "lost authority produced a client response: {transport:?}, member={active_member}, revoke={revoke}"
                );
                assert!(
                    timeout(Duration::from_millis(100), first.recv_from(&mut bytes))
                        .await
                        .is_err(),
                    "lost authority retried or selected another member"
                );
                assert_eq!(server.close().await.answered, 0);
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forwarding_group_limit_refuses_third_query_and_shutdown_joins_partial_reads() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let server = server(
        v4(),
        vec![(Transport::Tcp, address)],
        "none",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
    )
    .await;
    let mut clients = Vec::new();
    let mut peers = Vec::new();
    for _ in 0..2 {
        let mut client = TcpStream::connect(server.tcp).await.unwrap();
        frame(&mut client, QUERY).await;
        let (mut stream, _) = timeout(Duration::from_secs(2), upstream.accept())
            .await
            .unwrap()
            .unwrap();
        upstream_tcp_query(&mut stream).await;
        stream.write_all(&[0]).await.unwrap();
        clients.push(client);
        peers.push(stream);
    }
    let reply = exchange_once(&server, QUERY).await.unwrap();
    assert_eq!(
        Message::from_vec(&reply).unwrap().metadata.response_code,
        ResponseCode::ServFail
    );
    assert!(
        timeout(Duration::from_millis(40), upstream.accept())
            .await
            .is_err(),
        "third forwarding socket bypassed group limit"
    );
    let began = Instant::now();
    assert_eq!(server.close().await.answered, 1);
    assert!(began.elapsed() < Duration::from_secs(3));
    for stream in &mut peers {
        let mut byte = [0];
        assert!(matches!(
            timeout(Duration::from_secs(1), stream.read(&mut byte)).await,
            Ok(Ok(0)) | Ok(Err(_))
        ));
    }
    drop(peers);
    drop(upstream);
    let _rebound = TcpListener::bind(address).await.unwrap();
    drop(clients);
}

fn txt_answer(query: &[u8], length: usize) -> Vec<u8> {
    let mut reply = answer(query, 2, false);
    reply[3] = 0x80;
    reply[7] = 1;
    reply.extend_from_slice(b"\xc0\x0c\x00\x10\x00\x01\x00\x00\x00\x3c");
    let data = length - reply.len() - 2;
    reply.extend_from_slice(&(data as u16).to_be_bytes());
    while reply.len() < length {
        let count = (length - reply.len() - 1).min(255);
        reply.push(count as u8);
        reply.resize(reply.len() + count, b'x');
    }
    reply
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upstream_message_ceiling_and_client_amplification_are_independent() {
    for length in [4096, 4097] {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = upstream.local_addr().unwrap();
        let fixture = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = timeout(Duration::from_secs(3), upstream.accept())
                    .await
                    .unwrap()
                    .unwrap();
                let query = upstream_tcp_query(&mut stream).await;
                let reply = txt_answer(&query, length);
                stream.write_u16(reply.len() as u16).await.unwrap();
                stream.write_all(&reply).await.unwrap();
                let mut byte = [0];
                assert!(matches!(
                    timeout(Duration::from_secs(2), stream.read(&mut byte)).await,
                    Ok(Ok(0)) | Ok(Err(_))
                ));
            }
        });
        let server = server(
            v4(),
            vec![(Transport::Tcp, address)],
            "none",
            Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        )
        .await;
        let mut query = QUERY.to_vec();
        let kind = query.len() - 4;
        query[kind..kind + 2].copy_from_slice(&16u16.to_be_bytes());
        let mut client = TcpStream::connect(server.tcp).await.unwrap();
        frame(&mut client, &query).await;
        let reply = read_reply(&mut client).await;
        assert_eq!(
            reply.metadata.response_code,
            if length == 4096 {
                ResponseCode::NoError
            } else {
                ResponseCode::ServFail
            }
        );
        if length == 4096 {
            assert_eq!(reply.to_vec().unwrap().len(), 4096);
            assert_eq!(reply.answers.len(), 1);
            assert!(!reply.metadata.truncation);
        } else {
            assert!(reply.answers.is_empty());
        }
        let bytes = exchange_once(&server, &query).await.unwrap();
        assert!(bytes.len() <= 3 * query.len());
        let reply = Message::from_vec(&bytes).unwrap();
        assert_eq!(reply.metadata.truncation, length == 4096);
        assert!(reply.answers.is_empty());
        drop(client);
        fixture.await.unwrap();
        assert_eq!(server.close().await.answered, 2);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn application_io_adapter_enforces_exact_successful_operation_and_byte_limits() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    let auth = server
        .owner
        .prepare_legacy_dns(typed(20), v4())
        .unwrap()
        .consume()
        .unwrap();
    crate::forward::exact_io_limits_fixture(&auth, Arc::clone(&server.owner)).await;
    drop(auth);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn application_io_denials_prevent_more_bytes_after_actual_partial_progress() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    let auth = server
        .owner
        .prepare_legacy_dns(typed(20), v4())
        .unwrap()
        .consume()
        .unwrap();
    crate::forward::terminal_denial_fixture(&auth, Arc::clone(&server.owner)).await;
    drop(auth);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn udp_prefilter_budget_counts_wrong_ids_and_accepts_no_seventeenth_packet() {
    for bad_packets in [2, 16] {
        let upstream = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = upstream.local_addr().unwrap();
        let fixture = tokio::spawn(async move {
            let mut bytes = [0; 4097];
            let (length, peer) = timeout(Duration::from_secs(3), upstream.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            let good = answer(&bytes[..length], 0, false);
            // Correct ID from a foreign peer cannot satisfy the pinned destination.
            let foreign = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            if bad_packets == 2 {
                foreign.send_to(&good, peer).await.unwrap();
            }
            let mut bad = good.clone();
            bad[0] ^= 1;
            for _ in 0..if bad_packets == 2 { 1 } else { bad_packets } {
                upstream.send_to(&bad, peer).await.unwrap();
            }
            upstream.send_to(&good, peer).await.unwrap();
        });
        let server = server(
            v4(),
            vec![(Transport::UdpTcp, address)],
            "none",
            Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        )
        .await;
        let reply = exchange_once(&server, QUERY).await.unwrap();
        assert_eq!(
            Message::from_vec(&reply).unwrap().metadata.response_code,
            if bad_packets == 2 {
                ResponseCode::NoError
            } else {
                ResponseCode::ServFail
            }
        );
        fixture.await.unwrap();
        assert_eq!(server.close().await.answered, 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_complete_question_validation_stops_at_the_sixteenth_candidate() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    let auth = server
        .owner
        .prepare_legacy_dns(typed(20), v4())
        .unwrap()
        .consume()
        .unwrap();
    crate::forward::candidate_limit_fixture(&auth, Arc::clone(&server.owner)).await;
    drop(auth);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_foreign_question_flood_cannot_produce_a_successful_answer() {
    for bad_packets in [16] {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = upstream.local_addr().unwrap();
        let fixture = tokio::spawn(async move {
            let (mut stream, _) = timeout(Duration::from_secs(3), upstream.accept())
                .await
                .unwrap()
                .unwrap();
            let query = upstream_tcp_query(&mut stream).await;
            let good = answer(&query, 0, false);
            let mut bad = good.clone();
            bad[13] = b'f'; // Valid DNS name and ID, different question.
            let mut frames = Vec::new();
            for reply in std::iter::repeat_n(&bad, bad_packets).chain(std::iter::once(&good)) {
                frames.extend_from_slice(&(reply.len() as u16).to_be_bytes());
                frames.extend_from_slice(reply);
            }
            // One write avoids peer-close races while delivering hostile candidates.
            stream.write_all(&frames).await.unwrap();
            let mut bytes = [0];
            assert!(matches!(
                timeout(Duration::from_secs(2), stream.read(&mut bytes)).await,
                Ok(Ok(0)) | Ok(Err(_))
            ));
        });
        let server = server(
            v4(),
            vec![(Transport::Tcp, address)],
            "none",
            Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        )
        .await;
        let reply = exchange_once(&server, QUERY).await.unwrap();
        assert_eq!(
            Message::from_vec(&reply).unwrap().metadata.response_code,
            ResponseCode::ServFail
        );
        fixture.await.unwrap();
        assert_eq!(server.close().await.answered, 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_udp_frames_use_only_explicit_fallback_and_never_authorize_tcp() {
    for fault in ["question", "trailing", "count", "oversized", "invalid_tc"] {
        let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = tcp.local_addr().unwrap();
        let upstream = UdpSocket::bind(address).await.unwrap();
        let next = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let next_address = next.local_addr().unwrap();
        let first = tokio::spawn(async move {
            let mut bytes = [0; 4097];
            let (length, peer) = timeout(Duration::from_secs(3), upstream.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            let mut bad = answer(&bytes[..length], 0, fault == "invalid_tc");
            match fault {
                "question" | "invalid_tc" => bad[13] = b'f',
                "trailing" => bad.push(0),
                "count" => bad[7] = 2,
                "oversized" => bad.resize(4097, 0),
                _ => unreachable!(),
            }
            upstream.send_to(&bad, peer).await.unwrap();
            assert!(
                timeout(Duration::from_millis(100), tcp.accept())
                    .await
                    .is_err(),
                "invalid TC authorized a TCP query"
            );
            assert!(
                timeout(Duration::from_millis(100), upstream.recv_from(&mut bytes))
                    .await
                    .is_err(),
                "invalid frame caused retransmission"
            );
        });
        let second = tokio::spawn(async move {
            let mut bytes = [0; 4097];
            let (length, peer) = timeout(Duration::from_secs(3), next.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            next.send_to(&answer(&bytes[..length], 0, false), peer)
                .await
                .unwrap();
        });
        let server = server(
            v4(),
            vec![
                (Transport::UdpTcp, address),
                (Transport::UdpTcp, next_address),
            ],
            "on_failure",
            Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        )
        .await;
        let reply = exchange_once(&server, QUERY).await.unwrap();
        assert_eq!(
            Message::from_vec(&reply).unwrap().metadata.response_code,
            ResponseCode::NoError,
            "fault={fault}"
        );
        first.await.unwrap();
        second.await.unwrap();
        assert_eq!(server.close().await.answered, 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_lease_expiry_during_each_udp_attempt_forbids_response_and_fallback() {
    for active_member in 0..2 {
        let first = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let second = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = server(
            v4(),
            vec![
                (Transport::UdpTcp, first.local_addr().unwrap()),
                (Transport::UdpTcp, second.local_addr().unwrap()),
            ],
            "on_failure",
            Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        )
        .await;
        // The real conservative clock adds2s. Start late in this second so its
        // upper bound reaches an integer expiry inside the500ms member deadline.
        while !(650..=800).contains(
            &SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .subsec_millis(),
        ) {
            sleep(Duration::from_millis(5)).await;
        }
        let time = now();
        let mut document = server.doc.clone();
        for grant in document["legacyGrants"].as_array_mut().unwrap() {
            grant["lease"]["issuedAt"] = json!(time - 60);
            grant["lease"]["expiresAt"] = json!(time + 3);
        }
        apply_forwarding(
            &server.owner,
            &document,
            &[Transport::UdpTcp, Transport::UdpTcp],
            "on_failure",
            2,
        );
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(QUERY, server.udp).await.unwrap();
        let mut bytes = [0; 4097];
        if active_member == 1 {
            let (length, peer) = timeout(Duration::from_millis(200), first.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            first
                .send_to(&answer(&bytes[..length], 2, false), peer)
                .await
                .unwrap();
        }
        let target = if active_member == 0 { &first } else { &second };
        let (length, peer) = timeout(Duration::from_millis(200), target.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        while now() == time {
            sleep(Duration::from_millis(5)).await;
        }
        target
            .send_to(&answer(&bytes[..length], 0, false), peer)
            .await
            .unwrap();
        assert!(
            timeout(Duration::from_millis(200), client.recv_from(&mut bytes))
                .await
                .is_err()
        );
        assert!(
            timeout(Duration::from_millis(100), second.recv_from(&mut bytes))
                .await
                .is_err(),
            "expired admission selected fallback or retried"
        );
        assert_eq!(server.close().await.answered, 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generation_change_during_owned_tcp_cleanup_forbids_fallback_and_client_reply() {
    let first = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let next = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let server = server_delayed(
        v4(),
        vec![
            (Transport::Tcp, first.local_addr().unwrap()),
            (Transport::UdpTcp, next.local_addr().unwrap()),
        ],
        "on_failure",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        Duration::from_millis(300),
        Duration::ZERO,
    )
    .await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.send_to(QUERY, server.udp).await.unwrap();
    let (mut peer, _) = timeout(Duration::from_secs(2), first.accept())
        .await
        .unwrap()
        .unwrap();
    let query = upstream_tcp_query(&mut peer).await;
    let reply = answer(&query, 2, false); // Eligible fallback only while admission remains current.
    peer.write_u16(reply.len() as u16).await.unwrap();
    peer.write_all(&reply).await.unwrap();
    sleep(Duration::from_millis(50)).await;
    apply_forwarding(
        &server.owner,
        &server.doc,
        &[Transport::Tcp, Transport::UdpTcp],
        "on_failure",
        2,
    );
    let mut bytes = [0; 4097];
    assert!(matches!(
        timeout(Duration::from_secs(2), peer.read(&mut bytes)).await,
        Ok(Ok(0)) | Ok(Err(_))
    ));
    assert!(
        timeout(Duration::from_millis(200), client.recv_from(&mut bytes))
            .await
            .is_err()
    );
    assert!(
        timeout(Duration::from_millis(100), next.recv_from(&mut bytes))
            .await
            .is_err()
    );
    assert_eq!(server.close().await.answered, 0);
}
