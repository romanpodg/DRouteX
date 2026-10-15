//! One connection through the gateway (spec §6.2, §10; REQ-GW-001, 003, 004).

mod support;

use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use support::{client_hello, connect, gateway, origin, round_trip, upstream, upstream_with};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;
use veyra_gateway::{Gateway, Refusal, Service};

const LOOPBACK: Ipv4Addr = Ipv4Addr::LOCALHOST;
const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: media.example\r\n\r\n";

/// Send `head` from the admitted source, close, and return the verdict.
async fn verdict(gateway: &Arc<Gateway>, service: Service, head: &[u8]) -> Result<(), Refusal> {
    let (mut client, verdict) = connect(gateway.clone(), "127.0.0.1", service).await;
    let _ = client.write_all(head).await;
    let _ = client.shutdown().await;
    verdict.await.expect("gateway task")
}

#[tokio::test]
async fn tls_passes_through_byte_for_byte() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, verdict) = connect(gateway, "127.0.0.1", Service::Tls).await;
    let mut bytes = client_hello(Some("www.media.example"), &[]);
    bytes.extend(b"encrypted application data");
    assert_eq!(round_trip(&mut client, &bytes).await, bytes);
    drop(client);
    verdict.await.unwrap().unwrap();
    assert_eq!(origin.connections(), 1);
}

#[tokio::test]
async fn http_is_routed_by_host_and_passed_unchanged() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, _) = connect(gateway, "127.0.0.1", Service::Http).await;
    assert_eq!(round_trip(&mut client, REQUEST).await, REQUEST);
}

#[tokio::test]
async fn a_client_hello_in_pieces_still_routes() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, _) = connect(gateway, "127.0.0.1", Service::Tls).await;
    client.set_nodelay(true).unwrap();
    let hello = client_hello(Some("media.example"), &[]);
    for piece in [&hello[..3], &hello[3..40], &hello[40..]] {
        client.write_all(piece).await.unwrap();
        sleep(Duration::from_millis(20)).await;
    }
    let mut back = vec![0; hello.len()];
    client.read_exact(&mut back).await.unwrap();
    assert_eq!(back, hello);
}

#[tokio::test]
async fn strangers_are_turned_away_before_anything_is_read() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, verdict) = connect(gateway, "127.0.0.3", Service::Tls).await;
    assert!(matches!(
        verdict.await.unwrap(),
        Err(Refusal::SourceNotAdmitted)
    ));
    let mut byte = [0u8; 1];
    assert!(matches!(client.read(&mut byte).await, Ok(0) | Err(_)));
    assert_eq!(origin.connections(), 0);
}

#[tokio::test]
async fn names_not_routed_here_are_refused() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    for name in [
        "login.media.example", // home sends it direct, strict blocks it
        "us.example",          // routed to another node
        "tunnel.example",      // not allowed in mode B
        "ads.example",         // blocked
        "example.org",         // no rule
    ] {
        let result = verdict(&gateway, Service::Tls, &client_hello(Some(name), &[])).await;
        assert!(
            matches!(result, Err(Refusal::NotRouted)),
            "{name}: {result:?}"
        );
    }
    assert_eq!(origin.connections(), 0);
}

#[tokio::test]
async fn names_routed_here_for_another_profile_pass() {
    // 127.0.0.1 is admitted as home, but only strict routes video.example to
    // de-01. The gateway cannot tell profiles apart, so it serves both (spec §10.2).
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, _) = connect(gateway, "127.0.0.1", Service::Tls).await;
    let hello = client_hello(Some("video.example"), &[]);
    assert_eq!(round_trip(&mut client, &hello).await, hello);
}

#[tokio::test]
async fn connections_without_a_usable_name_are_refused() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let hello = client_hello(Some("media.example"), &[]);
    for (why, service, head) in [
        ("no SNI", Service::Tls, client_hello(None, &[])),
        (
            "an IP literal",
            Service::Tls,
            client_hello(Some("10.0.0.1"), &[]),
        ),
        ("not TLS", Service::Tls, b"\x00\x01garbage".to_vec()),
        ("closed mid-hello", Service::Tls, hello[..20].to_vec()),
        ("no Host", Service::Http, b"GET / HTTP/1.0\r\n\r\n".to_vec()),
    ] {
        let result = verdict(&gateway, service, &head).await;
        assert!(matches!(result, Err(Refusal::NoName)), "{why}: {result:?}");
    }
    assert_eq!(origin.connections(), 0);
}

#[tokio::test]
async fn private_and_own_addresses_are_refused_after_resolving() {
    let origin = origin().await;
    for ip in [
        Ipv4Addr::new(10, 0, 0, 1),
        Ipv4Addr::new(169, 254, 169, 254),
        Ipv4Addr::new(203, 0, 113, 10), // de-01's own gateway address
    ] {
        let gateway = Arc::new(gateway(upstream(&[ip]).await, origin.port));
        let result = verdict(&gateway, Service::Http, REQUEST).await;
        assert!(
            matches!(result, Err(Refusal::EgressBlocked)),
            "{ip}: {result:?}"
        );
    }
    assert_eq!(origin.connections(), 0);
}

#[tokio::test]
async fn every_connection_checks_what_the_name_resolves_to_now() {
    // DNS rebinding: first the (allowed) test origin, then a private address.
    let origin = origin().await;
    let rebinding = upstream_with(|n| match n {
        0 => vec![LOOPBACK],
        _ => vec![Ipv4Addr::new(10, 0, 0, 1)],
    })
    .await;
    let gateway = Arc::new(gateway(rebinding, origin.port));
    let (mut first, _) = connect(gateway.clone(), "127.0.0.1", Service::Http).await;
    assert_eq!(round_trip(&mut first, REQUEST).await, REQUEST);
    let result = verdict(&gateway, Service::Http, REQUEST).await;
    assert!(matches!(result, Err(Refusal::EgressBlocked)), "{result:?}");
    assert_eq!(origin.connections(), 1);
}

#[tokio::test]
async fn forbidden_addresses_are_skipped_for_allowed_ones() {
    let origin = origin().await;
    let mixed = upstream(&[Ipv4Addr::new(10, 0, 0, 1), LOOPBACK]).await;
    let gateway = Arc::new(gateway(mixed, origin.port));
    let (mut client, _) = connect(gateway, "127.0.0.1", Service::Http).await;
    assert_eq!(round_trip(&mut client, REQUEST).await, REQUEST);
}

#[tokio::test]
async fn a_silent_client_is_dropped_after_the_hello_timeout() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let started = Instant::now();
    let (_client, verdict) = connect(gateway, "127.0.0.1", Service::Tls).await;
    let result = verdict.await.unwrap();
    assert!(
        matches!(&result, Err(Refusal::Client(e)) if e.kind() == io::ErrorKind::TimedOut),
        "{result:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn unresolvable_and_unreachable_origins_are_refused() {
    let origin = origin().await;
    let nothing = Arc::new(gateway(upstream(&[]).await, origin.port));
    let result = verdict(&nothing, Service::Http, REQUEST).await;
    assert!(matches!(result, Err(Refusal::Unresolved)), "{result:?}");

    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_port = closed.local_addr().unwrap().port();
    drop(closed);
    let dead = Arc::new(gateway(upstream(&[LOOPBACK]).await, closed_port));
    let result = verdict(&dead, Service::Http, REQUEST).await;
    assert!(matches!(result, Err(Refusal::Unreachable)), "{result:?}");
}
