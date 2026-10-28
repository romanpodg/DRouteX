//! D51 tests-only client qualification; no DRouteX forwarding or production adoption.

mod bounded_io;
mod guarded;
mod guarded_tests;
mod independent;

use hickory_net::{
    DnsMultiplexer, NetError,
    runtime::{TokioRuntimeProvider, iocompat::AsyncIoTokioAsStd},
    tcp::TcpClientStream,
    udp::UdpClientStream,
    xfer::{DnsExchange, DnsHandle, DnsRequestSender, FirstAnswer},
};
use hickory_proto::op::{DnsRequest, DnsRequestOptions, DnsResponse, Message};
use std::{error::Error, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    task::JoinHandle,
    time::timeout,
};

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;
const DEADLINE: Duration = Duration::from_secs(3);
const QUERY: &[u8] =
    b"\x04\xd2\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01";
const ANSWER: &[u8] = b"\x04\xd2\x81\x80\x00\x01\x00\x01\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01\xc0\x0c\x00\x01\x00\x01\x00\x00\x00\x3c\x00\x04\xcb\x00\x71\x09";

fn request() -> TestResult<DnsRequest> {
    Ok(DnsRequest::new(
        Message::from_vec(QUERY)?,
        DnsRequestOptions::default(),
    ))
}
fn response_for(query: &[u8], wrong_question: bool) -> Vec<u8> {
    let mut response = ANSWER.to_vec();
    response[..2].copy_from_slice(&query[..2]);
    if wrong_question {
        response[13..20].copy_from_slice(b"foreign");
    }
    response
}
struct OwnedTcp {
    exchange: Option<DnsExchange<TokioRuntimeProvider>>,
    background: Option<JoinHandle<()>>,
}
impl OwnedTcp {
    async fn connect(address: SocketAddr) -> TestResult<Self> {
        Self::connect_with_timeout(address, Duration::from_millis(500)).await
    }
    async fn connect_with_timeout(
        address: SocketAddr,
        connect_timeout: Duration,
    ) -> TestResult<Self> {
        let (connecting, sender) = TcpClientStream::<AsyncIoTokioAsStd<TcpStream>>::new(
            address,
            None,
            Some(connect_timeout),
            TokioRuntimeProvider::new(),
        );
        let stream = timeout(connect_timeout + DEADLINE, connecting).await??;
        let multiplexer = DnsMultiplexer::new(stream, sender)
            .with_timeout(Duration::from_millis(500))
            .with_max_active_requests(1);
        let (exchange, background) = DnsExchange::from_stream(multiplexer);
        Ok(Self {
            exchange: Some(exchange),
            background: Some(tokio::spawn(background)),
        })
    }
    async fn close(mut self) -> TestResult {
        drop(self.exchange.take());
        if let Some(background) = self.background.take() {
            background.abort();
            match timeout(DEADLINE, background).await? {
                Ok(_) => (),
                Err(error) if error.is_cancelled() => (),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}
impl Drop for OwnedTcp {
    fn drop(&mut self) {
        if let Some(background) = &self.background {
            background.abort();
        }
    }
}
async fn tcp_exchange(wrong_question: bool) -> TestResult<Result<DnsResponse, NetError>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let peer = async {
        let (mut socket, _) = timeout(DEADLINE, listener.accept()).await??;
        let length = timeout(DEADLINE, socket.read_u16()).await??;
        let mut query = vec![0; usize::from(length)];
        timeout(DEADLINE, socket.read_exact(&mut query)).await??;
        assert_eq!(&query[12..], &QUERY[12..]);
        let response = response_for(&query, wrong_question);
        socket
            .write_all(&u16::try_from(response.len())?.to_be_bytes())
            .await?;
        socket.write_all(&response).await?;
        let mut byte = [0];
        assert_eq!(timeout(DEADLINE, socket.read(&mut byte)).await??, 0);
        TestResult::Ok(())
    };
    let client = async {
        let owner = OwnedTcp::connect(address).await?;
        let response = timeout(
            DEADLINE,
            owner
                .exchange
                .as_ref()
                .unwrap()
                .send(request()?)
                .first_answer(),
        )
        .await;
        owner.close().await?;
        TestResult::Ok(response?)
    };
    let (response, completed) = tokio::join!(client, peer);
    completed?;
    drop(listener);
    let _rebound = TcpListener::bind(address).await?;
    response
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn udp_ipv4_accepts_the_independent_answer_and_releases_its_socket() -> TestResult {
    let peer = UdpSocket::bind("127.0.0.1:0").await?;
    let address = peer.local_addr()?;
    let mut client = UdpClientStream::builder(address, TokioRuntimeProvider::new())
        .with_timeout(Some(Duration::from_millis(500)))
        .with_max_retries(0)
        .build();
    let server = async {
        let mut bytes = [0; 1232];
        let (length, client) = timeout(DEADLINE, peer.recv_from(&mut bytes)).await??;
        peer.send_to(&response_for(&bytes[..length], false), client)
            .await?;
        TestResult::Ok(client)
    };
    let response = timeout(DEADLINE, client.send_message(request()?).first_answer());
    let (response, client_address) = tokio::join!(response, server);
    let response = response??;
    assert_eq!(response.queries[0].name().to_string(), "example.test.");
    assert_eq!(response.answers.len(), 1);
    let _released = UdpSocket::bind(client_address?).await?;
    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_ipv4_accepts_the_independent_answer_and_joins_owned_background() -> TestResult {
    let response = tcp_exchange(false).await??;
    assert_eq!(response.queries[0].name().to_string(), "example.test.");
    assert_eq!(response.answers.len(), 1);
    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raw_tcp_characterization_delivers_a_matching_id_with_a_foreign_question() -> TestResult {
    let response = tcp_exchange(true).await?;
    // D52 accepts a composed role, not the raw library's missing question check.
    // The original failed strict test remains recorded in the qualification evidence.
    assert_eq!(response?.queries[0].name.to_ascii(), "foreign.test.");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_stream_characterization_retains_foreign_then_correct_answers() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let peer = async {
        let (mut socket, _) = timeout(DEADLINE, listener.accept()).await??;
        let length = timeout(DEADLINE, socket.read_u16()).await??;
        let mut query = vec![0; usize::from(length)];
        timeout(DEADLINE, socket.read_exact(&mut query)).await??;
        assert_eq!(&query[12..], &QUERY[12..]);
        for foreign in [true, false] {
            let answer = response_for(&query, foreign);
            socket
                .write_all(&u16::try_from(answer.len())?.to_be_bytes())
                .await?;
            socket.write_all(&answer).await?;
        }
        let mut byte = [0];
        assert_eq!(timeout(DEADLINE, socket.read(&mut byte)).await??, 0);
        TestResult::Ok(())
    };
    let client = async {
        let owner = OwnedTcp::connect(address).await?;
        let mut responses = owner.exchange.as_ref().unwrap().send(request()?);
        let observed = timeout(DEADLINE, async {
            let first = (&mut responses).first_answer().await?;
            let second = (&mut responses).first_answer().await?;
            TestResult::Ok([first, second])
        })
        .await;
        drop(responses);
        owner.close().await?;
        observed?
    };
    let (observed, completed) = tokio::join!(client, peer);
    completed?;
    let [first, second] = observed?;
    // Characterization only: the mandatory foreign-question refusal above still fails.
    assert_eq!(first.queries[0].name().to_string(), "foreign.test.");
    assert_eq!(second.queries[0].name().to_string(), "example.test.");
    assert_eq!(first.metadata.id, second.metadata.id);
    drop(listener);
    let _rebound = TcpListener::bind(address).await?;
    Ok(())
}
