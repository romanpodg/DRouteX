//! The gateway's listeners (spec §10.1).

mod support;

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;
use support::{client_hello, gateway, origin, round_trip, upstream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use veyra_gateway::{Listeners, Service};

const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: media.example\r\n\r\n";

#[tokio::test]
async fn serves_tls_and_http_listeners() {
    let origin = origin().await;
    let gateway = gateway(upstream(&[Ipv4Addr::LOCALHOST]).await, origin.port);
    let config = gateway.policy().gateway_config().expect("gateway section");
    let listeners = Listeners::bind(config).await.expect("bind");
    let addrs = listeners.local_addrs().expect("bound addresses");
    tokio::spawn(listeners.serve(Arc::new(gateway)));
    let services: Vec<Service> = addrs.iter().map(|(_, s)| *s).collect();
    assert_eq!(services, [Service::Tls, Service::Http]);

    let hello = client_hello(Some("media.example"), &[]);
    let mut tls = TcpStream::connect(addrs[0].0).await.unwrap();
    assert_eq!(round_trip(&mut tls, &hello).await, hello);
    let mut http = TcpStream::connect(addrs[1].0).await.unwrap();
    assert_eq!(round_trip(&mut http, REQUEST).await, REQUEST);
}

#[tokio::test]
async fn connections_beyond_the_cap_are_closed_at_once() {
    let origin = origin().await;
    let gateway = gateway(upstream(&[Ipv4Addr::LOCALHOST]).await, origin.port)
        .with_timeouts(Duration::from_secs(5), Duration::from_millis(300));
    let config = gateway.policy().gateway_config().expect("gateway section");
    let listeners = Listeners::bind(config)
        .await
        .expect("bind")
        .with_max_sessions(1);
    let tls = listeners.local_addrs().expect("bound addresses")[0].0;
    tokio::spawn(listeners.serve(Arc::new(gateway)));

    // Holds the only session while the gateway waits for its ClientHello.
    let _silent = TcpStream::connect(tls).await.unwrap();
    sleep(Duration::from_millis(100)).await;
    let mut second = TcpStream::connect(tls).await.unwrap();
    let _ = second
        .write_all(&client_hello(Some("media.example"), &[]))
        .await;
    let mut byte = [0u8; 1];
    let read = timeout(Duration::from_secs(1), second.read(&mut byte))
        .await
        .expect("closed at once, not after the hello timeout");
    assert!(matches!(read, Ok(0) | Err(_)), "{read:?}");
    assert_eq!(origin.connections(), 0);
}
