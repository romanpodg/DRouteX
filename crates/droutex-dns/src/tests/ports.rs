//! Reserve a complete numeric loopback fixture before starting any DNS work.
use std::{
    future::Future,
    io,
    net::{IpAddr, SocketAddr},
};
use tokio::{
    net::{TcpListener, UdpSocket},
    time::{Duration, Instant, timeout, timeout_at},
};

const CANDIDATES: usize = 64;
const SETUP: Duration = Duration::from_secs(5);

pub(crate) async fn paired_loopback(ip: IpAddr) -> io::Result<(TcpListener, UdpSocket)> {
    let deadline = Instant::now() + SETUP;
    timeout_at(deadline, paired_with(ip, TcpListener::bind, deadline))
        .await
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "dual-protocol fixture setup expired",
            )
        })?
}

async fn paired_with<F, B>(
    ip: IpAddr,
    mut tcp_bind: B,
    deadline: Instant,
) -> io::Result<(TcpListener, UdpSocket)>
where
    F: Future<Output = io::Result<TcpListener>>,
    B: FnMut(SocketAddr) -> F,
{
    if !ip.is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture requires loopback",
        ));
    }
    let mut last = io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        "no shared UDP/TCP fixture port",
    );
    for _ in 0..CANDIDATES {
        setup_current(deadline)?;
        // TCP and UDP have distinct availability/exclusion sets on Windows.
        // Pick an OS-allocated UDP candidate and keep it until TCP also binds.
        let udp = UdpSocket::bind(SocketAddr::new(ip, 0)).await?;
        let address = udp.local_addr()?;
        setup_current(deadline)?;
        match tcp_bind(address).await {
            Ok(tcp) => {
                setup_current(deadline)?;
                return Ok((tcp, udp));
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::AddrInUse | io::ErrorKind::PermissionDenied
                ) =>
            {
                last = error
            }
            Err(error) => return Err(error),
        }
        // This candidate is not a running fixture. Release it before selecting
        // another; there is no query, admission, upstream attempt or deadline reset.
        drop(udp);
    }
    Err(last)
}

fn setup_current(deadline: Instant) -> io::Result<()> {
    if Instant::now() >= deadline {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "dual-protocol fixture setup expired",
        ))
    } else {
        Ok(())
    }
}

#[tokio::test]
async fn unavailable_candidate_is_released_before_a_complete_pair_is_published() {
    use std::sync::{Arc, Mutex};
    let seen = Arc::new(Mutex::new(Vec::<SocketAddr>::new()));
    let record = Arc::clone(&seen);
    let (tcp, udp) = paired_with(
        "127.0.0.1".parse().unwrap(),
        move |address| {
            let record = Arc::clone(&record);
            async move {
                let previous = record.lock().unwrap().last().copied();
                if let Some(previous) = previous {
                    // The kernel may immediately reuse the released port for this
                    // candidate. Otherwise it must be independently bindable now.
                    if previous != address {
                        drop(UdpSocket::bind(previous).await.unwrap());
                    }
                }
                let count = {
                    let mut seen = record.lock().unwrap();
                    seen.push(address);
                    seen.len()
                };
                match count {
                    1 => Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "simulated protocol-specific reservation",
                    )),
                    2 => Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "simulated concurrent port claim",
                    )),
                    _ => TcpListener::bind(address).await,
                }
            }
        },
        Instant::now() + SETUP,
    )
    .await
    .unwrap();
    assert!((3..=64).contains(&seen.lock().unwrap().len()));
    assert_eq!(tcp.local_addr().unwrap(), udp.local_addr().unwrap());
    assert!(UdpSocket::bind(udp.local_addr().unwrap()).await.is_err());
    assert!(TcpListener::bind(tcp.local_addr().unwrap()).await.is_err());
}

#[tokio::test]
async fn unavailable_pair_exhaustion_is_finite_and_preserves_the_last_error() {
    use std::sync::{Arc, Mutex};
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    let result = paired_with(
        "127.0.0.1".parse().unwrap(),
        move |address| {
            record.lock().unwrap().push(address);
            std::future::ready(Err::<TcpListener, _>(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "fixture reservation",
            )))
        },
        Instant::now() + SETUP,
    )
    .await;
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    let last = {
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 64);
        *seen.last().unwrap()
    };
    drop(UdpSocket::bind(last).await.unwrap());
}

#[tokio::test]
async fn cancelled_pair_setup_releases_its_unpublished_udp_socket() {
    let (published, received) = tokio::sync::oneshot::channel();
    let mut published = Some(published);
    let task = tokio::spawn(async move {
        paired_with(
            "127.0.0.1".parse().unwrap(),
            move |address| {
                published.take().unwrap().send(address).unwrap();
                std::future::pending::<io::Result<TcpListener>>()
            },
            Instant::now() + SETUP,
        )
        .await
    });
    let address = timeout(Duration::from_secs(1), received)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(UdpSocket::bind(address).await.unwrap());
}
