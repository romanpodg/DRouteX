//! Manual external probe, never part of the ordinary offline test run.
//! Uses only disposable loopback configuration and the RFC8032 public test key.
use droutex_common::{DnsUpstreamId, ListenerId};
use droutex_config::{
    CompletePublication, InputFormat, OutboundObservation, ReferenceCatalog, ResourceData,
    parse_configuration,
    snapshot::{SnapshotMetadata, SnapshotResource, SnapshotSequence},
};
use droutex_core::deployment::{AdmissionCode, Artifacts, Cancellation, Deployment};
use droutex_dns::{DnsNode, ListenerSocket};
use droutex_gateway::{listener, origin::resolver::OriginResolver};
use droutex_security::{SigningKey, SigningKeyId, SnapshotTrust};
use hickory_proto::{
    op::{Message, MessageType, OpCode, Query, ResponseCode},
    rr::{Name, RData, RecordType},
};
use serde_json::{Value, json};
use std::{
    error::Error,
    process::Command,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::{oneshot, watch},
    task::JoinSet,
    time::{sleep, timeout},
};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
const DOMAIN: &str = "example.com";
const LIMIT: Duration = Duration::from_secs(15);

fn id<T: FromStr>(n: u32) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}")
        .parse()
        .expect("fixed probe identity")
}
fn text(n: u32) -> String {
    id::<ListenerId>(n).to_string()
}
fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("fixed hex"), 16)
                .expect("fixed hex")
        })
        .collect()
}
fn keys() -> Result<(SigningKey, SnapshotTrust)> {
    // RFC8032 fixture seed. Never use this key for an operational deployment.
    let public = hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
        .try_into()
        .map_err(|_| "public test key length")?;
    let key = SigningKey::from_pkcs8(
        SigningKeyId::from_bytes([1; 16])?,
        hex(
            "302e020100300506032b6570042204209d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        ),
        &public,
    )?;
    let trust = SnapshotTrust::new(vec![(key.id(), public)])?;
    Ok((key, trust))
}
fn publish(owner: &Deployment, doc: &Value, key: &SigningKey, sequence: u64) -> Result<()> {
    let resource = br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"udp_tcp","address":"1.1.1.1:53"}]}"#.to_vec();
    let checked = parse_configuration(
        &serde_json::to_vec(doc)?,
        InputFormat::Json,
        &ReferenceCatalog {
            upstreams: [id::<DnsUpstreamId>(60)].into(),
            ..Default::default()
        },
    )?;
    let publication = CompletePublication::new(
        checked,
        vec![],
        &[ResourceData {
            resource: SnapshotResource::Upstream(id(60)),
            bytes: &resource,
        }],
        &[],
    )?;
    let envelope = publication.encode(
        SnapshotMetadata {
            node: id(1),
            snapshot: id(2),
            source: id(3),
            sequence: SnapshotSequence::new(sequence)?,
        },
        key,
    )?;
    owner.activate(
        envelope,
        Artifacts::new(vec![(SnapshotResource::Upstream(id(60)), resource)])?,
        &Cancellation::default(),
    )?;
    Ok(())
}
fn document(udp: std::net::SocketAddr, tcp: std::net::SocketAddr) -> Result<Value> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let lease = json!({"cause":"manual","issuedAt":now-10,"expiresAt":now+300});
    Ok(json!({"schemaVersion":2,
        "profiles":[{"id":text(10),"name":"Probe DNS","defaultDnsAction":{"kind":"gateway","outbound":text(90)},"defaultTrafficAction":{"kind":"deny"}},
                    {"id":text(11),"name":"Probe TCP","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"outbound","outbound":text(90)}}],
        "listeners":[{"id":text(20),"name":"Probe UDP","enabled":true,"bind":udp.to_string(),"transport":"dns_udp"},
                     {"id":text(21),"name":"Probe TCP DNS","enabled":true,"bind":tcp.to_string(),"transport":"dns_tcp"},
                     {"id":text(22),"name":"Probe gateway","enabled":true,"bind":"127.0.0.1:443","transport":"gateway_tcp"}],
        "endpoints":[{"id":text(30),"name":"Probe endpoint","enabled":true,"profile":text(10),"access":[
            {"transport":"udp","listener":text(20),"grants":[text(40)]},{"transport":"tcp","listener":text(21),"grants":[text(41)]}]}],
        "networkGroups":[{"id":text(50),"name":"Probe group","trafficProfile":text(11)}],
        "legacyGrants":[{"id":text(40),"source":"127.0.0.1","listener":text(20),"group":text(50),"endpoint":text(30),"lease":lease},
                        {"id":text(41),"source":"127.0.0.1","listener":text(21),"group":text(50),"endpoint":text(30),"lease":lease}],
        "gatewayGrants":[{"id":text(42),"source":"127.0.0.1","listener":text(22),"group":text(50),"targets":[{"mode":"exact","value":DOMAIN}],"ports":[{"from":443,"to":443}],"outbounds":[text(90)],"lease":lease}],
        "outbounds":[{"id":text(90),"name":"Probe direct","enabled":true,"transport":{"kind":"vps_direct"}}],
        "originUpstream":text(60),
        "smartDnsRoutes":[{"endpoint":text(30),"grant":text(42),"domain":DOMAIN,"port":443,"outbound":text(90),"addresses":["127.0.0.1"],"ttl":1}]
    }))
}
async fn query(address: std::net::SocketAddr) -> Result<Message> {
    let socket = UdpSocket::bind("127.0.0.1:0").await?;
    socket.connect(address).await?;
    let mut message = Message::new(1234, MessageType::Query, OpCode::Query);
    message.queries.push(Query::query(
        Name::from_str(&format!("{DOMAIN}."))?,
        RecordType::A,
    ));
    socket.send(&message.to_vec()?).await?;
    let mut bytes = [0; 1232];
    let size = timeout(LIMIT, socket.recv(&mut bytes)).await??;
    let reply = Message::from_vec(&bytes[..size])?;
    if reply.metadata.id != 1234 || reply.queries != message.queries {
        return Err("mismatched probe DNS response".into());
    }
    Ok(reply)
}
async fn https(address: std::net::Ipv4Addr) -> Result<bool> {
    // curl validates the public CA chain and example.com hostname by default.
    // No proxy, alternate trust store, insecure mode or origin-dial substitution.
    let result = tokio::task::spawn_blocking(move || {
        Command::new(if cfg!(windows) { "curl.exe" } else { "curl" })
            .env_remove("CURL_CA_BUNDLE")
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR")
            .env_remove("SSLKEYLOGFILE")
            .args([
                "--disable",
                "--noproxy",
                "*",
                "--http1.1",
                "--resolve",
                &format!("{DOMAIN}:443:{address}"),
                "--max-time",
                "10",
                "--fail",
                "--silent",
                "--show-error",
                "--output",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
                "--write-out",
                "%{http_code}",
                "https://example.com/",
            ])
            .output()
    })
    .await??;
    http_result(result.status.code(), &result.stdout)
}

fn http_result(code: Option<i32>, output: &[u8]) -> Result<bool> {
    match (code, output) {
        (Some(0), b"200") => Ok(true),
        (Some(35 | 52 | 56), b"000") => Ok(false),
        _ => Err("unexpected HTTP result from public probe".into()),
    }
}

#[test]
fn unrelated_curl_failures_cannot_pass_the_revocation_probe() {
    assert!(http_result(Some(0), b"200").unwrap());
    for code in [35, 52, 56] {
        assert!(!http_result(Some(code), b"000").unwrap());
    }
    for code in [0, 6, 7, 22, 28, 60] {
        assert!(http_result(Some(code), b"000").is_err(), "curl code {code}");
    }
    assert!(http_result(None, b"000").is_err());
    assert!(http_result(Some(22), b"500").is_err());
}

async fn cleanup(tasks: &mut JoinSet<Result<()>>) -> Result<()> {
    let graceful = timeout(LIMIT, async {
        let mut failed = false;
        while let Some(result) = tasks.join_next().await {
            failed |= !matches!(result, Ok(Ok(())));
        }
        failed
    })
    .await;
    match graceful {
        Ok(false) => Ok(()),
        Ok(true) => Err("probe service cleanup failed".into()),
        Err(_) => {
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
            Err("probe service cleanup timed out; tasks aborted and joined".into())
        }
    }
}

fn observations() -> Vec<OutboundObservation> {
    vec![OutboundObservation {
        id: id(90),
        kind: droutex_common::ProxyOutboundKind::GatewayDirect,
        availability: droutex_common::OutboundAvailability::Available,
        tcp: droutex_policy::TcpCapability::Supported,
    }]
}

#[tokio::main]
async fn main() -> Result<()> {
    let udp = UdpSocket::bind("127.0.0.1:0").await?;
    let tcp = TcpListener::bind("127.0.0.1:0").await?;
    // Binding failure is an error; never stop an existing service or change host permissions.
    let ingress = TcpListener::bind("127.0.0.1:443").await?;
    let dns_address = udp.local_addr()?;
    let mut doc = document(dns_address, tcp.local_addr()?)?;
    let (key, trust) = keys()?;
    let node = DnsNode::new(Deployment::in_memory(id(1), trust));
    let owner = Arc::clone(node.deployment());
    publish(&owner, &doc, &key, 1)?;
    let resolver = Arc::new(OriginResolver::new(vec![])?);
    let (gateway_stop, shutdown) = watch::channel(false);
    let (ready, receiver) = oneshot::channel();
    let mut tasks = JoinSet::<Result<()>>::new();
    let gateway_work = listener::serve_preview_ready(
        Arc::clone(&owner),
        vec![listener::ListenerSocket {
            id: id(22),
            socket: ingress,
        }],
        shutdown,
        resolver,
        observations(),
        ready,
    );
    tasks.spawn(async move {
        gateway_work.await?;
        Ok(())
    });
    let (dns_stop, shutdown) = watch::channel(false);
    let check = async {
        let readiness = timeout(LIMIT, receiver).await??;
        tasks.spawn(async move {
            node.serve_with_gateway(
                vec![
                    ListenerSocket::Udp {
                        id: id(20),
                        socket: udp,
                    },
                    ListenerSocket::Tcp {
                        id: id(21),
                        socket: tcp,
                    },
                ],
                vec![],
                shutdown,
                readiness,
            )
            .await?;
            Ok(())
        });
        sleep(Duration::from_millis(250)).await;
        let reply = query(dns_address).await?;
        if reply.metadata.response_code != ResponseCode::NoError {
            return Err("SmartDNS positive answer refused".into());
        }
        let [record] = reply.answers.as_slice() else {
            return Err("expected exactly one gateway address".into());
        };
        let RData::A(address) = &record.data else {
            return Err("expected gateway IPv4 answer".into());
        };
        if address.0 != std::net::Ipv4Addr::LOCALHOST {
            return Err("unexpected gateway address".into());
        }
        if !https(address.0).await? {
            return Err("public-origin TLS/HTTP verification failed".into());
        }
        println!("PASS: SmartDNS answer -> gateway -> checked public origin -> verified HTTPS 200");
        doc["smartDnsRoutes"] = json!([]);
        publish(&owner, &doc, &key, 2)?;
        sleep(Duration::from_millis(150)).await;
        if query(dns_address).await?.metadata.response_code != ResponseCode::ServFail {
            return Err("revoked generation still advertised".into());
        }
        let ingress =
            owner.reserve_smartdns_gateway_ingress(id(22), std::net::Ipv4Addr::LOCALHOST.into())?;
        match ingress.prepare_for_name(DOMAIN, observations()) {
            Err(error) if error.code() == AdmissionCode::Scope => (),
            _ => return Err("expected current-generation route-scope refusal".into()),
        }
        if https(address.0).await? {
            return Err("cached gateway address still served revoked route".into());
        }
        println!("PASS: route removal/new generation refuses DNS and cached-address HTTPS");
        Ok::<(), Box<dyn Error + Send + Sync>>(())
    }
    .await;
    gateway_stop.send_replace(true);
    dns_stop.send_replace(true);
    cleanup(&mut tasks).await?;
    check
}
