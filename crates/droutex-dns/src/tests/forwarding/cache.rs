//! Real D54 wire exchanges. Counters distinguish a cache hit from another lookup.
use super::*;

struct Upstream {
    address: SocketAddr,
    count: Arc<AtomicUsize>,
    mode: Arc<AtomicUsize>,
    stop: watch::Sender<bool>,
    handle: JoinHandle<()>,
}
impl Upstream {
    async fn start(ip: IpAddr, transport: Transport, mode: usize) -> Self {
        let udp = UdpSocket::bind((ip, 0)).await.unwrap();
        let tcp = TcpListener::bind((ip, 0)).await.unwrap();
        let address = if transport == Transport::Tcp {
            tcp.local_addr().unwrap()
        } else {
            udp.local_addr().unwrap()
        };
        let count = Arc::new(AtomicUsize::new(0));
        let kind = Arc::new(AtomicUsize::new(mode));
        let calls = Arc::clone(&count);
        let current = Arc::clone(&kind);
        let (stop, mut cancelled) = watch::channel(false);
        let handle = tokio::spawn(async move {
            let mut bytes = [0; 4097];
            loop {
                if transport == Transport::Tcp {
                    let (mut socket, _) = tokio::select! { biased;
                        _=cancelled.changed()=>break,
                        accepted=tcp.accept()=>accepted.unwrap(),
                    };
                    let query = upstream_tcp_query(&mut socket).await;
                    calls.fetch_add(1, Ordering::Relaxed);
                    let reply = data(&query, current.load(Ordering::Relaxed));
                    socket.write_u16(reply.len() as u16).await.unwrap();
                    socket.write_all(&reply).await.unwrap();
                    let mut byte = [0];
                    assert!(matches!(
                        timeout(Duration::from_secs(2), socket.read(&mut byte)).await,
                        Ok(Ok(0)) | Ok(Err(_))
                    ));
                } else {
                    let (len, peer) = tokio::select! { biased;
                        _=cancelled.changed()=>break,
                        received=udp.recv_from(&mut bytes)=>received.unwrap(),
                    };
                    calls.fetch_add(1, Ordering::Relaxed);
                    udp.send_to(&data(&bytes[..len], current.load(Ordering::Relaxed)), peer)
                        .await
                        .unwrap();
                }
            }
        });
        Self {
            address,
            count,
            mode: kind,
            stop,
            handle,
        }
    }
    fn calls(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
    async fn close(self) {
        self.stop.send_replace(true);
        self.handle.await.unwrap();
    }
}
fn data(query: &[u8], mode: usize) -> Vec<u8> {
    match mode {
        0 | 1 | 6 => {
            let mut r = answer(query, 0, false);
            let len = r.len();
            r[len - 10..len - 6].copy_from_slice(
                &(if mode == 1 {
                    0u32
                } else if mode == 6 {
                    1
                } else {
                    60
                })
                .to_be_bytes(),
            );
            r
        }
        2 | 3 => answer(query, if mode == 2 { 2 } else { 5 }, false),
        4 | 5 => {
            let soa = record_vectors::record_cases()
                .into_iter()
                .find(|case| case.code == 6)
                .unwrap();
            let mut r = record_answer(query, &soa);
            r[3] = 0x80 | if mode == 4 { 3 } else { 0 };
            r[7] = 0;
            r[9] = 1;
            let ttl = r.len() - soa.data.len() - 6;
            r[ttl..ttl + 4].copy_from_slice(&1u32.to_be_bytes());
            r
        }
        7 => {
            let mut r = answer(query, 2, false);
            r[3] = 0x80;
            r[7] = 1;
            r.extend_from_slice(b"\xc0\x0c\x00\x10\x00\x01\x00\x00\x00\x3c\x04\xb0");
            for _ in 0..6 {
                r.push(199);
                r.extend_from_slice(&[b'x'; 199]);
            }
            r
        }
        _ => {
            // Legal empty NULL/unknown RDATA in any section of A or NXDOMAIN.
            let mut r = if mode >= 14 {
                data(query, 4)
            } else {
                answer(query, 0, false)
            };
            let section = (mode - 8) % 3;
            let count = match section {
                0 => 7,
                1 => 9,
                _ => 11,
            };
            r[count] += 1;
            let code = if mode.is_multiple_of(2) { 10u16 } else { 65280 };
            let mut empty = b"\xc0\x0c".to_vec();
            empty.extend_from_slice(&code.to_be_bytes());
            empty.extend_from_slice(b"\x00\x01\x00\x00\x00\x3c\x00\x00");
            if section == 0 && mode >= 14 {
                let question_end = answer(query, 2, false).len();
                r.splice(question_end..question_end, empty);
            } else {
                r.extend_from_slice(&empty);
            }
            r
        }
    }
}
async fn start(upstream: &Upstream, transport: Transport) -> Server {
    server(
        upstream.address.ip(),
        vec![(transport, upstream.address)],
        "none",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hits_reconstruct_current_request_across_udp_tcp_and_both_ip_families() {
    for ip in [v4(), "::1".parse().unwrap()] {
        for transport in [Transport::UdpTcp, Transport::Tcp] {
            let upstream = Upstream::start(ip, transport, 0).await;
            let server = start(&upstream, transport).await;
            assert_eq!(
                Message::from_vec(&exchange_once(&server, QUERY).await.unwrap())
                    .unwrap()
                    .answers
                    .len(),
                1
            );
            let mut query = QUERY.to_vec();
            query[..2].copy_from_slice(&987u16.to_be_bytes());
            query[13] = b'E';
            query[2] &= !1;
            query[11] = 1;
            query.extend_from_slice(b"\x00\x00\x29\x10\x00\x00\x00\x00\x00\x00\x00");
            let mut stream = TcpStream::connect(server.tcp).await.unwrap();
            frame(&mut stream, &query).await;
            let hit = read_reply(&mut stream).await;
            assert_eq!(hit.metadata.id, 987);
            assert_eq!(hit.queries[0].name.to_ascii(), "Example.test.");
            assert_eq!(hit.edns.unwrap().max_payload(), 1232);
            assert!(!hit.metadata.authentic_data);
            assert!(!hit.metadata.recursion_desired);
            assert_eq!(
                upstream.calls(),
                1,
                "{ip}, {transport:?}: repeated upstream I/O"
            );
            drop(stream);
            assert_eq!(server.close().await.answered, 2);
            upstream.close().await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn positive_and_soa_negatives_expire_and_qtypes_do_not_share_denial() {
    for mode in [4, 5, 6] {
        for transport in [Transport::UdpTcp, Transport::Tcp] {
            let upstream = Upstream::start(v4(), transport, mode).await;
            let server = start(&upstream, transport).await;
            for _ in 0..2 {
                exchange_once(&server, QUERY).await.unwrap();
            }
            assert_eq!(upstream.calls(), 1, "mode {mode}, {transport:?}");
            let mut aaaa = QUERY.to_vec();
            let len = aaaa.len();
            aaaa[len - 4..len - 2].copy_from_slice(&28u16.to_be_bytes());
            exchange_once(&server, &aaaa).await.unwrap();
            assert_eq!(upstream.calls(), 2);
            sleep(Duration::from_millis(1100)).await;
            exchange_once(&server, QUERY).await.unwrap();
            assert_eq!(upstream.calls(), 3);
            assert_eq!(server.close().await.answered, 4);
            upstream.close().await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zero_ttl_and_servfail_refused_are_never_cached() {
    for mode in [1, 2, 3] {
        for transport in [Transport::UdpTcp, Transport::Tcp] {
            let upstream = Upstream::start(v4(), transport, mode).await;
            let server = start(&upstream, transport).await;
            for _ in 0..2 {
                exchange_once(&server, QUERY).await.unwrap();
            }
            assert_eq!(upstream.calls(), 2, "mode {mode}, {transport:?}");
            assert_eq!(server.close().await.answered, 2);
            upstream.close().await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn udp_client_truncation_does_not_poison_the_complete_tcp_cache_value() {
    let upstream = Upstream::start(v4(), Transport::Tcp, 7).await;
    let server = start(&upstream, Transport::Tcp).await;
    let udp = exchange_once(&server, QUERY).await.unwrap();
    assert!(Message::from_vec(&udp).unwrap().metadata.truncation);
    assert!(udp.len() <= 3 * QUERY.len());
    let mut socket = TcpStream::connect(server.tcp).await.unwrap();
    frame(&mut socket, QUERY).await;
    let hit = read_reply(&mut socket).await;
    assert!(!hit.metadata.truncation);
    assert_eq!(hit.answers.len(), 1);
    assert_eq!(upstream.calls(), 1);
    drop(socket);
    assert_eq!(server.close().await.answered, 2);
    upstream.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generation_rollback_failed_candidate_and_revoked_policy_use_current_authority() {
    let upstream = Upstream::start(v4(), Transport::UdpTcp, 0).await;
    let mut server = start(&upstream, Transport::UdpTcp).await;
    exchange_once(&server, QUERY).await.unwrap();
    assert_eq!(upstream.calls(), 1);
    assert!(
        server
            .owner
            .activate(
                vec![0],
                Artifacts::new(vec![]).unwrap(),
                &Cancellation::default()
            )
            .is_err()
    );
    exchange_once(&server, QUERY).await.unwrap();
    assert_eq!(upstream.calls(), 1, "failed candidate flushed LKG");
    for seq in [2, 3] {
        apply_forwarding(
            &server.owner,
            &server.doc,
            &[Transport::UdpTcp],
            "none",
            seq,
        );
        exchange_once(&server, QUERY).await.unwrap();
        assert_eq!(upstream.calls(), seq as usize);
    }
    server.doc["profiles"][0]["defaultDnsAction"] = json!({"kind":"block"});
    apply_forwarding(&server.owner, &server.doc, &[Transport::UdpTcp], "none", 4);
    let denied = Message::from_vec(&exchange_once(&server, QUERY).await.unwrap()).unwrap();
    assert_eq!(denied.metadata.response_code, ResponseCode::ServFail);
    assert_eq!(upstream.calls(), 3);
    server.doc["endpoints"][0]["enabled"] = json!(false);
    apply_forwarding(&server.owner, &server.doc, &[Transport::UdpTcp], "none", 5);
    assert!(exchange_once(&server, QUERY).await.is_err());
    assert_eq!(upstream.calls(), 3);
    assert_eq!(server.close().await.answered, 5);
    upstream.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn permitted_fallback_stays_cached_after_primary_recovers() {
    for mode in [2, 8] {
        let primary = Upstream::start(v4(), Transport::UdpTcp, mode).await;
        let secondary = Upstream::start(v4(), Transport::UdpTcp, 0).await;
        let server = server(
            v4(),
            vec![
                (Transport::UdpTcp, primary.address),
                (Transport::UdpTcp, secondary.address),
            ],
            "on_failure",
            Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        )
        .await;
        let response = Message::from_vec(&exchange_once(&server, QUERY).await.unwrap()).unwrap();
        assert_eq!(response.answers.len(), 1);
        primary.mode.store(0, Ordering::Relaxed);
        exchange_once(&server, QUERY).await.unwrap();
        assert_eq!(primary.calls(), 1);
        assert_eq!(secondary.calls(), 1);
        assert_eq!(server.close().await.answered, 2);
        primary.close().await;
        secondary.close().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn legal_empty_rdata_failures_in_all_sections_are_retried_not_negative_cached() {
    for transport in [Transport::UdpTcp, Transport::Tcp] {
        for mode in 8..20 {
            let upstream = Upstream::start(v4(), transport, mode).await;
            let server = start(&upstream, transport).await;
            for _ in 0..2 {
                let reply =
                    Message::from_vec(&exchange_once(&server, QUERY).await.unwrap()).unwrap();
                assert_eq!(
                    reply.metadata.response_code,
                    ResponseCode::ServFail,
                    "mode {mode}"
                );
            }
            assert_eq!(upstream.calls(), 2, "mode {mode}, {transport:?}");
            assert_eq!(server.close().await.answered, 2);
            upstream.close().await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_ip_endpoints_profiles_and_actual_peer_groups_never_share_entries() {
    let upstream = Upstream::start(v4(), Transport::UdpTcp, 0).await;
    let udp = UdpSocket::bind((v4(), 0)).await.unwrap();
    let other = UdpSocket::bind((v4(), 0)).await.unwrap();
    let tcp = TcpListener::bind((v4(), 0)).await.unwrap();
    let udp_addr = udp.local_addr().unwrap();
    let other_addr = other.local_addr().unwrap();
    let tcp_addr = tcp.local_addr().unwrap();
    let mut doc = document(udp_addr, tcp_addr);
    let resolve = json!({"kind":"resolve","upstream":text(60)});
    doc["profiles"][0]["defaultDnsAction"] = resolve.clone();
    doc["profiles"].as_array_mut().unwrap().push(json!({"id":text(11),"name":"Second","defaultDnsAction":resolve,"defaultTrafficAction":{"kind":"deny"}}));
    doc["endpoints"][0]["access"].as_array_mut().unwrap().pop();
    doc["endpoints"][0]["access"][0]["grants"] = json!([text(40), text(43)]);
    doc["endpoints"].as_array_mut().unwrap().push(json!({"id":text(31),"name":"Second","enabled":true,"profile":text(11),"access":[{"transport":"tcp","listener":text(21),"grants":[text(41)]}]}));
    doc["legacyGrants"][1]["endpoint"] = json!(text(31));
    doc["listeners"].as_array_mut().unwrap().push(json!({"id":text(22),"name":"Third UDP","enabled":true,"bind":other_addr.to_string(),"transport":"dns_udp"}));
    doc["endpoints"].as_array_mut().unwrap().push(json!({"id":text(32),"name":"Same Profile, different Endpoint","enabled":true,"profile":text(10),"access":[{"transport":"udp","listener":text(22),"grants":[text(42)]}]}));
    let mut third = doc["legacyGrants"][0].clone();
    third["id"] = json!(text(42));
    third["listener"] = json!(text(22));
    third["endpoint"] = json!(text(32));
    doc["legacyGrants"].as_array_mut().unwrap().push(third);
    doc["networkGroups"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":text(51),"name":"Different peer group","trafficProfile":text(10)}));
    let mut peer = doc["legacyGrants"][0].clone();
    peer["id"] = json!(text(43));
    peer["source"] = json!("127.0.0.2");
    peer["group"] = json!(text(51));
    doc["legacyGrants"].as_array_mut().unwrap().push(peer);
    let owner = Arc::new(Deployment::in_memory(typed(1), keys().1));
    apply_forwarding(&owner, &doc, &[Transport::UdpTcp], "none", 1);
    let refresh = Arc::new(crate::clock::Refresh::new());
    let (stop, shutdown) = watch::channel(false);
    let handle = start_runtime(
        std::future::ready(()),
        runtime::serve_with_refresh(
            Arc::clone(&owner),
            vec![
                ListenerSocket::Udp {
                    id: typed(20),
                    socket: udp,
                },
                ListenerSocket::Udp {
                    id: typed(22),
                    socket: other,
                },
                ListenerSocket::Tcp {
                    id: typed(21),
                    socket: tcp,
                },
            ],
            shutdown,
            crate::forward::fixture(
                vec![(typed(60), vec![(Transport::UdpTcp, upstream.address)])],
                Arc::new(Mutex::new(Ok(BTreeSet::new()))),
            )
            .unwrap(),
            Arc::clone(&refresh),
        ),
    )
    .await;
    let server = Server {
        owner,
        refresh,
        doc,
        udp: udp_addr,
        tcp: tcp_addr,
        stop,
        handle,
    };
    sleep(Duration::from_millis(200)).await;
    for round in 0..2 {
        exchange_once(&server, QUERY).await.unwrap();
        let mut socket = TcpStream::connect(server.tcp).await.unwrap();
        frame(&mut socket, QUERY).await;
        assert_eq!(read_reply(&mut socket).await.answers.len(), 1);
        drop(socket);
        for (source, destination) in [(v4(), other_addr), ("127.0.0.2".parse().unwrap(), udp_addr)]
        {
            let client = UdpSocket::bind((source, 0)).await.unwrap();
            client.send_to(QUERY, destination).await.unwrap();
            let mut bytes = [0; 1232];
            let (len, _) = timeout(Duration::from_secs(3), client.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(Message::from_vec(&bytes[..len]).unwrap().answers.len(), 1);
        }
        assert_eq!(
            upstream.calls(),
            4,
            "round {round} crossed an admission scope or missed warm entries"
        );
    }
    assert_eq!(server.close().await.answered, 8);
    upstream.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simultaneous_identical_misses_own_separate_attempts_and_cancellation() {
    let upstream = UdpSocket::bind((v4(), 0)).await.unwrap();
    let server = server(
        v4(),
        vec![(Transport::UdpTcp, upstream.local_addr().unwrap())],
        "none",
        Arc::new(Mutex::new(Ok(BTreeSet::new()))),
    )
    .await;
    let first = UdpSocket::bind((v4(), 0)).await.unwrap();
    let second = UdpSocket::bind((v4(), 0)).await.unwrap();
    first.send_to(QUERY, server.udp).await.unwrap();
    second.send_to(QUERY, server.udp).await.unwrap();
    let mut bytes = [0; 4097];
    let mut attempts = Vec::new();
    for _ in 0..2 {
        let (len, peer) = timeout(Duration::from_millis(400), upstream.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        attempts.push((bytes[..len].to_vec(), peer));
    }
    assert_ne!(
        attempts[0].1, attempts[1].1,
        "misses shared an upstream socket"
    );
    assert_eq!(server.close().await.answered, 0);
    for (query, peer) in attempts {
        upstream
            .send_to(&answer(&query, 0, false), peer)
            .await
            .unwrap();
    }
    let mut late = [0; 1232];
    assert!(
        timeout(Duration::from_millis(100), first.recv_from(&mut late))
            .await
            .is_err()
    );
    assert!(
        timeout(Duration::from_millis(100), second.recv_from(&mut late))
            .await
            .is_err()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn decoded_hit_rechecks_publication_trust_and_expiry_before_consumption() {
    for race in 0..3 {
        let upstream =
            Upstream::start(v4(), Transport::UdpTcp, if race == 2 { 6 } else { 0 }).await;
        let barrier = Arc::new(crate::forward::CacheHitBarrier::default());
        let mut processor = crate::forward::fixture(
            vec![(typed(60), vec![(Transport::UdpTcp, upstream.address)])],
            Arc::new(Mutex::new(Ok(BTreeSet::new()))),
        )
        .unwrap();
        processor.fixture_cache_hit_barrier(Arc::clone(&barrier));
        let mut server = Server::start(v4(), processor).await;
        server.doc["profiles"][0]["defaultDnsAction"] =
            json!({"kind":"resolve","upstream":text(60)});
        apply_forwarding(&server.owner, &server.doc, &[Transport::UdpTcp], "none", 2);
        exchange_once(&server, QUERY).await.unwrap();
        assert_eq!(upstream.calls(), 1);
        let client = UdpSocket::bind((v4(), 0)).await.unwrap();
        client.send_to(QUERY, server.udp).await.unwrap();
        timeout(Duration::from_secs(1), barrier.entered.notified())
            .await
            .unwrap();
        match race {
            0 => apply_forwarding(&server.owner, &server.doc, &[Transport::UdpTcp], "none", 3),
            1 => {
                let public =
                    hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
                        .try_into()
                        .unwrap();
                assert!(
                    !server
                        .owner
                        .replace_trust(
                            SnapshotTrust::new(vec![(
                                SigningKeyId::from_bytes([2; 16]).unwrap(),
                                public
                            )])
                            .unwrap()
                        )
                        .unwrap()
                );
            }
            _ => sleep(Duration::from_millis(1100)).await,
        }
        barrier.release.notify_one();
        let mut bytes = [0; 1232];
        if race == 2 {
            let (len, _) = timeout(Duration::from_secs(1), client.recv_from(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(Message::from_vec(&bytes[..len]).unwrap().answers.len(), 1);
            assert_eq!(
                upstream.calls(),
                2,
                "expired hit did not become a bounded miss"
            );
            assert_eq!(server.close().await.answered, 2);
        } else {
            assert_eq!(server.close().await.answered, 1);
            assert!(
                timeout(Duration::from_millis(100), client.recv_from(&mut bytes))
                    .await
                    .is_err()
            );
            assert_eq!(
                upstream.calls(),
                1,
                "authority loss became another upstream attempt"
            );
        }
        upstream.close().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cache_to_miss_keeps_the_original_checkpoint_wait_budget() {
    let upstream = Upstream::start(v4(), Transport::UdpTcp, 0).await;
    let server = start(&upstream, Transport::UdpTcp).await;
    let auth = server
        .owner
        .prepare_legacy_dns(typed(20), v4())
        .unwrap()
        .consume()
        .unwrap();
    crate::forward::cache_transition_fixture(
        auth.dns_io_check().unwrap(),
        Arc::clone(&server.owner),
    )
    .await;
    assert_eq!(upstream.calls(), 0);
    drop(auth);
    assert_eq!(server.close().await.answered, 0);
    upstream.close().await;
}
