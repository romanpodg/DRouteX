use super::*;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static SERIAL: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "droutex-dns-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(path.join("cp")).unwrap();
        fs::create_dir(path.join("backups")).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_listener_survives_cp_loss_but_checkpoint_failure_closes_admission() {
    let fixture = Fixture::new();
    let udp = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let udp_addr = udp.local_addr().unwrap();
    let tcp_addr = tcp.local_addr().unwrap();
    let doc = document(udp_addr, tcp_addr);
    let owner = Arc::new(
        Deployment::provision(
            &fixture.0.join("dp"),
            &fixture.0.join("cp"),
            &fixture.0.join("backups"),
            typed(1),
            keys().1,
        )
        .unwrap(),
    );
    apply(&owner, &doc, 1);
    let (stop, shutdown) = watch::channel(false);
    let handle = tokio::spawn(runtime::serve(
        Arc::clone(&owner),
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
        wire::UnavailableProcessor,
    ));
    let server = Server {
        owner,
        doc,
        udp: udp_addr,
        tcp: tcp_addr,
        stop,
        handle,
    };
    fs::remove_dir(fixture.0.join("cp")).unwrap();
    fs::remove_dir(fixture.0.join("backups")).unwrap();
    assert_eq!(
        Message::from_vec(&exchange(&server, QUERY).await)
            .unwrap()
            .metadata
            .response_code,
        ResponseCode::ServFail
    );
    // A directory at the fixed transaction path makes the real store refuse its
    // next automatic clock checkpoint. No test hook or privileged operation.
    fs::create_dir(fixture.0.join("dp/manifest.next")).unwrap();
    sleep(Duration::from_millis(1200)).await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.send_to(QUERY, server.udp).await.unwrap();
    let mut bytes = [0; 1232];
    assert!(
        timeout(Duration::from_millis(200), client.recv_from(&mut bytes))
            .await
            .is_err()
    );
    let report = server.close().await;
    assert!(report.clock_failures >= 1);
    assert_eq!(report.answered, 1);
}
