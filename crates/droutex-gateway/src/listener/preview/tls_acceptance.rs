//! Local OpenSSL endpoints exercise actual TLS; the origin dial remains a
//! test-only loopback substitution and does not prove production public routing.
use super::*;
use crate::origin::resolver::tests::{
    admitted, apply, fixture_with_dial, id, keys, observations, serve_dns, smart_document,
};
use std::{path::PathBuf, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, UdpSocket},
    process::Command,
};

struct TempDir(PathBuf);
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tls_certificate_and_hostname_are_verified_through_preview_relay() {
    let directory =
        TempDir(std::env::temp_dir().join(format!("droutex-local-tls-{}", std::process::id())));
    std::fs::create_dir(&directory.0).unwrap();
    let certificate = directory.0.join("cert.pem");
    let key = directory.0.join("key.pem");
    let output = timeout(
        Duration::from_secs(10),
        Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=service.test",
                "-addext",
                "subjectAltName=DNS:service.test",
                "-keyout",
            ])
            .arg(&key)
            .arg("-out")
            .arg(&certificate)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        output.status.success(),
        "failed to generate disposable certificate"
    );
    let ingress = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = ingress.local_addr().unwrap();
    let reserved = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = reserved.local_addr().unwrap();
    drop(reserved);
    let dns = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let dns_address = dns.local_addr().unwrap();
    let dns_server = tokio::spawn(serve_dns(dns, "8.8.8.8".parse().unwrap(), 6));
    let owner = Arc::new(Deployment::in_memory(id(1), keys().1));
    apply(&owner, &smart_document(address), 1);
    drop(admitted(&owner).await);
    tokio::time::sleep(Duration::from_millis(120)).await;
    let (cancel, stop) = watch::channel(false);
    let server = tokio::spawn(serve_preview(
        owner,
        vec![ListenerSocket {
            id: id(22),
            socket: ingress,
        }],
        stop,
        Arc::new(fixture_with_dial(dns_address, target)),
        observations(),
    ));
    for (hostname, accepted) in [("service.test", true), ("wrong.test", false)] {
        let mut origin = Command::new("openssl")
            .args(["s_server", "-accept"])
            .arg(target.to_string())
            .arg("-cert")
            .arg(&certificate)
            .arg("-key")
            .arg(&key)
            .args(["-www", "-naccept", "1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(origin.stdout.take().unwrap()).lines();
        timeout(Duration::from_secs(3), async {
            loop {
                let line = lines
                    .next_line()
                    .await
                    .unwrap()
                    .expect("origin exited before binding");
                if line == "ACCEPT" {
                    break;
                }
            }
        })
        .await
        .unwrap();
        // Admission starts with no burst credit; let the per-group bucket refill.
        tokio::time::sleep(Duration::from_millis(120)).await;
        let mut client = Command::new("openssl")
            .args(["s_client", "-connect"])
            .arg(address.to_string())
            .args([
                "-servername",
                "service.test",
                "-verify_hostname",
                hostname,
                "-verify_return_error",
                "-CAfile",
            ])
            .arg(&certificate)
            .args(["-quiet", "-ign_eof"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        client
            .stdin
            .take()
            .unwrap()
            .write_all(b"GET / HTTP/1.0\r\n\r\n")
            .await
            .unwrap();
        let output = timeout(Duration::from_secs(8), client.wait_with_output())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            output.status.success(),
            accepted,
            "OpenSSL client: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if accepted {
            assert!(output.stdout.starts_with(b"HTTP/1.0 200 ok"));
        }
        timeout(Duration::from_secs(3), origin.wait())
            .await
            .unwrap()
            .unwrap();
    }
    timeout(Duration::from_secs(3), dns_server)
        .await
        .unwrap()
        .unwrap();
    cancel.send_replace(true);
    timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
