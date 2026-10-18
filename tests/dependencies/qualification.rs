use hickory_proto::{op::Message, rr::RecordType};
use rustls::{
    ClientConfig, RootCertStore, ServerConfig,
    pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName},
};
use std::{error::Error, path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tokio_rustls::{TlsAcceptor, TlsConnector};

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn identity() -> Result<(Arc<ServerConfig>, ClientConfig), Box<dyn Error + Send + Sync>> {
    let directory = PathBuf::from(std::env::var("DROUTEX_QUALIFICATION_CERT_DIR")?);
    let cert = CertificateDer::from(std::fs::read(directory.join("proxy.der"))?);
    let key = PrivatePkcs8KeyDer::from(std::fs::read(directory.join("proxy-key.der"))?);
    let server = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key.into())?;
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(std::fs::read(
        directory.join("ca.der"),
    )?))?;
    let client = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok((Arc::new(server), client))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tls_connect_preserves_bytes_and_denies_missing_credentials() -> TestResult {
    let (server_config, client_config) = identity()?;
    let client_config = Arc::new(client_config);
    for authorized in [true, false] {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let acceptor = TlsAcceptor::from(Arc::clone(&server_config));
        let server = async move {
            let (socket, _) = listener.accept().await?;
            let mut stream = acceptor.accept(socket).await?;
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                if request.len() >= 1024 {
                    return Err("fixture header limit".into());
                }
                request.push(stream.read_u8().await?);
            }
            let expected = b"CONNECT 198.18.1.2:9443 HTTP/1.1\r\nHost: 198.18.1.2:9443\r\nProxy-Authorization: Basic Zml4dHVyZTpub3QtYS1zZWNyZXQ=\r\n\r\n";
            if authorized {
                assert_eq!(request, expected);
                stream
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n\x00\xffdata")
                    .await?;
            } else {
                assert!(!request.windows(19).any(|s| s == b"Proxy-Authorization"));
                stream
                    .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                    .await?;
            }
            stream.shutdown().await?;
            TestResult::Ok(())
        };
        let operation = async {
            let socket = TcpStream::connect(address).await?;
            let mut client = TlsConnector::from(Arc::clone(&client_config))
                .connect(ServerName::try_from("proxy.test")?, socket)
                .await?;
            assert!(client.get_ref().1.protocol_version().is_some());
            let auth = if authorized {
                "Proxy-Authorization: Basic Zml4dHVyZTpub3QtYS1zZWNyZXQ=\r\n"
            } else {
                ""
            };
            client
                .write_all(
                    format!(
                        "CONNECT 198.18.1.2:9443 HTTP/1.1\r\nHost: 198.18.1.2:9443\r\n{auth}\r\n"
                    )
                    .as_bytes(),
                )
                .await?;
            let mut reply = Vec::new();
            client.read_to_end(&mut reply).await?;
            if authorized {
                assert_eq!(
                    reply,
                    b"HTTP/1.1 200 Connection Established\r\n\r\n\x00\xffdata"
                );
            } else {
                assert_eq!(reply, b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n");
            }
            TestResult::Ok(())
        };
        // Both peers live inside the deadline; cancellation drops both sockets.
        let result = timeout(Duration::from_secs(5), async {
            tokio::try_join!(server, operation)
        })
        .await;
        result??;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tls_rejects_wrong_hostname_and_untrusted_ca() -> TestResult {
    let (server_config, client_config) = identity()?;
    let untrusted = ClientConfig::builder()
        .with_root_certificates(RootCertStore::empty())
        .with_no_client_auth();
    for (hostname, config) in [("wrong.test", client_config), ("proxy.test", untrusted)] {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let acceptor = TlsAcceptor::from(Arc::clone(&server_config));
        let server = async move {
            let (socket, _) = listener.accept().await?;
            let _rejected_handshake = acceptor.accept(socket).await;
            TestResult::Ok(())
        };
        let client = async {
            let socket = TcpStream::connect(address).await?;
            let result = TlsConnector::from(Arc::new(config))
                .connect(ServerName::try_from(hostname)?, socket)
                .await;
            let error = result.expect_err("invalid certificate accepted");
            assert!(
                matches!(
                    error
                        .get_ref()
                        .and_then(|e| e.downcast_ref::<rustls::Error>()),
                    Some(rustls::Error::InvalidCertificate(_))
                ),
                "unexpected failure for {hostname}: {error}"
            );
            TestResult::Ok(())
        };
        let result = timeout(Duration::from_secs(5), async {
            tokio::try_join!(server, client)
        })
        .await;
        result??;
    }
    Ok(())
}

#[test]
fn decodes_independent_dns_wire_vector_and_rejects_truncation() {
    // A single IN A question for example.test, independent of Hickory's encoder.
    let wire =
        b"\x04\xd2\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01";
    let message = Message::from_vec(wire).expect("valid independent DNS vector");
    assert_eq!(message.metadata.id, 1234);
    assert_eq!(message.queries[0].name.to_ascii(), "example.test.");
    assert_eq!(message.queries[0].query_type, RecordType::A);
    assert!(Message::from_vec(&wire[..wire.len() - 1]).is_err());
}
