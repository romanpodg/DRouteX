//! Controlled composed TCP regressions. Origin DNS and dial use local fixtures;
//! these do not establish public routing or a cryptographic TLS handshake.
use super::*;
use crate::{
    origin::resolver::tests::{
        apply, fixture_with_dial, id, keys, observations, serve_dns, smart_document,
    },
    tls_metadata::tests::{sni, synthetic},
};
use droutex_core::deployment::{AdmissionCode, Cancellation};
use serde_json::{Value, json};
use std::{
    io,
    net::SocketAddr,
    num::NonZeroU16,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpSocket, UdpSocket},
    task::JoinHandle,
    time::{sleep, timeout},
};

const WAIT: Duration = Duration::from_secs(5);
// These composed fixtures share the real bounded inventory subprocess pool.
// Avoid competing with one another for native probe startup time.
static FIXTURES: Semaphore = Semaphore::const_new(1);

#[derive(Clone, Copy)]
enum Policy {
    Direct,
    ExpiringLease,
    Protected,
}

struct Fixture {
    _permit: tokio::sync::SemaphorePermit<'static>,
    owner: Arc<Deployment>,
    doc: Value,
    address: SocketAddr,
    dns: Option<UdpSocket>,
    stop: watch::Sender<bool>,
    server: JoinHandle<Result<()>>,
    clock: JoinHandle<()>,
}
impl Fixture {
    async fn start(target: SocketAddr, policy: Policy) -> Self {
        let permit = FIXTURES.acquire().await.unwrap();
        let ingress = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = ingress.local_addr().unwrap();
        let dns = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let owner = Arc::new(Deployment::in_memory(id(1), keys().1));
        let mut doc = smart_document(address);
        // Use the existing gateway fixture's maintained clock window for all
        // positive controls; denial must not be caused by a due checkpoint.
        timeout(Duration::from_secs(8), async {
            loop {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                if (now.as_secs() + 2).is_multiple_of(5) {
                    break;
                }
                sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        if matches!(policy, Policy::ExpiringLease) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            doc["gatewayGrants"][0]["lease"] =
                json!({"cause":"manual","issuedAt":now-60,"expiresAt":now+6});
        }
        let mut facts = observations();
        if matches!(policy, Policy::Protected) {
            doc["outbounds"][0]["transport"] = json!({"kind":"http_connect_tls", "host":"proxy.test", "port":443, "tlsServerName":"proxy.test"});
            facts[0].kind = droutex_common::ProxyOutboundKind::HttpConnectTls;
        }
        apply(&owner, &doc, 1);
        owner
            .refresh_authorization_clock(&Cancellation::default())
            .unwrap();
        let (stop, mut clock_stop) = watch::channel(false);
        let clock_owner = Arc::clone(&owner);
        let clock = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = clock_stop.changed() => break,
                    () = sleep(Duration::from_millis(100)) => {
                        let owner = Arc::clone(&clock_owner);
                        tokio::task::spawn_blocking(move || owner.refresh_authorization_clock(&Cancellation::default()))
                            .await.unwrap().unwrap();
                    }
                }
            }
        });
        let server = tokio::spawn(serve_preview(
            Arc::clone(&owner),
            vec![ListenerSocket {
                id: id(22),
                socket: ingress,
            }],
            stop.subscribe(),
            Arc::new(fixture_with_dial(dns.local_addr().unwrap(), target)),
            facts,
        ));
        sleep(Duration::from_millis(150)).await;
        Self {
            _permit: permit,
            owner,
            doc,
            address,
            dns: Some(dns),
            stop,
            server,
            clock,
        }
    }
    fn start_dns(&mut self) -> JoinHandle<()> {
        tokio::spawn(serve_dns(
            self.dns.take().unwrap(),
            "8.8.8.8".parse().unwrap(),
            3,
        ))
    }
    async fn finish(self) {
        self.stop.send_replace(true);
        timeout(WAIT, self.server).await.unwrap().unwrap().unwrap();
        timeout(WAIT, self.clock).await.unwrap().unwrap();
    }
}

async fn closed_without_bytes(stream: &mut TcpStream) {
    let mut received = Vec::new();
    let result = timeout(WAIT, stream.read_to_end(&mut received))
        .await
        .unwrap();
    assert!(
        result.is_ok()
            || matches!(result, Err(ref error) if matches!(error.kind(), io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted))
    );
    assert!(received.is_empty(), "denied work forwarded bytes");
}

async fn exchange_before_change(client: &mut TcpStream, origin: &mut TcpStream, hello: &[u8]) {
    let mut received = vec![0; hello.len()];
    timeout(WAIT, origin.read_exact(&mut received))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(received, hello);
    client.write_all(b"before").await.unwrap();
    let mut request = [0; 6];
    timeout(WAIT, origin.read_exact(&mut request))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&request, b"before");
    origin.write_all(b"ok").await.unwrap();
    let mut reply = [0; 2];
    timeout(WAIT, client.read_exact(&mut reply))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&reply, b"ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrong_sni_and_ech_refuse_before_origin_with_a_working_positive_control() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut fixture = Fixture::start(origin.local_addr().unwrap(), Policy::Direct).await;
    let mut encrypted = sni(b"service.test");
    encrypted.extend_from_slice(&[0xfe, 0x0d, 0, 1, 0]);
    for hello in [synthetic(&sni(b"unauthorized.test")), synthetic(&encrypted)] {
        let mut client = TcpStream::connect(fixture.address).await.unwrap();
        client.write_all(&hello).await.unwrap();
        closed_without_bytes(&mut client).await;
        let mut bytes = [0; 512];
        assert!(
            timeout(
                Duration::from_millis(100),
                fixture.dns.as_ref().unwrap().recv_from(&mut bytes)
            )
            .await
            .is_err(),
            "denied input reached origin DNS"
        );
        assert!(
            timeout(Duration::from_millis(100), origin.accept())
                .await
                .is_err(),
            "denied input reached origin TCP"
        );
    }
    // The same configuration must actually relay a valid request after denials.
    let responder = fixture.start_dns();
    let hello = synthetic(&sni(b"service.test"));
    let mut client = TcpStream::connect(fixture.address).await.unwrap();
    client.write_all(&hello).await.unwrap();
    let (mut peer, _) = timeout(WAIT, origin.accept()).await.unwrap().unwrap();
    exchange_before_change(&mut client, &mut peer, &hello).await;
    drop(client);
    drop(peer);
    timeout(WAIT, responder).await.unwrap().unwrap();
    fixture.finish().await;
}

#[derive(Clone, Copy)]
enum Change {
    Lease,
    Generation,
    Revocation,
}

async fn changing_authority_stops_both_directions(change: Change) {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut fixture = Fixture::start(
        origin.local_addr().unwrap(),
        if matches!(change, Change::Lease) {
            Policy::ExpiringLease
        } else {
            Policy::Direct
        },
    )
    .await;
    let responder = fixture.start_dns();
    let hello = synthetic(&sni(b"service.test"));
    let mut client = TcpStream::connect(fixture.address).await.unwrap();
    client.write_all(&hello).await.unwrap();
    let (mut peer, _) = timeout(WAIT, origin.accept()).await.unwrap().unwrap();
    exchange_before_change(&mut client, &mut peer, &hello).await;
    timeout(WAIT, responder).await.unwrap().unwrap();
    match change {
        Change::Lease => {
            timeout(Duration::from_secs(12), async {
                loop {
                    match fixture.owner.prepare_gateway(
                        id(22),
                        "127.0.0.1".parse().unwrap(),
                        "service.test",
                        NonZeroU16::new(443).unwrap(),
                        observations(),
                    ) {
                        Err(error) if error.code() == AdmissionCode::Lease => break,
                        // Initial bidirectional exchange proved this connection.
                        // Waiting is below IDLE; I/O here could itself race expiry.
                        Ok(attempt) => drop(attempt),
                        Err(error) => panic!("unexpected refusal before lease expiry: {error}"),
                    }
                    sleep(Duration::from_millis(100)).await;
                }
            })
            .await
            .unwrap();
        }
        Change::Generation | Change::Revocation => {
            if matches!(change, Change::Revocation) {
                fixture.doc["smartDnsRoutes"] = json!([]);
            } else {
                fixture.doc["profiles"][0]["name"] = json!("replacement generation");
            }
            apply(&fixture.owner, &fixture.doc, 2);
        }
    }
    // Pending reads wake on new input; no post-change bytes may cross either way.
    let _ = client.write_all(b"late-client").await;
    let _ = peer.write_all(b"late-origin").await;
    closed_without_bytes(&mut client).await;
    closed_without_bytes(&mut peer).await;
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lease_expiry_during_a_live_connection_blocks_both_directions() {
    changing_authority_stops_both_directions(Change::Lease).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generation_replacement_during_transfer_blocks_both_directions() {
    changing_authority_stops_both_directions(Change::Generation).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_revocation_during_transfer_blocks_both_directions() {
    changing_authority_stops_both_directions(Change::Revocation).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unavailable_origin_closes_the_client_without_a_successful_relay() {
    // Keep a numeric port reserved but non-listening: no race with port reuse.
    let reserved = TcpSocket::new_v4().unwrap();
    reserved.bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let mut fixture = Fixture::start(reserved.local_addr().unwrap(), Policy::Direct).await;
    let responder = fixture.start_dns();
    let mut client = TcpStream::connect(fixture.address).await.unwrap();
    client
        .write_all(&synthetic(&sni(b"service.test")))
        .await
        .unwrap();
    closed_without_bytes(&mut client).await;
    timeout(WAIT, responder).await.unwrap().unwrap();
    fixture.finish().await;
    drop(reserved);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_protected_policy_never_reaches_a_working_direct_origin() {
    use droutex_core::deployment::AdmissionScope;
    use droutex_policy::TcpIntent;
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut fixture = Fixture::start(origin.local_addr().unwrap(), Policy::Protected).await;
    let mut facts = observations();
    facts[0].kind = droutex_common::ProxyOutboundKind::HttpConnectTls;
    let auth = fixture
        .owner
        .prepare_gateway(
            id(22),
            "127.0.0.1".parse().unwrap(),
            "service.test",
            NonZeroU16::new(443).unwrap(),
            facts,
        )
        .unwrap()
        .consume()
        .unwrap();
    let AdmissionScope::Gateway(candidate) = auth.scope() else {
        panic!("expected gateway authority");
    };
    assert!(
        matches!(candidate.action().intent(), TcpIntent::ProtectedProxy(outbound) if outbound == id(90))
    );
    drop(auth);
    sleep(Duration::from_millis(100)).await;
    let responder = fixture.start_dns();
    let mut client = TcpStream::connect(fixture.address).await.unwrap();
    client
        .write_all(&synthetic(&sni(b"service.test")))
        .await
        .unwrap();
    closed_without_bytes(&mut client).await;
    assert!(
        timeout(Duration::from_millis(100), origin.accept())
            .await
            .is_err(),
        "protected policy silently connected DIRECT"
    );
    // Early refusal is also valid; it need not resolve an unsupported outbound.
    responder.abort();
    match responder.await {
        Ok(()) => (),
        Err(error) => assert!(error.is_cancelled()),
    }
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn origin_loss_after_exchange_closes_client_without_reconnecting() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut fixture = Fixture::start(origin.local_addr().unwrap(), Policy::Direct).await;
    let responder = fixture.start_dns();
    let hello = synthetic(&sni(b"service.test"));
    let mut client = TcpStream::connect(fixture.address).await.unwrap();
    client.write_all(&hello).await.unwrap();
    let (mut peer, _) = timeout(WAIT, origin.accept()).await.unwrap().unwrap();
    exchange_before_change(&mut client, &mut peer, &hello).await;
    timeout(WAIT, responder).await.unwrap().unwrap();
    drop(peer);
    closed_without_bytes(&mut client).await;
    assert!(
        timeout(Duration::from_millis(100), origin.accept())
            .await
            .is_err(),
        "lost origin caused an unplanned reconnect"
    );
    fixture.finish().await;
}
