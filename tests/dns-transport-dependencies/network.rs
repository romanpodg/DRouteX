use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::Semaphore,
    time::timeout,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_udp_ipv4_ipv6_preserves_wire_peer_and_local_binding() -> TestResult {
    timeout(DEADLINE, async {
        for host in ["127.0.0.1", "::1"] {
            let server = UdpSocket::bind((host, 0)).await?;
            let client = UdpSocket::bind((host, 0)).await?;
            let address = server.local_addr()?;
            let peer = client.local_addr()?;
            let serve = async {
                let mut buffer = [0; 1233];
                let (length, source) =
                    timeout(Duration::from_secs(2), server.recv_from(&mut buffer))
                        .await
                        .map_err(|_| format!("UDP {host} server receive timed out"))??;
                assert_eq!(source, peer);
                assert_eq!(&buffer[..length], QUERY);
                server.send_to(FAILURE, source).await?;
                TestResult::Ok(())
            };
            let request = async {
                client.send_to(QUERY, address).await?;
                let mut buffer = [0; 1233];
                let (length, source) =
                    timeout(Duration::from_secs(2), client.recv_from(&mut buffer))
                        .await
                        .map_err(|_| format!("UDP {host} client receive timed out"))??;
                assert_eq!(source, address);
                assert_eq!(&buffer[..length], FAILURE);
                TestResult::Ok(())
            };
            tokio::try_join!(serve, request)?;
            drop(server);
            assert_eq!(UdpSocket::bind(address).await?.local_addr()?, address);
        }
        TestResult::Ok(())
    })
    .await??;
    Ok(())
}

async fn frame(stream: &mut TcpStream) -> TestResult<Vec<u8>> {
    let length = stream.read_u16().await?;
    let mut wire = vec![0; usize::from(length)];
    stream.read_exact(&mut wire).await?;
    Ok(wire)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_split_prefix_payload_and_coalesced_pipelined_frames_remain_separate() -> TestResult {
    timeout(DEADLINE, async {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let serve = async {
            let (mut stream, peer) = listener.accept().await?;
            assert!(peer.ip().is_loopback());
            assert_eq!(stream.local_addr()?, address);
            for _ in 0..3 {
                assert_eq!(frame(&mut stream).await?, QUERY);
                stream.write_u16(FAILURE.len() as u16).await?;
                stream.write_all(FAILURE).await?;
            }
            stream.shutdown().await?;
            TestResult::Ok(())
        };
        let client = async {
            let mut stream = TcpStream::connect(address).await?;
            let prefix = (QUERY.len() as u16).to_be_bytes();
            stream.write_all(&prefix[..1]).await?;
            tokio::task::yield_now().await;
            stream.write_all(&prefix[1..]).await?;
            for part in QUERY.chunks(3) {
                stream.write_all(part).await?;
                tokio::task::yield_now().await;
            }
            let mut pipeline = Vec::new();
            for _ in 0..2 {
                pipeline.extend_from_slice(&prefix);
                pipeline.extend_from_slice(QUERY);
            }
            stream.write_all(&pipeline).await?;
            for _ in 0..3 {
                assert_eq!(frame(&mut stream).await?, FAILURE);
            }
            assert_eq!(stream.read(&mut [0; 1]).await?, 0);
            TestResult::Ok(())
        };
        tokio::try_join!(serve, client)?;
        TestResult::Ok(())
    })
    .await??;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_partial_frame_closes_its_socket_and_releases_owned_slot() -> TestResult {
    timeout(DEADLINE, async {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let slots = Semaphore::new(1);
        let serve = async {
            let (mut stream, _) = listener.accept().await?;
            let permit = slots.try_acquire()?;
            // Read one known byte before cancelling the incomplete two-byte prefix.
            assert_eq!(stream.read_u8().await?, 0);
            assert!(
                timeout(Duration::from_millis(100), stream.read_u8())
                    .await
                    .is_err()
            );
            drop(stream); // Do not resume a partially consumed frame as another query.
            drop(permit);
            assert_eq!(slots.available_permits(), 1);
            TestResult::Ok(())
        };
        let client = async {
            let mut stream = TcpStream::connect(address).await?;
            stream.write_all(&[0]).await?;
            assert_eq!(stream.read(&mut [0; 1]).await?, 0);
            TestResult::Ok(())
        };
        tokio::try_join!(serve, client)?;
        drop(listener);
        assert_eq!(TcpListener::bind(address).await?.local_addr()?, address);
        TestResult::Ok(())
    })
    .await??;
    Ok(())
}
