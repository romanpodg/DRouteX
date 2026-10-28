use super::bounded_io::{Budget, MESSAGE_BYTES};
use super::guarded::{Failure, TcpAttempt};
use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn count(value: &AtomicUsize) -> usize {
    value.load(Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug)]
enum Protocol {
    Udp,
    Tcp,
}
struct Sample {
    reply: Result<DnsResponse, Failure>,
    budget: Arc<Budget>,
}

fn request_from(bytes: &[u8]) -> TestResult<DnsRequest> {
    Ok(DnsRequest::new(
        Message::from_vec(bytes)?,
        DnsRequestOptions::default(),
    ))
}
async fn closed(socket: &mut TcpStream) -> TestResult {
    let mut byte = [0];
    match timeout(DEADLINE, socket.read(&mut byte)).await? {
        Ok(0) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => Ok(()),
        other => Err(format!("peer was not closed: {other:?}").into()),
    }
}
async fn tcp_question(socket: &mut TcpStream) -> TestResult<Vec<u8>> {
    let length = socket.read_u16().await?;
    let mut bytes = vec![0; usize::from(length)];
    socket.read_exact(&mut bytes).await?;
    Ok(bytes)
}
async fn frame(socket: &mut TcpStream, query: &[u8], bytes: &[u8]) -> TestResult {
    let mut answer = bytes.to_vec();
    if answer.len() >= 2 {
        answer[..2].copy_from_slice(&query[..2]);
    }
    socket.write_u16(u16::try_from(answer.len())?).await?;
    socket.write_all(&answer).await?;
    Ok(())
}
async fn exchange(
    protocol: Protocol,
    bind: &str,
    query: &[u8],
    answers: &[Vec<u8>],
) -> TestResult<Sample> {
    let request = request_from(query)?;
    match protocol {
        Protocol::Udp => {
            let peer = UdpSocket::bind(bind).await?;
            let budget = Budget::new(peer.local_addr()?);
            let server = async {
                let mut bytes = [0; MESSAGE_BYTES];
                let (length, client) = peer.recv_from(&mut bytes).await?;
                assert_eq!(&bytes[12..length], &query[12..]);
                for answer in answers {
                    let mut answer = answer.clone(); // Fixture response needs this request's chosen ID.
                    if answer.len() >= 2 {
                        answer[..2].copy_from_slice(&bytes[..2]);
                    }
                    peer.send_to(&answer, client).await?;
                }
                TestResult::Ok(client)
            };
            let (reply, client) = tokio::join!(
                super::guarded::udp(Arc::clone(&budget), request),
                timeout(DEADLINE, server)
            );
            let client = client.map_err(|error| {
                format!("UDP peer did not receive query: {error}; client={reply:?}")
            })??;
            let _released_client = UdpSocket::bind(client).await?;
            drop(peer);
            let _released_peer = UdpSocket::bind(budget.peer).await?;
            assert_eq!(count(&budget.counters.sockets), 0);
            Ok(Sample { reply, budget })
        }
        Protocol::Tcp => {
            let listener = TcpListener::bind(bind).await?;
            let budget = Budget::new(listener.local_addr()?);
            let server = async {
                let (mut socket, _) = listener.accept().await?;
                let bytes = tcp_question(&mut socket).await?;
                assert_eq!(&bytes[12..], &query[12..]);
                for answer in answers {
                    frame(&mut socket, &bytes, answer).await?;
                }
                closed(&mut socket).await
            };
            let client = async {
                let mut owner = TcpAttempt::connect(Arc::clone(&budget)).await?;
                let reply = owner.query(request).await;
                owner.finish().await?;
                TestResult::Ok(reply)
            };
            let (reply, completed) = tokio::join!(client, timeout(DEADLINE, server));
            completed??;
            drop(listener);
            let _rebound = TcpListener::bind(budget.peer).await?;
            assert_eq!(count(&budget.counters.sockets), 0);
            Ok(Sample {
                reply: reply?,
                budget,
            })
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Bad {
    Name,
    Type,
    Class,
    Opcode,
    QueryBit,
    NoQuestion,
    ManyQuestions,
    DeclaredCount,
    Compression,
    Trailing,
    RecordClass,
    EmptyRecord,
    IncompleteOpt,
}
fn bad_answer(case: Bad) -> Vec<u8> {
    let mut answer = ANSWER.to_vec();
    match case {
        Bad::Name => answer[13..20].copy_from_slice(b"foreign"),
        Bad::Type => answer[QUERY.len() - 4..QUERY.len() - 2].copy_from_slice(&28u16.to_be_bytes()),
        Bad::Class => answer[QUERY.len() - 2..QUERY.len()].copy_from_slice(&3u16.to_be_bytes()),
        Bad::Opcode => answer[2] |= 0x08,
        Bad::QueryBit => answer[2] &= 0x7f,
        Bad::NoQuestion => {
            answer.truncate(12);
            answer[4..8].fill(0);
        }
        Bad::ManyQuestions => {
            answer = QUERY.to_vec();
            answer[2] = 0x81;
            answer[4..6].copy_from_slice(&2u16.to_be_bytes());
            answer.extend_from_slice(&QUERY[12..]);
        }
        Bad::DeclaredCount => answer[6..8].copy_from_slice(&u16::MAX.to_be_bytes()),
        Bad::Compression => {
            answer[12] = 0xc0;
            answer[13] = 0x0c;
        }
        Bad::Trailing => answer.extend_from_slice(b"trailing"),
        Bad::RecordClass => {
            answer[QUERY.len() + 4..QUERY.len() + 6].copy_from_slice(&3u16.to_be_bytes())
        }
        Bad::EmptyRecord => {
            answer.truncate(ANSWER.len() - 4);
            let length = answer.len();
            answer[length - 2..].fill(0);
        }
        Bad::IncompleteOpt => {
            answer[10..12].copy_from_slice(&1u16.to_be_bytes());
            answer.extend_from_slice(
                b"\x00\x00\x29\x04\xd0\x00\x00\x00\x00\x00\x05\xfd\xe9\x00\x02\x2a",
            );
        }
    }
    answer
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_tcp_rejects_each_foreign_or_malformed_candidate_before_a_valid_answer()
-> TestResult {
    for case in [
        Bad::Name,
        Bad::Type,
        Bad::Class,
        Bad::Opcode,
        Bad::QueryBit,
        Bad::NoQuestion,
        Bad::ManyQuestions,
        Bad::DeclaredCount,
        Bad::Compression,
        Bad::Trailing,
        Bad::RecordClass,
        Bad::EmptyRecord,
        Bad::IncompleteOpt,
    ] {
        let sample = exchange(
            Protocol::Tcp,
            "127.0.0.1:0",
            QUERY,
            &[bad_answer(case), ANSWER.to_vec()],
        )
        .await?;
        let answer = sample.reply.map_err(|e| format!("{case:?}: {e}"))?;
        assert_eq!(
            answer.queries[0].name.to_ascii(),
            "example.test.",
            "{case:?}"
        );
        assert_eq!(
            answer.queries[0].query_type,
            hickory_proto::rr::RecordType::A,
            "{case:?}"
        );
        assert_eq!(
            answer.queries[0].query_class,
            hickory_proto::rr::DNSClass::IN,
            "{case:?}"
        );
        assert_eq!(answer.as_buffer().len(), ANSWER.len(), "{case:?}");
        if !matches!(
            case,
            Bad::QueryBit | Bad::DeclaredCount | Bad::Compression | Bad::EmptyRecord
        ) {
            assert_eq!(count(&sample.budget.counters.rejected), 1, "{case:?}");
        }
        assert_eq!(count(&sample.budget.counters.opened), 1);
        assert_eq!(count(&sample.budget.counters.write_bytes), QUERY.len() + 2);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_udp_never_accepts_foreign_or_malformed_only_answers() -> TestResult {
    for case in [
        Bad::Name,
        Bad::Type,
        Bad::Class,
        Bad::Opcode,
        Bad::QueryBit,
        Bad::NoQuestion,
        Bad::ManyQuestions,
        Bad::DeclaredCount,
        Bad::Compression,
        Bad::Trailing,
        Bad::RecordClass,
        Bad::EmptyRecord,
        Bad::IncompleteOpt,
    ] {
        let sample = exchange(Protocol::Udp, "127.0.0.1:0", QUERY, &[bad_answer(case)])
            .await
            .map_err(|error| format!("{case:?}: {error}"))?;
        assert!(sample.reply.is_err(), "{case:?}: {:?}", sample.reply);
        assert_eq!(count(&sample.budget.counters.datagrams), 1, "{case:?}");
        assert_eq!(count(&sample.budget.counters.opened), 1);
        assert_eq!(count(&sample.budget.counters.write_operations), 1);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_ipv6_udp_accepts_the_independent_answer() -> TestResult {
    let sample = exchange(Protocol::Udp, "[::1]:0", QUERY, &[ANSWER.to_vec()]).await?;
    assert_eq!(sample.reply?.answers[0].data.to_string(), "203.0.113.9");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_ipv6_tcp_accepts_the_independent_answer() -> TestResult {
    let sample = exchange(Protocol::Tcp, "[::1]:0", QUERY, &[ANSWER.to_vec()]).await?;
    assert_eq!(sample.reply?.answers[0].data.to_string(), "203.0.113.9");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_ordinary_and_auxiliary_record_types_use_real_udp_and_tcp() -> TestResult {
    for case in super::independent::record_cases() {
        let mut query = QUERY.to_vec();
        let offset = query.len() - 4;
        query[offset..offset + 2].copy_from_slice(&case.code.to_be_bytes());
        let mut answer = query.clone(); // Independent wire question must remain in the response.
        answer[2..4].copy_from_slice(b"\x81\x80");
        answer[6..8].copy_from_slice(&1u16.to_be_bytes());
        answer.extend_from_slice(b"\xc0\x0c");
        answer.extend_from_slice(&case.code.to_be_bytes());
        answer.extend_from_slice(b"\x00\x01\x00\x00\x00\x3c");
        answer.extend_from_slice(&u16::try_from(case.data.len())?.to_be_bytes());
        answer.extend_from_slice(case.data);
        for protocol in [Protocol::Udp, Protocol::Tcp] {
            let sample = exchange(
                protocol,
                "127.0.0.1:0",
                &query,
                std::slice::from_ref(&answer),
            )
            .await?;
            assert_eq!(
                sample.reply?.answers[0].data.to_string().trim(),
                case.expected,
                "{protocol:?} type {}",
                case.code
            );
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_dns_name_matching_ignores_presentation_case() -> TestResult {
    let mut answer = ANSWER.to_vec();
    answer[13..20].copy_from_slice(b"EXAMPLE");
    for protocol in [Protocol::Udp, Protocol::Tcp] {
        let sample = exchange(
            protocol,
            "127.0.0.1:0",
            QUERY,
            std::slice::from_ref(&answer),
        )
        .await?;
        assert_eq!(sample.reply?.queries[0].name.to_ascii(), "EXAMPLE.test.");
        assert_eq!(count(&sample.budget.counters.rejected), 0);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_tcp_rejects_foreign_then_accepts_the_original_question() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let budget = Budget::new(listener.local_addr()?);
    let peer = async {
        let (mut socket, _) = listener.accept().await?;
        let length = socket.read_u16().await?;
        let mut bytes = vec![0; usize::from(length)];
        socket.read_exact(&mut bytes).await?;
        for foreign in [true, false] {
            let response = response_for(&bytes, foreign);
            socket.write_u16(u16::try_from(response.len())?).await?;
            socket.write_all(&response).await?;
        }
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).await?, 0);
        TestResult::Ok(())
    };
    let client = async {
        let mut owner = TcpAttempt::connect(Arc::clone(&budget)).await?;
        let result = owner.query(request()?).await;
        owner.finish().await?;
        TestResult::Ok(result?)
    };
    let (response, completed) = tokio::join!(client, timeout(DEADLINE, peer));
    completed??;
    assert_eq!(response?.queries[0].name.to_ascii(), "example.test.");
    assert_eq!(
        budget
            .counters
            .rejected
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(
        budget
            .counters
            .sockets
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_udp_rejects_trailing_input_without_reissuing_the_request() -> TestResult {
    let peer = UdpSocket::bind("127.0.0.1:0").await?;
    let budget = Budget::new(peer.local_addr()?);
    let server = async {
        let mut bytes = [0; 512];
        let (length, address) = peer.recv_from(&mut bytes).await?;
        let mut response = response_for(&bytes[..length], false);
        response.extend_from_slice(b"trailing");
        peer.send_to(&response, address).await?;
        TestResult::Ok(address)
    };
    let (response, address) = tokio::join!(
        super::guarded::udp(Arc::clone(&budget), request()?),
        timeout(DEADLINE, server),
    );
    assert!(
        matches!(response, Err(super::guarded::Failure::Rejected)),
        "{response:?}"
    );
    let _released = UdpSocket::bind(address??).await?;
    assert_eq!(
        budget
            .counters
            .sockets
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
    Ok(())
}

fn io_violation(error: &std::io::Error, expected: super::bounded_io::Violation) -> bool {
    error
        .get_ref()
        .and_then(|error| error.downcast_ref::<super::bounded_io::Violation>())
        == Some(&expected)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_udp_counts_internally_discarded_datagrams_and_stops_at_sixteen() -> TestResult {
    use super::bounded_io::{UDP_RECEIVES, Violation};
    for foreign_count in [UDP_RECEIVES - 1, UDP_RECEIVES] {
        let peer = UdpSocket::bind("127.0.0.1:0").await?;
        let budget = Budget::new(peer.local_addr()?);
        let server = async {
            let mut bytes = [0; 512];
            let (length, address) = peer.recv_from(&mut bytes).await?;
            let mut wrong = response_for(&bytes[..length], false);
            wrong[0] ^= 1;
            for _ in 0..foreign_count {
                peer.send_to(&wrong, address).await?;
            }
            peer.send_to(&response_for(&bytes[..length], false), address)
                .await?;
            TestResult::Ok(address)
        };
        let (reply, address) = tokio::join!(
            super::guarded::udp(Arc::clone(&budget), request()?),
            timeout(DEADLINE, server)
        );
        if foreign_count == UDP_RECEIVES - 1 {
            assert_eq!(reply?.answers[0].data.to_string(), "203.0.113.9");
        } else {
            assert!(
                matches!(reply, Err(Failure::Transport(NetError::Io(ref error))) if io_violation(error, Violation::ReceiveBudget)),
                "{reply:?}"
            );
        }
        assert_eq!(count(&budget.counters.datagrams), UDP_RECEIVES);
        assert_eq!(count(&budget.counters.rejected), 0); // All false IDs were hidden by Hickory.
        assert_eq!(count(&budget.counters.write_operations), 1);
        let _released = UdpSocket::bind(address??).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_tcp_candidate_budget_accepts_sixteenth_valid_or_refuses_sixteenth_foreign()
-> TestResult {
    for foreign_count in [super::guarded::CANDIDATES - 1, super::guarded::CANDIDATES] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let budget = Budget::new(listener.local_addr()?);
        let server = async {
            let (mut socket, _) = listener.accept().await?;
            let bytes = tcp_question(&mut socket).await?;
            for index in 0..foreign_count {
                frame(&mut socket, &bytes, &bad_answer(Bad::Name)).await?;
                // Do not overflow Hickory's response queue: prove each delivered
                // candidate reached the guard before sending another one.
                while count(&budget.counters.rejected) <= index {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            if foreign_count < super::guarded::CANDIDATES {
                frame(&mut socket, &bytes, ANSWER).await?;
            }
            closed(&mut socket).await
        };
        let client = async {
            let mut owner = TcpAttempt::connect(Arc::clone(&budget)).await?;
            let reply = owner.query(request()?).await;
            owner.finish().await?;
            TestResult::Ok(reply)
        };
        let (reply, completed) = tokio::join!(client, timeout(DEADLINE, server));
        completed??;
        let reply = reply?;
        if foreign_count < super::guarded::CANDIDATES {
            assert_eq!(reply?.queries[0].name.to_ascii(), "example.test.");
        } else {
            assert!(matches!(reply, Err(Failure::Candidates)), "{reply:?}");
        }
        assert_eq!(count(&budget.counters.rejected), foreign_count);
        assert_eq!(count(&budget.counters.sockets), 0);
        assert_eq!(count(&budget.counters.write_bytes), QUERY.len() + 2);
        drop(listener);
        let _rebound = TcpListener::bind(budget.peer).await?;
    }
    Ok(())
}

fn txt_packet(length: usize, edns: bool) -> (Vec<u8>, Vec<u8>) {
    assert!(length >= 43 && length <= u16::MAX as usize);
    let mut query = QUERY.to_vec();
    query[27] = 16;
    let mut answer = query.clone(); // The fixture constructs both wire packets without library serialization.
    answer[2..4].copy_from_slice(b"\x81\x80");
    answer[6..8].copy_from_slice(&1u16.to_be_bytes());
    answer.extend_from_slice(b"\xc0\x0c\x00\x10\x00\x01\x00\x00\x00\x3c");
    let rdata_length = length - 42;
    answer.extend_from_slice(&u16::try_from(rdata_length).unwrap().to_be_bytes());
    while answer.len() < length {
        let characters = (length - answer.len() - 1).min(255);
        answer.push(u8::try_from(characters).unwrap());
        answer.resize(answer.len() + characters, b'a');
    }
    if edns {
        query[10..12].copy_from_slice(&1u16.to_be_bytes());
        query.extend_from_slice(b"\x00\x00\x29\x10\x00\x00\x00\x00\x00\x00\x00");
    }
    (query, answer)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_message_ceiling_accepts_4096_and_udp_never_truncates_to_512() -> TestResult {
    let (query, answer) = txt_packet(MESSAGE_BYTES, true);
    for protocol in [Protocol::Udp, Protocol::Tcp] {
        let sample = exchange(
            protocol,
            "127.0.0.1:0",
            &query,
            std::slice::from_ref(&answer),
        )
        .await?;
        assert_eq!(
            sample.reply?.as_buffer().len(),
            MESSAGE_BYTES,
            "{protocol:?}"
        );
    }
    for length in [513, MESSAGE_BYTES + 1, 65507] {
        let (query, mut answer) = txt_packet(if length == 513 { 512 } else { length }, false);
        if length == 513 {
            answer.push(0);
        } // First512 bytes form a valid TXT answer.
        let sample = exchange(Protocol::Udp, "127.0.0.1:0", &query, &[answer]).await?;
        assert!(
            sample.reply.is_err(),
            "oversized datagram {length}: {:?}",
            sample.reply
        );
    }
    let (query, answer) = txt_packet(MESSAGE_BYTES + 1, false);
    let sample = exchange(Protocol::Tcp, "127.0.0.1:0", &query, &[answer]).await?;
    assert!(sample.reply.is_err());
    assert_eq!(count(&sample.budget.counters.rejected), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bounded_tcp_limits_completed_read_and_write_operations_before_hidden_parsing() -> TestResult
{
    use super::bounded_io::{IO_OPERATIONS, Provider, Violation};
    use hickory_net::runtime::{RuntimeProvider, iocompat::AsyncIoStdAsTokio};
    for reading in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let budget = Budget::new(listener.local_addr()?);
        let server = async {
            let (mut socket, _) = listener.accept().await?;
            if reading {
                socket.write_all(&vec![42; IO_OPERATIONS + 1]).await?;
            } else {
                let mut bytes = vec![0; IO_OPERATIONS];
                socket.read_exact(&mut bytes).await?;
                assert!(bytes.iter().all(|byte| *byte == 42));
            }
            closed(&mut socket).await
        };
        let client = async {
            let stream = Provider(Arc::clone(&budget))
                .connect_tcp(budget.peer, None, None)
                .await?;
            let mut stream = AsyncIoStdAsTokio(stream);
            let mut byte = [42];
            for _ in 0..IO_OPERATIONS {
                if reading {
                    stream.read_exact(&mut byte).await?;
                } else {
                    stream.write_all(&byte).await?;
                }
            }
            let error = if reading {
                stream.read_exact(&mut byte).await.unwrap_err()
            } else {
                stream.write_all(&byte).await.unwrap_err()
            };
            assert!(
                io_violation(
                    &error,
                    if reading {
                        Violation::ReadBudget
                    } else {
                        Violation::WriteBudget
                    }
                ),
                "{error:?}"
            );
            drop(stream);
            TestResult::Ok(())
        };
        let (result, completed) =
            tokio::join!(timeout(DEADLINE, client), timeout(DEADLINE, server));
        result??;
        completed??;
        assert_eq!(
            count(if reading {
                &budget.counters.read_operations
            } else {
                &budget.counters.write_operations
            }),
            IO_OPERATIONS
        );
        assert_eq!(count(&budget.counters.sockets), 0);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bounded_tcp_limits_total_read_and_write_bytes_without_overshoot() -> TestResult {
    use super::bounded_io::{Provider, TCP_READ_BYTES, Violation, WRITE_BYTES};
    use hickory_net::runtime::{RuntimeProvider, iocompat::AsyncIoStdAsTokio};
    for (reading, limit) in [(true, TCP_READ_BYTES), (false, WRITE_BYTES)] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let budget = Budget::new(listener.local_addr()?);
        let server = async {
            let (mut socket, _) = listener.accept().await?;
            if reading {
                socket.write_all(&vec![42; limit + 1]).await?;
            } else {
                let mut bytes = vec![0; limit];
                socket.read_exact(&mut bytes).await?;
                assert!(bytes.iter().all(|byte| *byte == 42));
            }
            closed(&mut socket).await
        };
        let client = async {
            let stream = Provider(Arc::clone(&budget))
                .connect_tcp(budget.peer, None, None)
                .await?;
            let mut stream = AsyncIoStdAsTokio(stream);
            let mut bytes = vec![42; limit];
            if reading {
                stream.read_exact(&mut bytes).await?;
            } else {
                stream.write_all(&bytes).await?;
            }
            let error = if reading {
                stream.read_exact(&mut [0]).await.unwrap_err()
            } else {
                stream.write_all(&[42]).await.unwrap_err()
            };
            assert!(
                io_violation(
                    &error,
                    if reading {
                        Violation::ReadBudget
                    } else {
                        Violation::WriteBudget
                    }
                ),
                "{error:?}"
            );
            drop(stream);
            TestResult::Ok(())
        };
        let (result, completed) =
            tokio::join!(timeout(DEADLINE, client), timeout(DEADLINE, server));
        result??;
        completed??;
        assert_eq!(
            count(if reading {
                &budget.counters.read_bytes
            } else {
                &budget.counters.write_bytes
            }),
            limit
        );
        assert_eq!(count(&budget.counters.sockets), 0);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_connect_partial_read_and_partial_write_cancellation_release_and_join() -> TestResult
{
    use super::bounded_io::{Hold, HoldAt};
    for phase in [HoldAt::Connect, HoldAt::Read, HoldAt::Write] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let (reached, mut observed) = tokio::sync::watch::channel(false);
        let budget = Budget::with_hold(listener.local_addr()?, Some(Hold { at: phase, reached }));
        let server = async {
            let (mut socket, _) = listener.accept().await?;
            match phase {
                HoldAt::Connect => (),
                HoldAt::Read => {
                    let bytes = tcp_question(&mut socket).await?;
                    frame(&mut socket, &bytes, ANSWER).await?;
                }
                HoldAt::Write => {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).await?;
                    assert_eq!(byte, [0]); // The first byte of this small frame's length.
                }
            }
            closed(&mut socket).await
        };
        let client = async {
            if phase == HoldAt::Connect {
                let mut connecting = Box::pin(TcpAttempt::connect(Arc::clone(&budget)));
                let cancelled: TestResult = tokio::select! {
                    result = &mut connecting => {
                        if let Ok(owner) = result { owner.finish().await?; }
                        Err("connection completed before controlled cancellation".into())
                    }
                    reached = observed.wait_for(|value| *value) => reached.map(|_| ()).map_err(Into::into),
                };
                drop(connecting);
                cancelled?;
                assert_eq!(count(&budget.counters.write_bytes), 0);
            } else {
                let mut owner = TcpAttempt::connect(Arc::clone(&budget)).await?;
                let mut query = Box::pin(owner.query(request()?));
                let cancelled: TestResult = tokio::select! {
                    result = &mut query => Err(format!("request completed before {phase:?} cancellation: {result:?}").into()),
                    reached = observed.wait_for(|value| *value) => reached.map(|_| ()).map_err(Into::into),
                };
                drop(query);
                // Cancellation cannot make this owner issue another wire query.
                assert!(matches!(
                    owner.query(request()?).await,
                    Err(Failure::Cleanup)
                ));
                owner.finish().await?;
                cancelled?;
                assert_eq!(
                    count(if phase == HoldAt::Read {
                        &budget.counters.read_bytes
                    } else {
                        &budget.counters.write_bytes
                    }),
                    1
                );
            }
            TestResult::Ok(())
        };
        let (result, completed) =
            tokio::join!(timeout(DEADLINE, client), timeout(DEADLINE, server));
        result??;
        completed??;
        assert_eq!(count(&budget.counters.sockets), 0, "{phase:?}");
        assert_eq!(count(&budget.counters.opened), 1);
        drop(listener);
        let _rebound = TcpListener::bind(budget.peer).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_udp_cancellation_drops_the_owned_receive_future_and_socket() -> TestResult {
    let peer = UdpSocket::bind("127.0.0.1:0").await?;
    let budget = Budget::new(peer.local_addr()?);
    let (sent, received) = tokio::sync::oneshot::channel();
    let server = async {
        let mut bytes = [0; 512];
        let (_, address) = peer.recv_from(&mut bytes).await?;
        sent.send(address).map_err(|_| "cancelled observer")?;
        TestResult::Ok(())
    };
    let client = async {
        let mut query = Box::pin(super::guarded::udp(Arc::clone(&budget), request()?));
        let address = tokio::select! {
            result = &mut query => return Err(format!("unexpected completion before cancellation: {result:?}").into()),
            address = received => address?,
        };
        drop(query);
        assert_eq!(count(&budget.counters.sockets), 0);
        let _released = UdpSocket::bind(address).await?;
        TestResult::Ok(())
    };
    let (result, completed) = tokio::join!(timeout(DEADLINE, client), timeout(DEADLINE, server));
    result??;
    completed??;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_tcp_keeps_the_original_deadline_across_connect_and_reply() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let budget = Budget::new(listener.local_addr()?);
    let server = async {
        let (mut socket, _) = listener.accept().await?;
        let bytes = tcp_question(&mut socket).await?;
        let late = tokio::select! {
            closed = closed(&mut socket) => { closed?; false },
            _ = tokio::time::sleep_until(budget.deadline + Duration::from_millis(100)) => true,
        };
        if late {
            // A caller that incorrectly resets500ms at query time accepts this.
            frame(&mut socket, &bytes, ANSWER).await?;
            closed(&mut socket).await?;
        }
        TestResult::Ok(())
    };
    let client = async {
        tokio::time::sleep_until(budget.deadline - Duration::from_millis(250)).await;
        let mut owner = TcpAttempt::connect(Arc::clone(&budget)).await?;
        let reply = owner.query(request()?).await;
        owner.finish().await?;
        TestResult::Ok(reply)
    };
    let (reply, completed) = tokio::join!(client, timeout(DEADLINE, server));
    completed??;
    assert!(
        matches!(reply?, Err(Failure::Deadline)),
        "shared deadline was not retained"
    );
    assert_eq!(count(&budget.counters.sockets), 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_dropped_replies_and_invalid_trickle_never_extend_the_attempt() -> TestResult {
    for protocol in [Protocol::Udp, Protocol::Tcp] {
        let sample = exchange(protocol, "127.0.0.1:0", QUERY, &[]).await?;
        assert!(
            matches!(
                sample.reply,
                Err(Failure::Deadline | Failure::Transport(NetError::Timeout))
            ),
            "{protocol:?}: {:?}",
            sample.reply
        );
        assert_eq!(count(&sample.budget.counters.opened), 1);
    }
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let budget = Budget::new(listener.local_addr()?);
    let server = async {
        let (mut socket, _) = listener.accept().await?;
        let bytes = tcp_question(&mut socket).await?;
        for _ in 0..5 {
            frame(&mut socket, &bytes, &bad_answer(Bad::Name)).await?;
            tokio::time::sleep(Duration::from_millis(75)).await;
        }
        closed(&mut socket).await
    };
    let client = async {
        let mut owner = TcpAttempt::connect(Arc::clone(&budget)).await?;
        let reply = owner.query(request()?).await;
        owner.finish().await?;
        TestResult::Ok(reply)
    };
    let (reply, completed) = tokio::join!(client, timeout(DEADLINE, server));
    completed??;
    assert!(matches!(
        reply?,
        Err(Failure::Deadline | Failure::Transport(NetError::Timeout))
    ));
    assert_eq!(count(&budget.counters.rejected), 5);
    assert_eq!(count(&budget.counters.sockets), 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bounded_pending_udp_polls_do_not_spend_successful_io_budget() -> TestResult {
    use super::bounded_io::Provider;
    use hickory_net::runtime::{DnsUdpSocket, RuntimeProvider};
    let peer = UdpSocket::bind("127.0.0.1:0").await?;
    let budget = Budget::new(peer.local_addr()?);
    let socket = Provider(Arc::clone(&budget))
        .bind_udp("127.0.0.1:0".parse()?, budget.peer)
        .await?;
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    for _ in 0..100 {
        assert!(
            socket
                .poll_recv_from(&mut context, &mut [0; 512])
                .is_pending()
        );
    }
    assert_eq!(count(&budget.counters.datagrams), 0);
    drop(socket);
    assert_eq!(count(&budget.counters.sockets), 0);
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum ForeignSource {
    Port,
    Ip,
    Id,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_udp_refuses_observed_foreign_source_and_id_before_the_valid_packet() -> TestResult
{
    for foreign in [ForeignSource::Port, ForeignSource::Ip, ForeignSource::Id] {
        let peer = UdpSocket::bind("127.0.0.1:0").await?;
        let budget = Budget::new(peer.local_addr()?);
        let server = async {
            let mut bytes = [0; 512];
            let (length, client) = peer.recv_from(&mut bytes).await?;
            let mut bad = response_for(&bytes[..length], false);
            *bad.last_mut().unwrap() = 66;
            match foreign {
                ForeignSource::Id => {
                    bad[0] ^= 1;
                    peer.send_to(&bad, client).await?;
                }
                ForeignSource::Ip | ForeignSource::Port => {
                    let bind = if matches!(foreign, ForeignSource::Ip) {
                        SocketAddr::new("127.0.0.2".parse()?, budget.peer.port())
                    } else {
                        "127.0.0.1:0".parse()?
                    };
                    let stranger = UdpSocket::bind(bind).await?;
                    let actual = stranger.local_addr()?;
                    assert_eq!(
                        actual.port() == budget.peer.port(),
                        matches!(foreign, ForeignSource::Ip)
                    );
                    assert_eq!(
                        actual.ip() == budget.peer.ip(),
                        matches!(foreign, ForeignSource::Port)
                    );
                    stranger.send_to(&bad, client).await?;
                }
            }
            // The adapter has actually received the foreign packet before the
            // fixture sends a valid one, so packet reordering cannot mask refusal.
            while count(&budget.counters.datagrams) == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            peer.send_to(&response_for(&bytes[..length], false), client)
                .await?;
            TestResult::Ok(client)
        };
        let (reply, address) = tokio::join!(
            super::guarded::udp(Arc::clone(&budget), request()?),
            timeout(DEADLINE, server)
        );
        assert_eq!(
            reply?.answers[0].data.to_string(),
            "203.0.113.9",
            "{foreign:?}"
        );
        assert_eq!(count(&budget.counters.datagrams), 2);
        assert_eq!(count(&budget.counters.write_operations), 1);
        let _released = UdpSocket::bind(address??).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_tcp_bytewise_frames_and_inconsistent_lengths_preserve_failure_boundaries()
-> TestResult {
    // The complete bytewise frame is followed by EOF inside length/body cases.
    for (advertised, body, bytewise, accepted) in [
        (ANSWER.len(), ANSWER.len(), true, true),
        (0, 0, false, false),
        (1, 0, false, false),
        (u16::MAX as usize, 8, false, false),
        (ANSWER.len() + 1, ANSWER.len(), false, false),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let budget = Budget::new(listener.local_addr()?);
        let server = async {
            let (mut socket, _) = listener.accept().await?;
            let bytes = tcp_question(&mut socket).await?;
            let answer = response_for(&bytes, false);
            let mut wire = u16::try_from(advertised)?.to_be_bytes().to_vec();
            wire.extend_from_slice(&answer[..body]);
            if bytewise {
                for byte in wire {
                    socket.write_all(&[byte]).await?;
                    tokio::task::yield_now().await;
                }
            } else {
                socket.write_all(&wire).await?;
                socket.shutdown().await?;
            }
            closed(&mut socket).await
        };
        let client = async {
            let mut owner = TcpAttempt::connect(Arc::clone(&budget)).await?;
            let reply = owner.query(request()?).await;
            owner.finish().await?;
            TestResult::Ok(reply)
        };
        let (reply, completed) = tokio::join!(client, timeout(DEADLINE, server));
        completed??;
        let reply = reply?;
        assert_eq!(
            reply.is_ok(),
            accepted,
            "advertised={advertised} body={body}: {reply:?}"
        );
        assert_eq!(count(&budget.counters.sockets), 0);
        drop(listener);
        let _rebound = TcpListener::bind(budget.peer).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_attempt_opens_no_socket_and_provider_refuses_a_different_numeric_target()
-> TestResult {
    use super::bounded_io::{Provider, Violation};
    use hickory_net::runtime::RuntimeProvider;
    let peer = UdpSocket::bind("127.0.0.1:0").await?;
    let mut budget = Budget::new(peer.local_addr()?);
    Arc::get_mut(&mut budget).unwrap().deadline = tokio::time::Instant::now();
    assert!(matches!(
        super::guarded::udp(Arc::clone(&budget), request()?).await,
        Err(Failure::Deadline)
    ));
    assert!(matches!(
        TcpAttempt::connect(Arc::clone(&budget)).await,
        Err(Failure::Deadline)
    ));
    assert_eq!(count(&budget.counters.opened), 0);
    let budget = Budget::new(peer.local_addr()?);
    let wrong: SocketAddr = "127.0.0.2:1".parse()?;
    let error = match Provider(Arc::clone(&budget))
        .connect_tcp(wrong, None, None)
        .await
    {
        Ok(_) => return Err("different target was connected".into()),
        Err(error) => error,
    };
    assert!(io_violation(&error, Violation::Peer));
    assert_eq!(count(&budget.counters.opened), 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_udp_bind_releases_only_the_uncommitted_socket_reservation() -> TestResult {
    use super::bounded_io::{Provider, Violation};
    use hickory_net::runtime::{DnsUdpSocket, RuntimeProvider};
    let occupied = UdpSocket::bind("127.0.0.1:0").await?;
    let peer = UdpSocket::bind("127.0.0.1:0").await?;
    let budget = Budget::new(peer.local_addr()?);
    let provider = Provider(Arc::clone(&budget));
    let error = match provider.bind_udp(occupied.local_addr()?, budget.peer).await {
        Ok(_) => return Err("occupied UDP port unexpectedly bound".into()),
        Err(error) => error,
    };
    assert!(
        matches!(
            error.kind(),
            std::io::ErrorKind::AddrInUse | std::io::ErrorKind::PermissionDenied
        ),
        "{error:?}"
    );
    assert_eq!(
        count(&budget.counters.opened),
        0,
        "failed bind retained the socket reservation"
    );
    let socket = provider
        .bind_udp("127.0.0.1:0".parse()?, budget.peer)
        .await?;
    assert_eq!(socket.send_to(QUERY, budget.peer).await?, QUERY.len());
    let mut bytes = [0; 512];
    let (length, address) = timeout(DEADLINE, peer.recv_from(&mut bytes)).await??;
    assert_eq!(&bytes[..length], QUERY);
    drop(socket);
    assert_eq!(count(&budget.counters.sockets), 0);
    assert_eq!(count(&budget.counters.opened), 1);
    let _released = UdpSocket::bind(address).await?;
    let error = match provider.bind_udp("127.0.0.1:0".parse()?, budget.peer).await {
        Ok(_) => return Err("completed attempt opened another socket".into()),
        Err(error) => error,
    };
    assert!(io_violation(&error, Violation::ExtraSocket));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guarded_tcp_discards_wrong_id_before_the_valid_id_on_the_same_stream() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let budget = Budget::new(listener.local_addr()?);
    let server = async {
        let (mut socket, _) = listener.accept().await?;
        let query = tcp_question(&mut socket).await?;
        let mut bad = response_for(&query, false);
        bad[0] ^= 1;
        *bad.last_mut().unwrap() = 66;
        socket.write_u16(u16::try_from(bad.len())?).await?;
        socket.write_all(&bad).await?;
        frame(&mut socket, &query, ANSWER).await?;
        closed(&mut socket).await?;
        TestResult::Ok(u16::from_be_bytes([query[0], query[1]]))
    };
    let client = async {
        let mut owner = TcpAttempt::connect(Arc::clone(&budget)).await?;
        let reply = owner.query(request()?).await;
        owner.finish().await?;
        TestResult::Ok(reply?)
    };
    let (reply, expected_id) = tokio::join!(client, timeout(DEADLINE, server));
    let reply = reply?;
    assert_eq!(reply.metadata.id, expected_id??);
    assert_eq!(reply.answers[0].data.to_string(), "203.0.113.9");
    assert_eq!(count(&budget.counters.rejected), 0);
    assert_eq!(count(&budget.counters.read_bytes), 2 * (ANSWER.len() + 2));
    assert_eq!(count(&budget.counters.write_bytes), QUERY.len() + 2);
    assert_eq!(count(&budget.counters.sockets), 0);
    drop(listener);
    let _rebound = TcpListener::bind(budget.peer).await?;
    Ok(())
}
