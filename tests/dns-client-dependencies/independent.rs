//! Raw-client probes that do not depend on the proposed caller-validation boundary.

use super::*;
use hickory_proto::rr::RecordType;
use tokio::sync::oneshot;

#[derive(Clone, Copy, Debug)]
enum Foreign {
    Port,
    Address,
    Id,
    Name,
    Type,
    Class,
}

async fn udp_probe(
    bind: &str,
    query: &[u8],
    answer: &[u8],
    foreign: Option<Foreign>,
) -> TestResult<Result<DnsResponse, NetError>> {
    let peer = UdpSocket::bind(bind).await?;
    let address = peer.local_addr()?;
    let mut client = UdpClientStream::builder(address, TokioRuntimeProvider::new())
        .with_timeout(Some(Duration::from_millis(500)))
        .with_max_retries(0)
        .build();
    let server = async {
        let mut bytes = [0; 1232];
        let (length, client_address) = peer.recv_from(&mut bytes).await?;
        assert_eq!(&bytes[12..length], &query[12..]);
        let mut good = answer.to_vec();
        good[..2].copy_from_slice(&bytes[..2]);
        if let Some(foreign) = foreign {
            let mut bad = good.clone(); // Independent candidate must coexist with the valid one.
            *bad.last_mut().unwrap() = 66;
            match foreign {
                Foreign::Id => bad[0] ^= 1,
                Foreign::Name => bad[13..20].copy_from_slice(b"foreign"),
                Foreign::Type => {
                    bad[query.len() - 4..query.len() - 2].copy_from_slice(&28u16.to_be_bytes())
                }
                Foreign::Class => {
                    bad[query.len() - 2..query.len()].copy_from_slice(&3u16.to_be_bytes())
                }
                Foreign::Port | Foreign::Address => (),
            }
            match foreign {
                Foreign::Port | Foreign::Address => {
                    let bind: SocketAddr = if matches!(foreign, Foreign::Address) {
                        SocketAddr::new("127.0.0.2".parse()?, address.port())
                    } else {
                        "127.0.0.1:0".parse()?
                    };
                    let stranger = UdpSocket::bind(bind).await?;
                    assert_ne!(stranger.local_addr()?, address);
                    if matches!(foreign, Foreign::Address) {
                        assert_eq!(stranger.local_addr()?.port(), address.port());
                        assert_ne!(stranger.local_addr()?.ip(), address.ip());
                    } else {
                        assert_eq!(stranger.local_addr()?.ip(), address.ip());
                        assert_ne!(stranger.local_addr()?.port(), address.port());
                    }
                    stranger.send_to(&bad, client_address).await?;
                }
                _ => {
                    peer.send_to(&bad, client_address).await?;
                }
            }
        }
        peer.send_to(&good, client_address).await?;
        TestResult::Ok(client_address)
    };
    let request = DnsRequest::new(Message::from_vec(query)?, DnsRequestOptions::default());
    let response = timeout(DEADLINE, client.send_message(request).first_answer());
    let (response, client_address) = tokio::join!(response, timeout(DEADLINE, server));
    let response = response?;
    let client_address = client_address??;
    let _released = UdpSocket::bind(client_address).await?;
    Ok(response)
}

#[derive(Clone, Copy, Debug)]
enum Delivery {
    Whole,
    Bytewise,
    Truncated(usize),
}

async fn tcp_probe(
    bind: &str,
    query: &[u8],
    answer: &[u8],
    delivery: Delivery,
) -> TestResult<Result<DnsResponse, NetError>> {
    let listener = TcpListener::bind(bind).await?;
    let address = listener.local_addr()?;
    let peer = async {
        let (mut socket, _) = listener.accept().await?;
        let length = socket.read_u16().await?;
        let mut incoming = vec![0; usize::from(length)];
        socket.read_exact(&mut incoming).await?;
        assert_eq!(&incoming[12..], &query[12..]);
        let mut frame = u16::try_from(answer.len())?.to_be_bytes().to_vec();
        frame.extend_from_slice(answer);
        frame[2..4].copy_from_slice(&incoming[..2]);
        match delivery {
            Delivery::Whole => socket.write_all(&frame).await?,
            Delivery::Bytewise => {
                for byte in &frame {
                    socket.write_all(&[*byte]).await?;
                    tokio::task::yield_now().await;
                }
            }
            Delivery::Truncated(length) => {
                socket.write_all(&frame[..length]).await?;
                socket.shutdown().await?;
            }
        }
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).await?, 0);
        TestResult::Ok(())
    };
    let request = DnsRequest::new(Message::from_vec(query)?, DnsRequestOptions::default());
    let client = async {
        let owner = OwnedTcp::connect(address).await?;
        let response = timeout(
            DEADLINE,
            owner
                .exchange
                .as_ref()
                .unwrap()
                .send(request)
                .first_answer(),
        )
        .await;
        owner.close().await?;
        TestResult::Ok(response?)
    };
    let (response, completed) = tokio::join!(client, timeout(DEADLINE, peer));
    completed??;
    drop(listener);
    let _rebound = TcpListener::bind(address).await?;
    response
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn udp_foreign_peer_port_id_name_type_and_class_allow_the_next_valid_answer() -> TestResult {
    for foreign in [
        Foreign::Port,
        Foreign::Address,
        Foreign::Id,
        Foreign::Name,
        Foreign::Type,
        Foreign::Class,
    ] {
        let response = udp_probe("127.0.0.1:0", QUERY, ANSWER, Some(foreign)).await??;
        assert_eq!(response.queries[0].name.to_ascii(), "example.test.");
        assert_eq!(
            response.answers[0].data.to_string(),
            "203.0.113.9",
            "{foreign:?}"
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn udp_ipv6_uses_the_independent_peer_and_releases_its_socket() -> TestResult {
    let response = udp_probe("[::1]:0", QUERY, ANSWER, None).await??;
    assert_eq!(response.answers[0].data.to_string(), "203.0.113.9");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_ipv6_uses_the_independent_peer_and_joins_owned_background() -> TestResult {
    let response = tcp_probe("[::1]:0", QUERY, ANSWER, Delivery::Whole).await??;
    assert_eq!(response.answers[0].data.to_string(), "203.0.113.9");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_bytewise_prefix_and_payload_preserve_the_whole_answer() -> TestResult {
    let response = tcp_probe("127.0.0.1:0", QUERY, ANSWER, Delivery::Bytewise).await??;
    assert_eq!(response.answers[0].data.to_string(), "203.0.113.9");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_eof_in_the_length_or_body_never_yields_a_successful_answer() -> TestResult {
    for length in [1, 2, 3, 14, ANSWER.len() + 1] {
        let response = tcp_probe("127.0.0.1:0", QUERY, ANSWER, Delivery::Truncated(length)).await?;
        assert!(response.is_err(), "truncated frame length {length}");
    }
    Ok(())
}

#[path = "../dns-fixtures/records.rs"]
mod record_vectors;
pub(super) use record_vectors::record_cases;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_ordinary_and_auxiliary_records_cross_real_udp_and_tcp() -> TestResult {
    for case in record_cases() {
        let mut query = QUERY.to_vec();
        let question_type = query.len() - 4;
        query[question_type..question_type + 2].copy_from_slice(&case.code.to_be_bytes());
        let mut answer = query.clone(); // Wire response retains the independently defined question.
        answer[2..4].copy_from_slice(b"\x81\x80");
        answer[6..8].copy_from_slice(&1u16.to_be_bytes());
        answer.extend_from_slice(b"\xc0\x0c");
        answer.extend_from_slice(&case.code.to_be_bytes());
        answer.extend_from_slice(b"\x00\x01\x00\x00\x00\x3c");
        answer.extend_from_slice(&u16::try_from(case.data.len())?.to_be_bytes());
        answer.extend_from_slice(case.data);
        for response in [
            udp_probe("127.0.0.1:0", &query, &answer, None).await??,
            tcp_probe("127.0.0.1:0", &query, &answer, Delivery::Whole).await??,
        ] {
            assert_eq!(response.answers.len(), 1, "type {}", case.code);
            assert_eq!(
                response.answers[0].record_type(),
                RecordType::from(case.code)
            );
            assert_eq!(
                response.answers[0].data.to_string().trim(),
                case.expected,
                "type {}",
                case.code
            );
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_udp_answer_times_out_without_another_wire_query() -> TestResult {
    let peer = UdpSocket::bind("127.0.0.1:0").await?;
    let mut client = UdpClientStream::builder(peer.local_addr()?, TokioRuntimeProvider::new())
        .with_timeout(Some(Duration::from_millis(500)))
        .with_max_retries(0)
        .build();
    let server = async {
        let mut bytes = [0; 512];
        let (_, address) = peer.recv_from(&mut bytes).await?;
        assert!(
            timeout(Duration::from_millis(700), peer.recv_from(&mut bytes))
                .await
                .is_err()
        );
        TestResult::Ok(address)
    };
    let request = client.send_message(request()?);
    let (response, address) = tokio::join!(
        timeout(DEADLINE, request.first_answer()),
        timeout(DEADLINE, server),
    );
    assert!(matches!(response?, Err(NetError::Timeout)));
    let _released = UdpSocket::bind(address??).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_udp_response_wait_releases_the_observed_socket() -> TestResult {
    let peer = UdpSocket::bind("127.0.0.1:0").await?;
    let mut client = UdpClientStream::builder(peer.local_addr()?, TokioRuntimeProvider::new())
        .with_timeout(Some(Duration::from_millis(500)))
        .with_max_retries(0)
        .build();
    let (sent, received) = oneshot::channel();
    let server = async {
        let mut bytes = [0; 512];
        let (_, address) = peer.recv_from(&mut bytes).await?;
        sent.send(address)
            .map_err(|_| "UDP cancellation receiver dropped")?;
        TestResult::Ok(())
    };
    let request = request()?;
    let cancelled = async {
        let mut response = Box::pin(client.send_message(request).first_answer());
        let address = tokio::select! {
            response = &mut response => return Err(format!("unexpected UDP completion: {response:?}").into()),
            address = received => address?,
        };
        drop(response);
        let _released = UdpSocket::bind(address).await?;
        TestResult::Ok(())
    };
    let (cancelled, completed) =
        tokio::join!(timeout(DEADLINE, cancelled), timeout(DEADLINE, server));
    cancelled??;
    completed??;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_tcp_response_wait_aborts_joins_and_closes_the_partial_peer() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let (sent, received) = oneshot::channel();
    let peer = async {
        let (mut socket, _) = listener.accept().await?;
        let length = socket.read_u16().await?;
        let mut query = vec![0; usize::from(length)];
        socket.read_exact(&mut query).await?;
        socket.write_all(&[0]).await?; // Only half the DNS frame prefix, not an answer.
        sent.send(())
            .map_err(|_| "TCP cancellation receiver dropped")?;
        let mut byte = [0];
        // Aborting with unread peer bytes may close with RST instead of FIN.
        // Both prove closure; a live socket, extra data or other error fails.
        match socket.read(&mut byte).await {
            Ok(0) => (),
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => (),
            result => return Err(format!("cancelled TCP peer did not close: {result:?}").into()),
        }
        TestResult::Ok(())
    };
    let client = async {
        let owner = OwnedTcp::connect(address).await?;
        let request = request()?;
        let mut response = Box::pin(
            owner
                .exchange
                .as_ref()
                .unwrap()
                .send(request)
                .first_answer(),
        );
        let observed = tokio::select! {
            response = &mut response => Err(format!("unexpected TCP completion: {response:?}").into()),
            received = received => received.map_err(Into::into),
        };
        drop(response);
        owner.close().await?;
        observed
    };
    let (cancelled, completed) = tokio::join!(timeout(DEADLINE, client), timeout(DEADLINE, peer));
    let cancelled: TestResult = cancelled?;
    cancelled?;
    completed??;
    drop(listener);
    let _rebound = TcpListener::bind(address).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unlistening_reserved_tcp_port_eventually_reports_connection_refused() -> TestResult {
    let reserved = tokio::net::TcpSocket::new_v4()?;
    reserved.bind("127.0.0.1:0".parse()?)?;
    // Windows may report refusal after the ordinary500ms request budget expires.
    // This isolated classification probe allows3s; it does not widen other probes.
    let error = match OwnedTcp::connect_with_timeout(reserved.local_addr()?, DEADLINE).await {
        Ok(owner) => {
            owner.close().await?;
            return Err("unlistening TCP port accepted a connection".into());
        }
        Err(error) => error,
    };
    assert!(
        matches!(error.downcast_ref::<NetError>(), Some(NetError::Io(error)) if error.kind() == std::io::ErrorKind::ConnectionRefused),
        "{error:?}"
    );
    Ok(())
}
