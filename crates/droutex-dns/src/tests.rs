use super::*;
#[path = "tests/forwarding.rs"]
mod forwarding;
use crate::{
    runtime,
    wire::{self, Processor},
};
use droutex_common::ProfileId;
use droutex_config::{
    CompletePublication, DnsAction, InputFormat, ReferenceCatalog, parse_configuration,
    snapshot::{SnapshotMetadata, SnapshotSequence},
};
use droutex_core::deployment::{Artifacts, Cancellation};
use droutex_policy::PolicySelection;
use droutex_security::{SigningKey, SigningKeyId, SnapshotTrust};
use hickory_proto::{
    op::{Message, MessageType, ResponseCode},
    rr::{
        RData, Record,
        rdata::{A, TXT},
    },
};
use serde_json::{Value, json};
use std::{
    future::{Future, poll_fn},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpSocket, TcpStream},
    sync::{Notify, oneshot},
    task::JoinHandle,
    time::{Instant, sleep, timeout},
};

// Independently specified DNS bytes, not produced by the codec under test.
const QUERY: &[u8] =
    b"\x04\xd2\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01";
fn typed<T: FromStr>(n: usize) -> T
where
    T::Err: fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn text(n: usize) -> String {
    typed::<ListenerId>(n).to_string()
}
fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn keys() -> (SigningKey, SnapshotTrust) {
    // RFC8032 public test seed, never an operational credential.
    let public = hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
        .try_into()
        .unwrap();
    let key=SigningKey::from_pkcs8(SigningKeyId::from_bytes([1;16]).unwrap(),hex("302e020100300506032b6570042204209d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"),&public).unwrap();
    let trust = SnapshotTrust::new(vec![(key.id(), public)]).unwrap();
    (key, trust)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
fn document(udp: SocketAddr, tcp: SocketAddr) -> Value {
    let time = now();
    let source = udp.ip().to_string();
    json!({"schemaVersion":2,
        "profiles":[{"id":text(10),"name":"DNS","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"deny"}}],
        "listeners":[{"id":text(20),"name":"UDP","enabled":true,"bind":udp.to_string(),"transport":"dns_udp"},
                     {"id":text(21),"name":"TCP","enabled":true,"bind":tcp.to_string(),"transport":"dns_tcp"}],
        "endpoints":[{"id":text(30),"name":"DNS endpoint","enabled":true,"profile":text(10),"access":[
            {"transport":"udp","listener":text(20),"grants":[text(40)]},{"transport":"tcp","listener":text(21),"grants":[text(41)]}]}],
        "networkGroups":[{"id":text(50),"name":"Group","trafficProfile":text(10)}],
        "legacyGrants":[{"id":text(40),"source":source,"listener":text(20),"group":text(50),"endpoint":text(30),"lease":{"cause":"manual","issuedAt":time-10,"expiresAt":time+300}},
                        {"id":text(41),"source":source,"listener":text(21),"group":text(50),"endpoint":text(30),"lease":{"cause":"manual","issuedAt":time-10,"expiresAt":time+300}}]})
}
fn apply(deployment: &Deployment, doc: &Value, sequence: u64) {
    let checked = parse_configuration(
        &serde_json::to_vec(doc).unwrap(),
        InputFormat::Json,
        &ReferenceCatalog::default(),
    )
    .unwrap();
    let publication = CompletePublication::new(checked, vec![], &[], &[]).unwrap();
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
    deployment
        .activate(
            envelope,
            Artifacts::new(vec![]).unwrap(),
            &Cancellation::default(),
        )
        .unwrap();
}
#[derive(Default)]
struct Probe {
    calls: AtomicUsize,
    profiles: Mutex<Vec<ProfileId>>,
    entered: Notify,
    release: Notify,
    blocked: bool,
    large: bool,
}
impl Processor for Arc<Probe> {
    async fn answer(
        &self,
        request: &Message,
        selection: PolicySelection<'_, DnsAction>,
        _: &crate::forward::Processing<'_, '_>,
    ) -> Result<Message> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.profiles.lock().unwrap().push(selection.profile());
        self.entered.notify_one();
        if self.blocked {
            self.release.notified().await;
        }
        let mut reply = wire::response(request, ResponseCode::NoError);
        let name = request.queries[0].name.clone();
        reply.answers.push(if self.large {
            Record::from_rdata(name, 60, RData::TXT(TXT::new(vec!["x".repeat(250); 4])))
        } else {
            Record::from_rdata(name, 60, RData::A(A(Ipv4Addr::new(203, 0, 113, 9))))
        });
        Ok(reply)
    }
}
struct Server {
    owner: Arc<Deployment>,
    doc: Value,
    udp: SocketAddr,
    tcp: SocketAddr,
    stop: watch::Sender<bool>,
    handle: JoinHandle<Result<ShutdownReport>>,
}
impl Server {
    async fn start<P: Processor>(ip: IpAddr, processor: P) -> Self {
        Self::start_after(ip, processor, std::future::ready(())).await
    }
    async fn start_after<P: Processor>(
        ip: IpAddr,
        processor: P,
        ready: impl Future<Output = ()> + Send + 'static,
    ) -> Self {
        let udp = UdpSocket::bind((ip, 0)).await.unwrap();
        let tcp = TcpListener::bind((ip, 0)).await.unwrap();
        let udp_addr = udp.local_addr().unwrap();
        let tcp_addr = tcp.local_addr().unwrap();
        let doc = document(udp_addr, tcp_addr);
        let node = DnsNode::new(Deployment::in_memory(typed(1), keys().1));
        apply(node.deployment(), &doc, 1);
        let owner = Arc::clone(node.deployment());
        let (stop, shutdown) = watch::channel(false);
        let handle = start_runtime(
            ready,
            runtime::serve(
                node.deployment,
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
        Self {
            owner,
            doc,
            udp: udp_addr,
            tcp: tcp_addr,
            stop,
            handle,
        }
    }
    async fn close(self) -> ShutdownReport {
        self.stop.send_replace(true);
        let report = timeout(Duration::from_secs(7), self.handle)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        // Actual release/rebind of both transports, not a semaphore assertion.
        let _udp = UdpSocket::bind(self.udp).await.unwrap();
        let _tcp = TcpListener::bind(self.tcp).await.unwrap();
        report
    }
}
async fn start_runtime(
    ready: impl Future<Output = ()> + Send + 'static,
    serving: impl Future<Output = Result<ShutdownReport>> + Send + 'static,
) -> JoinHandle<Result<ShutdownReport>> {
    let (started, observed) = oneshot::channel();
    let handle = tokio::spawn(async move {
        ready.await;
        tokio::pin!(serving);
        let mut started = Some(started);
        poll_fn(|context| {
            let outcome = serving.as_mut().poll(context);
            if let Some(started) = started.take() {
                let _ = started.send(());
            }
            outcome
        })
        .await
    });
    timeout(Duration::from_secs(3), observed)
        .await
        .expect("DNS fixture runtime did not start")
        .expect("DNS fixture runtime ended before its first poll");
    // Intake buckets begin inside serve's first poll; warm up only after it.
    sleep(Duration::from_millis(40)).await;
    handle
}
async fn exchange_once(
    server: &Server,
    bytes: &[u8],
) -> std::result::Result<Vec<u8>, tokio::time::error::Elapsed> {
    let client = UdpSocket::bind((server.udp.ip(), 0)).await.unwrap();
    client.send_to(bytes, server.udp).await.unwrap();
    let mut response = [0; MAX_UDP_BYTES + 1];
    let (length, peer) = timeout(Duration::from_secs(3), client.recv_from(&mut response))
        .await?
        .unwrap();
    assert_eq!(peer, server.udp);
    Ok(response[..length].to_vec())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn single_datagram_exchange_never_retransmits_a_delayed_authorized_query() {
    let probe = Arc::new(Probe {
        blocked: true,
        ..Probe::default()
    });
    let server = Server::start(v4(), Arc::clone(&probe)).await;
    let release = async {
        timeout(Duration::from_secs(2), probe.entered.notified())
            .await
            .unwrap();
        // Longer than the general retry helper's 100ms resend interval.
        sleep(Duration::from_millis(150)).await;
        assert_eq!(
            probe.calls.load(Ordering::Relaxed),
            1,
            "single exchange retransmitted its query"
        );
        probe.release.notify_waiters();
    };
    let (response, ()) = tokio::join!(exchange_once(&server, QUERY), release);
    let response = response.expect("no reply to the single fixture datagram");
    assert_eq!(Message::from_vec(&response).unwrap().metadata.id, 1234);
    assert_eq!(server.close().await.answered, 1);
}

fn v4() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}
async fn exchange(server: &Server, bytes: &[u8]) -> Vec<u8> {
    let client = UdpSocket::bind((server.udp.ip(), 0)).await.unwrap();
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        client.send_to(bytes, server.udp).await.unwrap();
        let mut response = [0; MAX_UDP_BYTES + 1];
        if let Ok(Ok((length, peer))) =
            timeout(Duration::from_millis(100), client.recv_from(&mut response)).await
        {
            assert_eq!(peer, server.udp);
            return response[..length].to_vec();
        }
        assert!(
            Instant::now() < end,
            "no admitted reply within bounded retry window"
        );
    }
}
async fn frame(stream: &mut TcpStream, bytes: &[u8]) {
    stream
        .write_all(&(bytes.len() as u16).to_be_bytes())
        .await
        .unwrap();
    stream.write_all(bytes).await.unwrap();
}
async fn read_reply(stream: &mut TcpStream) -> Message {
    timeout(Duration::from_secs(3), async {
        let n = stream.read_u16().await.unwrap();
        let mut bytes = vec![0; n as usize];
        stream.read_exact(&mut bytes).await.unwrap();
        Message::from_vec(&bytes).unwrap()
    })
    .await
    .unwrap()
}

#[test]
fn malformed_wire_is_refused_before_handler_and_options_are_bounded() {
    for length in 0..QUERY.len() {
        assert!(wire::parse(&QUERY[..length]).is_err(), "prefix {length}");
    }
    for name in [b"\xc0\x0c".as_slice(), b"\xc0\xff", b"\x40", b"\x3fshort"] {
        let mut bytes = QUERY[..12].to_vec();
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(b"\x00\x01\x00\x01");
        assert!(wire::parse(&bytes).is_err());
    }
    for index in [4, 6, 8, 10] {
        let mut bytes = QUERY.to_vec();
        bytes[index..index + 2].copy_from_slice(&u16::MAX.to_be_bytes());
        assert!(wire::parse(&bytes).is_err());
    }
    let mut trailing = QUERY.to_vec();
    trailing.push(0);
    assert!(wire::parse(&trailing).is_err());
    let mut options = QUERY.to_vec();
    options[11] = 1;
    options.extend_from_slice(b"\x00\x00\x29\x04\xd0\x00\x00\x00\x00\x00\x44");
    for _ in 0..17 {
        options.extend_from_slice(b"\xfd\xe9\x00\x00");
    }
    assert!(wire::parse(&options).is_err());
    options[QUERY.len()] = 1;
    assert!(wire::parse(&options).is_err());
}

#[test]
fn incomplete_edns_option_cannot_disappear_during_library_conversion() {
    let mut bytes = QUERY.to_vec();
    bytes[11] = 1;
    bytes.extend_from_slice(b"\x00\x00\x29\x04\xd0\x00\x00\x00\x00\x00\x02\xfd\xe9");
    // The pinned codec accepts the record and silently clears its incomplete option.
    assert!(Message::from_vec(&bytes).is_ok());
    assert!(wire::parse(&bytes).is_err());
}

#[test]
fn maximum_question_and_option_allow_a_compressed_root_opt_owner() {
    let mut bytes = QUERY[..12].to_vec();
    bytes[11] = 1;
    for length in [63, 63, 63, 61] {
        bytes.push(length);
        bytes.extend(std::iter::repeat_n(b'a', usize::from(length)));
    }
    let root = bytes.len();
    bytes.extend_from_slice(b"\x00\x00\x01\x00\x01");
    bytes.extend_from_slice(&(0xc000 | u16::try_from(root).unwrap()).to_be_bytes());
    bytes.extend_from_slice(b"\x00\x29\x04\xd0\x00\x00\x00\x00\x10\x00\xfd\xe9\x0f\xfc");
    bytes.extend(std::iter::repeat_n(0, 4092));
    assert_eq!(bytes.len(), 4379);
    assert!(wire::parse(&bytes).is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admitted_udp_is_servfail_and_preserves_ordinary_query_types() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    for kind in [1u16, 28, 5, 16, 64, 65, 257] {
        let mut bytes = QUERY.to_vec();
        let n = bytes.len();
        bytes[n - 4..n - 2].copy_from_slice(&kind.to_be_bytes());
        let reply = Message::from_vec(&exchange(&server, &bytes).await).unwrap();
        assert_eq!(reply.metadata.id, 1234);
        assert_eq!(reply.metadata.message_type, MessageType::Response);
        assert_eq!(reply.metadata.response_code, ResponseCode::ServFail);
        assert!(!reply.metadata.recursion_available);
        assert!(!reply.metadata.authentic_data);
        assert_eq!(u16::from(reply.queries[0].query_type), kind);
        assert!(reply.answers.is_empty());
    }
    assert_eq!(server.close().await.answered, 7);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admitted_ipv6_udp_and_tcp_use_exact_peer() {
    let server = Server::start("::1".parse().unwrap(), wire::UnavailableProcessor).await;
    let reply = Message::from_vec(&exchange(&server, QUERY).await).unwrap();
    assert_eq!(reply.metadata.response_code, ResponseCode::ServFail);
    let mut stream = TcpStream::connect(server.tcp).await.unwrap();
    frame(&mut stream, QUERY).await;
    assert_eq!(read_reply(&mut stream).await.metadata.id, 1234);
    drop(stream);
    assert_eq!(server.close().await.answered, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_peer_and_malformed_requests_never_invoke_processor() {
    let probe = Arc::new(Probe::default());
    let server = Server::start(v4(), Arc::clone(&probe)).await;
    let client = UdpSocket::bind("127.0.0.2:0").await.unwrap();
    client.send_to(QUERY, server.udp).await.unwrap();
    let mut bytes = [0; 1232];
    assert!(
        timeout(Duration::from_millis(150), client.recv_from(&mut bytes))
            .await
            .is_err()
    );
    let socket = TcpSocket::new_v4().unwrap();
    socket.bind("127.0.0.2:0".parse().unwrap()).unwrap();
    let mut stream = socket.connect(server.tcp).await.unwrap();
    assert!(matches!(
        timeout(Duration::from_secs(1), stream.read(&mut bytes)).await,
        Ok(Ok(0)) | Ok(Err(_))
    ));
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let mut malformed = QUERY.to_vec();
    malformed.push(0);
    client.send_to(&malformed, server.udp).await.unwrap();
    assert!(
        timeout(Duration::from_millis(150), client.recv_from(&mut bytes))
            .await
            .is_err()
    );
    assert_eq!(probe.calls.load(Ordering::Relaxed), 0);
    let report = server.close().await;
    assert!(report.denied >= 2);
    assert_eq!(report.malformed, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_and_badvers_are_explicit_without_handler_or_option_echo() {
    let probe = Arc::new(Probe::default());
    let server = Server::start(v4(), Arc::clone(&probe)).await;
    let mut root = QUERY[..12].to_vec();
    root.extend_from_slice(b"\x00\x00\x01\x00\x01");
    let mut axfr = QUERY.to_vec();
    let n = axfr.len();
    axfr[n - 4..n - 2].copy_from_slice(&252u16.to_be_bytes());
    let mut notify = QUERY.to_vec();
    notify[2] = 0x21;
    let mut cd = QUERY.to_vec();
    cd[3] |= 0x10;
    let mut dnssec = QUERY.to_vec();
    dnssec[11] = 1;
    dnssec.extend_from_slice(b"\x00\x00\x29\x04\xd0\x00\x00\x80\x00\x00\x00");
    for bytes in [&root, &axfr, &notify, &cd, &dnssec] {
        assert_eq!(
            Message::from_vec(&exchange(&server, bytes).await)
                .unwrap()
                .metadata
                .response_code,
            ResponseCode::NotImp
        );
    }
    let mut edns = QUERY.to_vec();
    edns[11] = 1;
    edns.extend_from_slice(b"\x00\x00\x29\x00\x01\x00\x01\x80\x00\x00\x05\xfd\xe9\x00\x01\x2a");
    let reply = Message::from_vec(&exchange(&server, &edns).await).unwrap();
    // BADVERS and BADSIG share wire code16; Hickory decodes its BADSIG variant.
    assert_eq!(u16::from(reply.metadata.response_code), 16);
    assert!(reply.edns.unwrap().options().as_ref().is_empty());
    assert_eq!(probe.calls.load(Ordering::Relaxed), 0);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn amplification_forces_valid_tc_and_tcp_preserves_full_answer() {
    let probe = Arc::new(Probe {
        large: true,
        ..Probe::default()
    });
    let server = Server::start(v4(), probe).await;
    let bytes = exchange(&server, QUERY).await;
    assert!(bytes.len() <= QUERY.len() * 3);
    let reply = Message::from_vec(&bytes).unwrap();
    assert!(reply.metadata.truncation);
    assert!(reply.answers.is_empty());
    assert_eq!(reply.queries, wire::parse(QUERY).unwrap().queries);
    let mut stream = TcpStream::connect(server.tcp).await.unwrap();
    frame(&mut stream, QUERY).await;
    let reply = read_reply(&mut stream).await;
    assert!(!reply.metadata.truncation);
    assert_eq!(reply.answers.len(), 1);
    drop(stream);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_split_prefix_payload_and_coalesced_pipeline_keep_ids() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    let mut stream = TcpStream::connect(server.tcp).await.unwrap();
    let prefix = (QUERY.len() as u16).to_be_bytes();
    stream.write_all(&prefix[..1]).await.unwrap();
    sleep(Duration::from_millis(10)).await;
    stream.write_all(&prefix[1..]).await.unwrap();
    stream.write_all(&QUERY[..17]).await.unwrap();
    sleep(Duration::from_millis(10)).await;
    stream.write_all(&QUERY[17..]).await.unwrap();
    assert_eq!(read_reply(&mut stream).await.metadata.id, 1234);
    let mut pipeline = vec![];
    for id in [7u16, 8, 9] {
        let mut bytes = QUERY.to_vec();
        bytes[..2].copy_from_slice(&id.to_be_bytes());
        pipeline.extend_from_slice(&prefix);
        pipeline.extend_from_slice(&bytes);
    }
    stream.write_all(&pipeline).await.unwrap();
    for id in [7, 8, 9] {
        assert_eq!(read_reply(&mut stream).await.metadata.id, id);
    }
    drop(stream);
    assert_eq!(server.close().await.answered, 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixture_waits_for_runtime_start_before_warming_rate_buckets() {
    let (release, delayed) = oneshot::channel();
    let startup = Server::start_after(v4(), wire::UnavailableProcessor, async {
        delayed.await.unwrap();
    });
    tokio::pin!(startup);
    assert!(
        timeout(Duration::from_millis(100), &mut startup)
            .await
            .is_err(),
        "fixture returned before the server task was allowed to start"
    );
    release.send(()).unwrap();
    let server = startup.await;
    let mut stream = TcpStream::connect(server.tcp).await.unwrap();
    frame(&mut stream, QUERY).await;
    assert_eq!(read_reply(&mut stream).await.metadata.id, 1234);
    drop(stream);
    assert_eq!(server.close().await.answered, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revocation_during_processing_drops_reply_and_does_not_repin() {
    let probe = Arc::new(Probe {
        blocked: true,
        ..Probe::default()
    });
    let server = Server::start(v4(), Arc::clone(&probe)).await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.send_to(QUERY, server.udp).await.unwrap();
    timeout(Duration::from_secs(2), probe.entered.notified())
        .await
        .unwrap();
    let mut revoked = server.doc.clone();
    revoked["listeners"][0]["enabled"] = json!(false);
    apply(&server.owner, &revoked, 2);
    probe.release.notify_one();
    let mut bytes = [0; 1232];
    assert!(
        timeout(Duration::from_millis(200), client.recv_from(&mut bytes))
            .await
            .is_err()
    );
    assert_eq!(
        *probe.profiles.lock().unwrap(),
        vec![typed::<ProfileId>(10)]
    );
    assert_eq!(server.close().await.answered, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_listener_id_cannot_serve_replaced_bind_and_tcp_rechecks_each_query() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    let mut stream = TcpStream::connect(server.tcp).await.unwrap();
    frame(&mut stream, QUERY).await;
    read_reply(&mut stream).await;
    let replacement = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let mut changed = server.doc.clone();
    changed["listeners"][0]["bind"] = json!(replacement.local_addr().unwrap().to_string());
    changed["listeners"][1]["enabled"] = json!(false);
    apply(&server.owner, &changed, 2);
    frame(&mut stream, QUERY).await;
    let mut bytes = [0; 1232];
    assert!(matches!(
        timeout(Duration::from_secs(1), stream.read(&mut bytes)).await,
        Ok(Ok(0)) | Ok(Err(_))
    ));
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.send_to(QUERY, server.udp).await.unwrap();
    assert!(
        timeout(Duration::from_millis(150), client.recv_from(&mut bytes))
            .await
            .is_err()
    );
    drop(stream);
    assert_eq!(server.close().await.answered, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn incomplete_tcp_frame_has_absolute_deadline_and_shutdown_releases_socket() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    let mut stream = TcpStream::connect(server.tcp).await.unwrap();
    stream.write_all(&[0]).await.unwrap();
    sleep(Duration::from_millis(2500)).await;
    stream.write_all(&[31]).await.unwrap();
    let mut bytes = [0; 1];
    let began = Instant::now();
    let outcome = timeout(Duration::from_secs(3), stream.read(&mut bytes)).await;
    assert!(matches!(outcome, Ok(Ok(0)) | Ok(Err(_))));
    assert!(began.elapsed() < Duration::from_secs(3));
    let _idle = TcpStream::connect(server.tcp).await.unwrap();
    let report = server.close().await;
    assert!(report.timed_out >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_lease_cannot_submit_a_blocked_response() {
    let probe = Arc::new(Probe {
        blocked: true,
        ..Probe::default()
    });
    let mut server = Server::start(v4(), Arc::clone(&probe)).await;
    while SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_millis()
        > 400
    {
        sleep(Duration::from_millis(20)).await;
    }
    let time = now();
    for grant in server.doc["legacyGrants"].as_array_mut().unwrap() {
        grant["lease"]["issuedAt"] = json!(time - 60);
        grant["lease"]["expiresAt"] = json!(time + 3);
    }
    apply(&server.owner, &server.doc, 2);
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.send_to(QUERY, server.udp).await.unwrap();
    timeout(Duration::from_secs(1), probe.entered.notified())
        .await
        .unwrap();
    // The conservative upper clock bound reaches expiry before the processor ends.
    sleep(Duration::from_millis(1200)).await;
    probe.release.notify_one();
    let mut bytes = [0; 1232];
    assert!(
        timeout(Duration::from_millis(200), client.recv_from(&mut bytes))
            .await
            .is_err()
    );
    assert_eq!(server.close().await.answered, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_drains_bounded_queries_and_cancels_incomplete_reads() {
    let probe = Arc::new(Probe {
        blocked: true,
        ..Probe::default()
    });
    let server = Server::start(v4(), Arc::clone(&probe)).await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.send_to(QUERY, server.udp).await.unwrap();
    timeout(Duration::from_secs(2), probe.entered.notified())
        .await
        .unwrap();
    let mut incomplete = TcpStream::connect(server.tcp).await.unwrap();
    incomplete.write_all(&[0]).await.unwrap();
    let began = Instant::now();
    let report = server.close().await;
    assert!(began.elapsed() < Duration::from_secs(3));
    // Graceful drain can submit SERVFAIL only if current authority remains valid;
    // stopping maintenance may instead close the conservative clock boundary.
    assert!(report.answered <= 1);
    assert!(report.timed_out >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zero_oversize_and_trailing_frames_close_without_handler() {
    let probe = Arc::new(Probe::default());
    let server = Server::start(v4(), Arc::clone(&probe)).await;
    for bytes in [vec![], vec![0; 65535], {
        let mut q = QUERY.to_vec();
        q.push(0);
        q
    }] {
        let mut stream = TcpStream::connect(server.tcp).await.unwrap();
        frame(&mut stream, &bytes).await;
        let mut response = [0; 1];
        assert!(matches!(
            timeout(Duration::from_secs(2), stream.read(&mut response)).await,
            Ok(Ok(0)) | Ok(Err(_))
        ));
        sleep(Duration::from_millis(20)).await;
    }
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.send_to(&vec![0; 1500], server.udp).await.unwrap();
    let mut response = [0; 1232];
    assert!(
        timeout(Duration::from_millis(150), client.recv_from(&mut response))
            .await
            .is_err()
    );
    assert_eq!(probe.calls.load(Ordering::Relaxed), 0);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_ip_profiles_remain_bound_to_their_actual_listeners() {
    let udp_a = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let udp_b = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = udp_a.local_addr().unwrap();
    let b = udp_b.local_addr().unwrap();
    let mut doc = document(a, tcp.local_addr().unwrap());
    let profile = json!({"id":text(11),"name":"Second","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"deny"}});
    doc["profiles"].as_array_mut().unwrap().push(profile);
    doc["listeners"].as_array_mut().unwrap().push(json!({"id":text(22),"name":"Second UDP","enabled":true,"bind":b.to_string(),"transport":"dns_udp"}));
    doc["endpoints"].as_array_mut().unwrap().push(json!({"id":text(31),"name":"Second endpoint","enabled":true,"profile":text(11),"access":[{"transport":"udp","listener":text(22),"grants":[text(42)]}]}));
    let mut grant = doc["legacyGrants"][0].clone();
    grant["id"] = json!(text(42));
    grant["listener"] = json!(text(22));
    grant["endpoint"] = json!(text(31));
    doc["legacyGrants"].as_array_mut().unwrap().push(grant);
    let owner = Arc::new(Deployment::in_memory(typed(1), keys().1));
    apply(&owner, &doc, 1);
    let probe = Arc::new(Probe::default());
    let (stop, shutdown) = watch::channel(false);
    let handle = tokio::spawn(runtime::serve(
        owner,
        vec![
            ListenerSocket::Udp {
                id: typed(20),
                socket: udp_a,
            },
            ListenerSocket::Udp {
                id: typed(22),
                socket: udp_b,
            },
        ],
        shutdown,
        Arc::clone(&probe),
    ));
    sleep(Duration::from_millis(40)).await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let mut bytes = [0; 1232];
    for address in [a, b, a, b] {
        client.send_to(QUERY, address).await.unwrap();
        timeout(Duration::from_secs(1), client.recv_from(&mut bytes))
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(
        *probe.profiles.lock().unwrap(),
        [10, 11, 10, 11].map(typed::<ProfileId>)
    );
    stop.send_replace(true);
    handle.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_refuses_unconfigured_duplicate_and_excess_sockets() {
    for count in [0, 1, 2, 17] {
        let owner = Arc::new(Deployment::in_memory(typed(1), keys().1));
        let udp = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = udp.local_addr().unwrap();
        let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        apply(&owner, &document(address, tcp.local_addr().unwrap()), 1);
        let mut sockets = vec![];
        if count == 2 {
            sockets.push(ListenerSocket::Udp {
                id: typed(20),
                socket: udp,
            });
        }
        for _ in sockets.len()..count {
            sockets.push(ListenerSocket::Udp {
                id: typed(20),
                socket: UdpSocket::bind("127.0.0.1:0").await.unwrap(),
            });
        }
        let (_stop, shutdown) = watch::channel(false);
        assert_eq!(
            runtime::serve(owner, sockets, shutdown, wire::UnavailableProcessor)
                .await
                .unwrap_err()
                .code(),
            ErrorCode::Configuration
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_tcp_limit_refuses_65th_connection_and_shutdown_releases_all() {
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    // Populate the accepted empty-start rate buckets; every held peer is admitted
    // before its first (absent) frame, while all slots remain node-wide.
    sleep(Duration::from_millis(1100)).await;
    let mut held = vec![];
    for _ in 0..64 {
        held.push(TcpStream::connect(server.tcp).await.unwrap());
    }
    sleep(Duration::from_millis(100)).await;
    let mut extra = TcpStream::connect(server.tcp).await.unwrap();
    let mut bytes = [0; 1];
    assert!(matches!(
        timeout(Duration::from_secs(1), extra.read(&mut bytes)).await,
        Ok(Ok(0)) | Ok(Err(_))
    ));
    assert!(server.close().await.denied >= 1);
    drop(held);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_group_quota_refuses_tcp_while_udp_requests_hold_64_slots() {
    let probe = Arc::new(Probe {
        blocked: true,
        ..Probe::default()
    });
    let server = Server::start(v4(), Arc::clone(&probe)).await;
    sleep(Duration::from_millis(1100)).await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let deadline = Instant::now() + Duration::from_millis(800);
    while probe.calls.load(Ordering::Relaxed) < 64 {
        client.send_to(QUERY, server.udp).await.unwrap();
        tokio::task::yield_now().await;
        assert!(
            Instant::now() < deadline,
            "could not fill the actual shared group quota"
        );
    }
    let mut stream = TcpStream::connect(server.tcp).await.unwrap();
    let mut bytes = [0; 1];
    assert!(matches!(
        timeout(Duration::from_millis(200), stream.read(&mut bytes)).await,
        Ok(Ok(0)) | Ok(Err(_))
    ));
    assert_eq!(probe.calls.load(Ordering::Relaxed), 64);
    probe.release.notify_waiters();
    let report = server.close().await;
    assert!(report.denied >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_processor_gets_servfail_within_the_original_query_budget() {
    let probe = Arc::new(Probe {
        blocked: true,
        ..Probe::default()
    });
    let server = Server::start(v4(), probe).await;
    let began = Instant::now();
    let bytes = exchange(&server, QUERY).await;
    let reply = Message::from_vec(&bytes).unwrap();
    assert_eq!(reply.metadata.response_code, ResponseCode::ServFail);
    assert!(!reply.metadata.authentic_data);
    assert!(!reply.metadata.recursion_available);
    assert!(began.elapsed() < Duration::from_millis(2200));
    let mut stream = TcpStream::connect(server.tcp).await.unwrap();
    let began = Instant::now();
    frame(&mut stream, QUERY).await;
    assert_eq!(
        read_reply(&mut stream).await.metadata.response_code,
        ResponseCode::ServFail
    );
    assert!(began.elapsed() < Duration::from_millis(2200));
    drop(stream);
    let report = server.close().await;
    assert!(report.timed_out >= 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn consumption_waits_for_the_actual_next_checkpoint_without_repinning() {
    use droutex_core::deployment::AdmissionCode;
    use std::{future::poll_fn, task::Poll};
    let owner = Arc::new(Deployment::in_memory(typed(1), keys().1));
    apply(
        &owner,
        &document(
            "127.0.0.1:5353".parse().unwrap(),
            "127.0.0.1:5354".parse().unwrap(),
        ),
        1,
    );
    sleep(Duration::from_millis(40)).await;
    let refresh = crate::clock::Refresh::new();
    let setup_end = Instant::now() + Duration::from_secs(6);
    let mut attempts = Vec::with_capacity(2);
    while attempts.len() < 2 {
        match owner.prepare_legacy_dns(typed(20), v4()) {
            Ok(attempt) => attempts.push(attempt),
            Err(error) if error.code() == AdmissionCode::CheckpointRequired => {
                assert!(Instant::now() < setup_end);
                let upkeep = Arc::clone(&owner);
                assert!(
                    refresh
                        .perform(
                            move || upkeep.refresh_authorization_clock(&Cancellation::default())
                        )
                        .await
                        .unwrap()
                );
            }
            Err(error) => panic!("unexpected setup refusal: {error:?}"),
        }
    }
    let attempt = attempts.pop().unwrap();
    let retained = attempts.pop().unwrap();
    let end = Instant::now() + Duration::from_secs(6);
    loop {
        match owner.prepare_legacy_dns(typed(20), v4()) {
            Err(error) if error.code() == AdmissionCode::CheckpointRequired => break,
            Ok(temporary) => drop(temporary),
            Err(error) => panic!("unexpected refusal before the next checkpoint: {error:?}"),
        }
        assert!(
            Instant::now() < end,
            "no next checkpoint window within the bounded probe"
        );
        sleep(Duration::from_millis(20)).await;
    }
    let mut consuming = Box::pin(runtime::consume(
        attempt,
        &refresh,
        Instant::now() + Duration::from_secs(2),
    ));
    let mut retired = Box::pin(runtime::consume(
        retained,
        &refresh,
        Instant::now() + Duration::from_secs(2),
    ));
    poll_fn(|context| {
        assert!(consuming.as_mut().poll(context).is_pending());
        assert!(retired.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    let upkeep = Arc::clone(&owner);
    assert!(
        refresh
            .perform(move || upkeep.refresh_authorization_clock(&Cancellation::default()))
            .await
            .unwrap()
    );
    let auth = consuming.await.unwrap();
    assert_eq!(auth.decision().metadata().sequence.get(), 1);
    assert!(auth.verify_current().is_ok());
    apply(
        &owner,
        &document(
            "127.0.0.1:5353".parse().unwrap(),
            "127.0.0.1:5354".parse().unwrap(),
        ),
        2,
    );
    assert_eq!(retired.await.unwrap_err().code(), ErrorCode::Unavailable);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_submission_rechecks_actual_authority_before_resuming_io() {
    use std::{
        future::{Future, poll_fn},
        task::Poll,
    };
    let server = Server::start(v4(), wire::UnavailableProcessor).await;
    let auth = server
        .owner
        .prepare_legacy_dns(typed(20), v4())
        .unwrap()
        .consume()
        .unwrap();
    let wake = Notify::new();
    let submitted = std::sync::atomic::AtomicBool::new(false);
    let operation = async {
        wake.notified().await;
        submitted.store(true, Ordering::Relaxed);
        Ok::<_, std::io::Error>(1)
    };
    let refresh = crate::clock::Refresh::new();
    let mut guarded = Box::pin(runtime::submit(
        &auth,
        &refresh,
        Instant::now() + Duration::from_secs(2),
        operation,
    ));
    poll_fn(|context| {
        assert!(guarded.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    let mut revoked = server.doc.clone();
    revoked["listeners"][0]["enabled"] = json!(false);
    apply(&server.owner, &revoked, 2);
    wake.notify_one();
    assert_eq!(guarded.await.unwrap_err().code(), ErrorCode::Unavailable);
    assert!(!submitted.load(Ordering::Relaxed));
    drop(auth);
    server.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_node_replies_from_the_explicit_nondefault_destination() {
    let udp = UdpSocket::bind("127.0.0.2:0").await.unwrap();
    let address = udp.local_addr().unwrap();
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut doc = document(address, tcp.local_addr().unwrap());
    for grant in doc["legacyGrants"].as_array_mut().unwrap() {
        grant["source"] = json!("127.0.0.1");
    }
    let node = DnsNode::new(Deployment::in_memory(typed(1), keys().1));
    apply(node.deployment(), &doc, 1);
    let (stop, shutdown) = watch::channel(false);
    let handle = tokio::spawn(node.serve(
        vec![ListenerSocket::Udp {
            id: typed(20),
            socket: udp,
        }],
        shutdown,
    ));
    sleep(Duration::from_millis(40)).await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.connect(address).await.unwrap();
    client.send(QUERY).await.unwrap();
    let mut bytes = [0; 1232];
    let received = timeout(Duration::from_secs(1), client.recv(&mut bytes)).await;
    stop.send_replace(true);
    let report = handle.await.unwrap().unwrap();
    let length = received.unwrap().unwrap();
    let reply = Message::from_vec(&bytes[..length]).unwrap();
    assert_eq!(reply.metadata.response_code, ResponseCode::ServFail);
    assert_eq!(report.answered, 1);
    let _rebound = UdpSocket::bind(address).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wildcard_udp_is_refused_before_startup_and_releases_each_socket() {
    for (bind, peer) in [("0.0.0.0", "127.0.0.1"), ("::", "::1")] {
        let bind: IpAddr = bind.parse().unwrap();
        let peer: IpAddr = peer.parse().unwrap();
        let udp = UdpSocket::bind((bind, 0)).await.unwrap();
        let address = udp.local_addr().unwrap();
        let tcp = TcpListener::bind((peer, 0)).await.unwrap();
        let mut doc = document(address, tcp.local_addr().unwrap());
        for grant in doc["legacyGrants"].as_array_mut().unwrap() {
            grant["source"] = json!(peer.to_string());
        }
        let node = DnsNode::new(Deployment::in_memory(typed(1), keys().1));
        apply(node.deployment(), &doc, 1);
        let (_stop, shutdown) = watch::channel(false);
        assert_eq!(
            timeout(
                Duration::from_secs(1),
                node.serve(
                    vec![ListenerSocket::Udp {
                        id: typed(20),
                        socket: udp
                    }],
                    shutdown
                )
            )
            .await
            .unwrap()
            .unwrap_err()
            .code(),
            ErrorCode::Configuration
        );
        let _released = UdpSocket::bind(address).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wildcard_republication_cannot_reuse_an_existing_explicit_udp_socket() {
    let probe = Arc::new(Probe::default());
    let mut server = Server::start(v4(), Arc::clone(&probe)).await;
    server.doc["listeners"][0]["bind"] =
        json!(SocketAddr::new("0.0.0.0".parse().unwrap(), server.udp.port()).to_string());
    apply(&server.owner, &server.doc, 2);
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.send_to(QUERY, server.udp).await.unwrap();
    let mut bytes = [0; 1232];
    assert!(
        timeout(Duration::from_millis(150), client.recv_from(&mut bytes))
            .await
            .is_err()
    );
    assert_eq!(probe.calls.load(Ordering::Relaxed), 0);
    assert_eq!(server.close().await.answered, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_wildcard_keeps_the_kernel_established_nondefault_destination() {
    let udp = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let tcp = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let address = tcp.local_addr().unwrap();
    let doc = document(udp.local_addr().unwrap(), address);
    let node = DnsNode::new(Deployment::in_memory(typed(1), keys().1));
    apply(node.deployment(), &doc, 1);
    let (stop, shutdown) = watch::channel(false);
    let handle = tokio::spawn(node.serve(
        vec![ListenerSocket::Tcp {
            id: typed(21),
            socket: tcp,
        }],
        shutdown,
    ));
    sleep(Duration::from_millis(40)).await;
    let mut stream = TcpStream::connect(SocketAddr::new(
        "127.0.0.2".parse().unwrap(),
        address.port(),
    ))
    .await
    .unwrap();
    frame(&mut stream, QUERY).await;
    assert_eq!(
        read_reply(&mut stream).await.metadata.response_code,
        ResponseCode::ServFail
    );
    drop(stream);
    stop.send_replace(true);
    assert_eq!(handle.await.unwrap().unwrap().answered, 1);
    let _released = TcpListener::bind(address).await.unwrap();
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux;
