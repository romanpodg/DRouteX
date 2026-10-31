//! D52 I/O bounds with the original live DNS admission. Hickory owns framing.

use super::inventory::{Inventory, Probe};
use super::live::Live;

use hickory_net::runtime::{
    DnsUdpSocket, RuntimeProvider, TokioHandle, TokioRuntimeProvider, TokioTime,
    iocompat::AsyncIoTokioAsStd,
};
use std::{
    future::{Future, poll_fn},
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, ready},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpStream, UdpSocket},
    time::{Instant, timeout_at},
};

pub(super) const MESSAGE_BYTES: usize = 4096;
pub(super) const UDP_RECEIVES: usize = 16;
pub(super) const TCP_READ_BYTES: usize = 16 * (MESSAGE_BYTES + 2);
pub(super) const WRITE_BYTES: usize = MESSAGE_BYTES + 2;
pub(super) const IO_OPERATIONS: usize = 256;
pub(super) const ATTEMPT: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Violation {
    Deadline,
    ReceiveBudget,
    ReadBudget,
    WriteBudget,
    DatagramSize,
    Peer,
    ExtraSocket,
}
impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Violation {}
fn denied(violation: Violation) -> io::Error {
    io::Error::other(violation)
}

#[derive(Default)]
pub(super) struct Counters {
    pub datagrams: AtomicUsize,
    pub read_bytes: AtomicUsize,
    pub read_operations: AtomicUsize,
    pub write_bytes: AtomicUsize,
    pub write_operations: AtomicUsize,
    pub sockets: AtomicUsize,
    pub opened: AtomicUsize,
    pub rejected: AtomicUsize,
}

pub(super) struct Budget {
    pub peer: SocketAddr,
    pub deadline: Instant,
    pub counters: Counters,
    pub live: Arc<Live>,
    inventory: Arc<Inventory>,
    probe: Mutex<Option<Probe>>,
    first_send_checked: AtomicBool,
    destination_refused: AtomicBool,
    #[cfg(test)]
    checkpoint_after_write: AtomicBool,
    #[cfg(test)]
    checkpoint_after_inventory: AtomicBool,
    #[cfg(test)]
    inventory_paused: tokio::sync::Notify,
}
impl Budget {
    pub fn new(
        peer: SocketAddr,
        deadline: Instant,
        live: Arc<Live>,
        inventory: Arc<Inventory>,
    ) -> Arc<Self> {
        Arc::new(Self {
            peer,
            deadline,
            counters: Counters::default(),
            live,
            inventory,
            probe: Mutex::new(None),
            first_send_checked: AtomicBool::new(false),
            destination_refused: AtomicBool::new(false),
            #[cfg(test)]
            checkpoint_after_write: AtomicBool::new(false),
            #[cfg(test)]
            checkpoint_after_inventory: AtomicBool::new(false),
            #[cfg(test)]
            inventory_paused: tokio::sync::Notify::new(),
        })
    }
    pub fn check(&self) -> io::Result<()> {
        if Instant::now() >= self.deadline {
            Err(denied(Violation::Deadline))
        } else {
            Ok(())
        }
    }
    fn socket(&self, target: SocketAddr) -> io::Result<SocketSlot<'_>> {
        self.check()?;
        if target != self.peer {
            return Err(denied(Violation::Peer));
        }
        self.counters
            .opened
            .compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed)
            .map_err(|_| denied(Violation::ExtraSocket))?;
        Ok(SocketSlot {
            budget: self,
            committed: false,
        })
    }
    fn poll(&self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.live
            .poll(context, self.deadline)
            .map(|result| result.map_err(io::Error::other))
    }
    async fn submit<T>(&self, operation: impl Future<Output = io::Result<T>>) -> io::Result<T> {
        tokio::pin!(operation);
        timeout_at(
            self.deadline,
            poll_fn(|context| {
                ready!(self.poll(context))?;
                operation.as_mut().poll(context)
            }),
        )
        .await
        .map_err(|_| denied(Violation::Deadline))?
    }
    fn poll_inventory(&self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        ready!(self.poll(context))?;
        let mut probe = self.probe.lock().map_err(|_| denied(Violation::Peer))?;
        let current =
            probe.get_or_insert_with(|| self.inventory.start(self.peer.ip(), self.deadline));
        let outcome = ready!(current.poll(context));
        probe.take();
        if outcome.is_err() {
            self.destination_refused.store(true, Ordering::Relaxed);
        }
        #[cfg(test)]
        if outcome.is_ok()
            && self
                .checkpoint_after_inventory
                .swap(false, Ordering::Relaxed)
        {
            self.live.test_deny(Some(
                droutex_core::deployment::AdmissionCode::CheckpointRequired,
            ));
            self.inventory_paused.notify_one();
        }
        Poll::Ready(outcome.map_err(io::Error::other))
    }
    fn poll_first_send(&self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if !self.first_send_checked.load(Ordering::Acquire) {
            ready!(self.poll_inventory(context))?;
            ready!(self.poll(context))?;
            self.first_send_checked.store(true, Ordering::Release);
            return Poll::Ready(Ok(()));
        }
        self.poll(context)
    }
    pub async fn finish_inventory(&self) -> crate::Result<()> {
        let probe = self
            .probe
            .lock()
            .map_err(|_| crate::DnsError(crate::ErrorCode::State))?
            .take();
        if let Some(probe) = probe {
            self.destination_refused.store(true, Ordering::Relaxed);
            probe.finish().await?;
        }
        Ok(())
    }
    pub fn destination_refused(&self) -> bool {
        self.destination_refused.load(Ordering::Relaxed)
    }
}

// Failed or cancelled binds/connects own no usable socket. Return only that
// uncommitted reservation; a successfully opened attempt remains one-shot.
struct SocketSlot<'a> {
    budget: &'a Budget,
    committed: bool,
}
impl SocketSlot<'_> {
    fn commit(mut self) {
        self.committed = true;
    }
}
impl Drop for SocketSlot<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.budget.counters.opened.store(0, Ordering::Relaxed);
        }
    }
}

// The library clones RuntimeProvider into owned request/background futures.
// This Arc shares one attempt's limits and observations across those owners.
#[derive(Clone)]
pub(super) struct Provider(pub Arc<Budget>);
impl RuntimeProvider for Provider {
    type Handle = TokioHandle;
    type Timer = TokioTime;
    type Udp = BoundedUdp;
    type Tcp = AsyncIoTokioAsStd<BoundedTcp>;
    fn create_handle(&self) -> Self::Handle {
        TokioRuntimeProvider::new().create_handle()
    }
    fn connect_tcp(
        &self,
        server: SocketAddr,
        bind: Option<SocketAddr>,
        _: Option<Duration>,
    ) -> Pin<Box<dyn Send + Future<Output = io::Result<Self::Tcp>>>> {
        let budget = Arc::clone(&self.0);
        Box::pin(async move {
            let slot = budget.socket(server)?;
            let connecting = TokioRuntimeProvider::new().connect_tcp(server, bind, Some(ATTEMPT));
            tokio::pin!(connecting);
            let mut started = false;
            let stream = timeout_at(
                budget.deadline,
                poll_fn(|context| {
                    if !started {
                        ready!(budget.poll_inventory(context))?;
                    }
                    ready!(budget.poll(context))?;
                    // Preserve this same connecting future after its first poll.
                    // A checkpoint pause before that point must refresh inventory.
                    started = true;
                    connecting.as_mut().poll(context)
                }),
            )
            .await
            .map_err(|_| denied(Violation::Deadline))??;
            budget.check()?;
            slot.commit();
            budget.counters.sockets.fetch_add(1, Ordering::Relaxed);
            let stream = BoundedTcp {
                inner: stream.0,
                budget,
                read: Progress::default(),
                write: Progress::default(),
            };
            Ok(AsyncIoTokioAsStd(stream))
        })
    }
    fn bind_udp(
        &self,
        local: SocketAddr,
        server: SocketAddr,
    ) -> Pin<Box<dyn Send + Future<Output = io::Result<Self::Udp>>>> {
        let budget = Arc::clone(&self.0);
        Box::pin(async move {
            let slot = budget.socket(server)?;
            let inner = budget.submit(UdpSocket::bind(local)).await?;
            budget.check()?;
            slot.commit();
            budget.counters.sockets.fetch_add(1, Ordering::Relaxed);
            Ok(BoundedUdp { inner, budget })
        })
    }
}

pub(super) struct BoundedUdp {
    inner: UdpSocket,
    budget: Arc<Budget>,
}
impl DnsUdpSocket for BoundedUdp {
    type Time = TokioTime;
    fn poll_recv_from(
        &self,
        cx: &mut Context<'_>,
        destination: &mut [u8],
    ) -> Poll<io::Result<(usize, SocketAddr)>> {
        ready!(self.budget.poll(cx))?;
        if self.budget.counters.datagrams.load(Ordering::Relaxed) >= UDP_RECEIVES {
            return Poll::Ready(Err(denied(Violation::ReceiveBudget)));
        }
        let mut bytes = [0; MESSAGE_BYTES + 1];
        let mut buffer = ReadBuf::new(&mut bytes);
        let address = ready!(self.inner.poll_recv_from(cx, &mut buffer))?;
        self.budget
            .counters
            .datagrams
            .fetch_add(1, Ordering::Relaxed);
        self.budget.check()?;
        let length = buffer.filled().len();
        if length > MESSAGE_BYTES || length > destination.len() {
            return Poll::Ready(Err(denied(Violation::DatagramSize)));
        }
        destination[..length].copy_from_slice(buffer.filled());
        Poll::Ready(Ok((length, address)))
    }
    fn poll_send_to(
        &self,
        cx: &mut Context<'_>,
        bytes: &[u8],
        target: SocketAddr,
    ) -> Poll<io::Result<usize>> {
        ready!(self.budget.poll_first_send(cx))?;
        if target != self.budget.peer {
            return Poll::Ready(Err(denied(Violation::Peer)));
        }
        if bytes.len() > MESSAGE_BYTES
            || self
                .budget
                .counters
                .write_operations
                .load(Ordering::Relaxed)
                != 0
        {
            return Poll::Ready(Err(denied(Violation::WriteBudget)));
        }
        let count = ready!(self.inner.poll_send_to(cx, bytes, target))?;
        self.budget
            .counters
            .write_bytes
            .fetch_add(count, Ordering::Relaxed);
        self.budget
            .counters
            .write_operations
            .fetch_add(1, Ordering::Relaxed);
        self.budget.check()?;
        Poll::Ready(Ok(count))
    }
}
impl Drop for BoundedUdp {
    fn drop(&mut self) {
        self.budget.counters.sockets.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Default)]
struct Progress {
    bytes: usize,
    operations: usize,
}
impl Progress {
    fn capacity(&self, limit: usize, violation: Violation) -> io::Result<usize> {
        if self.operations >= IO_OPERATIONS || self.bytes >= limit {
            return Err(denied(violation));
        }
        Ok(limit - self.bytes)
    }
    fn record(&mut self, count: usize, bytes: &AtomicUsize, operations: &AtomicUsize) {
        self.bytes += count;
        self.operations += 1;
        bytes.fetch_add(count, Ordering::Relaxed);
        operations.fetch_add(1, Ordering::Relaxed);
    }
}
pub(super) struct BoundedTcp {
    inner: TcpStream,
    budget: Arc<Budget>,
    read: Progress,
    write: Progress,
}
impl AsyncRead for BoundedTcp {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.budget.poll(cx))?;
        let capacity = this.read.capacity(TCP_READ_BYTES, Violation::ReadBudget)?;
        let capacity = capacity.min(buffer.remaining());
        let mut limited = ReadBuf::new(buffer.initialize_unfilled_to(capacity));
        ready!(Pin::new(&mut this.inner).poll_read(cx, &mut limited))?;
        let count = limited.filled().len();
        buffer.advance(count);
        this.read.record(
            count,
            &this.budget.counters.read_bytes,
            &this.budget.counters.read_operations,
        );
        this.budget.check()?;
        Poll::Ready(Ok(()))
    }
}
impl AsyncWrite for BoundedTcp {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.budget.poll_first_send(cx))?;
        let capacity = this.write.capacity(WRITE_BYTES, Violation::WriteBudget)?;
        #[cfg(test)]
        let capacity = if this.budget.checkpoint_after_write.load(Ordering::Relaxed) {
            capacity.min(1)
        } else {
            capacity
        };
        let count =
            ready!(Pin::new(&mut this.inner).poll_write(cx, &bytes[..bytes.len().min(capacity)]))?;
        this.write.record(
            count,
            &this.budget.counters.write_bytes,
            &this.budget.counters.write_operations,
        );
        #[cfg(test)]
        if this
            .budget
            .checkpoint_after_write
            .swap(false, Ordering::Relaxed)
        {
            // Private negative fixture only adds a refusal; it can never grant
            // authority rejected by the real original core checker.
            this.budget.live.test_deny(Some(
                droutex_core::deployment::AdmissionCode::CheckpointRequired,
            ));
        }
        this.budget.check()?;
        Poll::Ready(Ok(count))
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.budget.poll(cx))?;
        Pin::new(&mut this.inner).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.budget.poll(cx))?;
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}
impl Drop for BoundedTcp {
    fn drop(&mut self) {
        self.budget.counters.sockets.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
pub(crate) async fn partial_write_checkpoint_fixture(
    check: droutex_core::deployment::DnsIoCheck,
    owner: Arc<droutex_core::deployment::Deployment>,
) {
    use crate::clock::Refresh;
    use std::collections::BTreeSet;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::watch,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let stream = TcpStream::connect(address).await.unwrap();
    let (mut peer, _) = listener.accept().await.unwrap();
    let refresh = Arc::new(Refresh::new());
    let (_stop, stop) = watch::channel(false);
    let live = Arc::new(Live::new(
        check,
        Arc::clone(&refresh),
        Instant::now() + Duration::from_millis(1500),
        Arc::new(AtomicBool::new(false)),
        stop,
    ));
    let inventory = Arc::new(Inventory::fixture(Arc::new(Mutex::new(
        Ok(BTreeSet::new()),
    ))));
    let budget = Budget::new(
        address,
        Instant::now() + ATTEMPT,
        Arc::clone(&live),
        inventory,
    );
    budget.checkpoint_after_write.store(true, Ordering::Relaxed);
    budget.counters.sockets.store(1, Ordering::Relaxed);
    let mut socket = BoundedTcp {
        inner: stream,
        budget: Arc::clone(&budget),
        read: Progress::default(),
        write: Progress::default(),
    };
    let sending = async move {
        socket.write_all(b"abcdef").await.unwrap();
        drop(socket);
    };
    let receiving =
        async {
            let mut first = [0; 1];
            peer.read_exact(&mut first).await.unwrap();
            assert_eq!(&first, b"a");
            live.test_deny(None);
            assert!(
                refresh
                    .perform(move || owner.refresh_authorization_clock(
                        &droutex_core::deployment::Cancellation::default()
                    ))
                    .await
                    .unwrap()
            );
            let mut bytes = first.to_vec();
            peer.read_to_end(&mut bytes).await.unwrap();
            assert_eq!(
                &bytes, b"abcdef",
                "checkpoint restarted an already partially written operation"
            );
        };
    tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(sending, receiving);
    })
    .await
    .unwrap();
    budget.finish_inventory().await.unwrap();
    assert_eq!(budget.counters.write_bytes.load(Ordering::Relaxed), 6);
    assert_eq!(budget.counters.sockets.load(Ordering::Relaxed), 0);
}

#[cfg(test)]
pub(crate) async fn inventory_checkpoint_fixture(
    check: droutex_core::deployment::DnsIoCheck,
    owner: Arc<droutex_core::deployment::Deployment>,
    tcp: bool,
) {
    use crate::clock::Refresh;
    use hickory_net::runtime::{DnsUdpSocket, RuntimeProvider};
    use std::collections::BTreeSet;
    use tokio::{net::TcpListener, sync::watch, time::timeout};
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = UdpSocket::bind(address).await.unwrap();
    let source = Arc::new(Mutex::new(Ok(BTreeSet::new())));
    let inventory = Arc::new(Inventory::fixture(Arc::clone(&source)));
    let refresh = Arc::new(Refresh::new());
    let (_stop, stop) = watch::channel(false);
    let live = Arc::new(Live::new(
        check,
        Arc::clone(&refresh),
        Instant::now() + Duration::from_millis(1500),
        Arc::new(AtomicBool::new(false)),
        stop,
    ));
    let budget = Budget::new(
        address,
        Instant::now() + ATTEMPT,
        Arc::clone(&live),
        inventory,
    );
    budget
        .checkpoint_after_inventory
        .store(true, Ordering::Relaxed);
    let provider = Provider(Arc::clone(&budget));
    let operation = async {
        if tcp {
            let stream = provider.connect_tcp(address, None, Some(ATTEMPT)).await;
            let refused = stream.is_err();
            drop(stream);
            refused
        } else {
            let socket = provider
                .bind_udp("0.0.0.0:0".parse().unwrap(), address)
                .await
                .unwrap();
            let sent = poll_fn(|context| socket.poll_send_to(context, b"one", address)).await;
            drop(socket);
            sent.is_err()
        }
    };
    let changed =
        async {
            budget.inventory_paused.notified().await;
            *source.lock().unwrap() = Ok([address.ip()].into());
            live.test_deny(None);
            assert!(
                refresh
                    .perform(move || owner.refresh_authorization_clock(
                        &droutex_core::deployment::Cancellation::default()
                    ))
                    .await
                    .unwrap()
            );
        };
    let (refused, ()) = timeout(Duration::from_secs(3), async {
        tokio::join!(operation, changed)
    })
    .await
    .unwrap();
    budget.finish_inventory().await.unwrap();
    let mut bytes = [0; 8];
    assert!(
        refused,
        "new socket I/O used inventory cached before checkpoint wait"
    );
    assert!(
        timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err(),
        "connected after target became self"
    );
    assert!(
        timeout(Duration::from_millis(100), peer.recv_from(&mut bytes))
            .await
            .is_err(),
        "sent after target became self"
    );
    assert_eq!(budget.counters.write_operations.load(Ordering::Relaxed), 0);
    assert_eq!(budget.counters.sockets.load(Ordering::Relaxed), 0);
}

#[cfg(test)]
pub(crate) async fn exact_io_limits_fixture(
    auth: &droutex_core::deployment::AuthorizedRequest<'_>,
    owner: Arc<droutex_core::deployment::Deployment>,
) {
    use crate::clock::Refresh;
    use std::collections::BTreeSet;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::watch,
    };
    let refresh = Arc::new(Refresh::new());
    for (reading, operations, limit) in [
        (true, true, IO_OPERATIONS),
        (false, true, IO_OPERATIONS),
        (true, false, TCP_READ_BYTES),
        (false, false, WRITE_BYTES),
    ] {
        let upkeep = Arc::clone(&owner);
        assert!(
            refresh
                .perform(move || {
                    upkeep.refresh_authorization_clock(
                        &droutex_core::deployment::Cancellation::default(),
                    )
                })
                .await
                .unwrap()
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let stream = TcpStream::connect(address).await.unwrap();
        let (mut peer, _) = listener.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();
        peer.set_nodelay(true).unwrap();
        let (_stop, stopped) = watch::channel(false);
        let live = Arc::new(Live::new(
            auth.dns_io_check().unwrap(),
            Arc::clone(&refresh),
            Instant::now() + Duration::from_millis(1500),
            Arc::new(AtomicBool::new(false)),
            stopped,
        ));
        let inventory = Arc::new(Inventory::fixture(Arc::new(Mutex::new(
            Ok(BTreeSet::new()),
        ))));
        let budget = Budget::new(address, Instant::now() + ATTEMPT, live, inventory);
        budget.counters.sockets.store(1, Ordering::Relaxed);
        let mut bounded = BoundedTcp {
            inner: stream,
            budget: Arc::clone(&budget),
            read: Progress::default(),
            write: Progress::default(),
        };
        let remote = async {
            if reading {
                peer.write_all(&vec![42; limit + 1]).await.unwrap();
            } else {
                let mut bytes = vec![0; limit];
                peer.read_exact(&mut bytes).await.unwrap();
                assert!(bytes.iter().all(|byte| *byte == 42));
            }
            let mut byte = [0];
            assert!(matches!(
                tokio::time::timeout(Duration::from_secs(1), peer.read(&mut byte)).await,
                Ok(Ok(0)) | Ok(Err(_))
            ));
        };
        let local = async {
            let mut bytes = vec![42; if operations { 1 } else { limit }];
            for _ in 0..if operations { limit } else { 1 } {
                if reading {
                    bounded.read_exact(&mut bytes).await.unwrap();
                } else {
                    bounded.write_all(&bytes).await.unwrap();
                }
            }
            let error = if reading {
                bounded.read_exact(&mut [0]).await.unwrap_err()
            } else {
                bounded.write_all(&[42]).await.unwrap_err()
            };
            assert_eq!(
                error.get_ref().unwrap().downcast_ref::<Violation>(),
                Some(&if reading {
                    Violation::ReadBudget
                } else {
                    Violation::WriteBudget
                })
            );
            drop(bounded);
        };
        tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(local, remote);
        })
        .await
        .unwrap();
        let count = if operations {
            if reading {
                &budget.counters.read_operations
            } else {
                &budget.counters.write_operations
            }
        } else if reading {
            &budget.counters.read_bytes
        } else {
            &budget.counters.write_bytes
        };
        assert_eq!(count.load(Ordering::Relaxed), limit);
        assert_eq!(budget.counters.sockets.load(Ordering::Relaxed), 0);
        budget.finish_inventory().await.unwrap();
    }
}

#[cfg(test)]
pub(crate) async fn terminal_denial_fixture(
    auth: &droutex_core::deployment::AuthorizedRequest<'_>,
    owner: Arc<droutex_core::deployment::Deployment>,
) {
    use crate::clock::Refresh;
    use droutex_core::deployment::{AdmissionCode, Cancellation};
    use std::collections::BTreeSet;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::watch,
    };
    for code in [
        AdmissionCode::Lease,
        AdmissionCode::Clock,
        AdmissionCode::Retired,
    ] {
        let refresh = Arc::new(Refresh::new());
        let upkeep = Arc::clone(&owner);
        assert!(
            refresh
                .perform(move || upkeep.refresh_authorization_clock(&Cancellation::default()))
                .await
                .unwrap()
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let stream = TcpStream::connect(address).await.unwrap();
        let (mut peer, _) = listener.accept().await.unwrap();
        let (_stop, stopped) = watch::channel(false);
        let live = Arc::new(Live::new(
            auth.dns_io_check().unwrap(),
            refresh,
            Instant::now() + Duration::from_millis(1500),
            Arc::new(AtomicBool::new(false)),
            stopped,
        ));
        let inventory = Arc::new(Inventory::fixture(Arc::new(Mutex::new(
            Ok(BTreeSet::new()),
        ))));
        let budget = Budget::new(
            address,
            Instant::now() + ATTEMPT,
            Arc::clone(&live),
            inventory,
        );
        budget.counters.sockets.store(1, Ordering::Relaxed);
        let mut bounded = BoundedTcp {
            inner: stream,
            budget: Arc::clone(&budget),
            read: Progress::default(),
            write: Progress::default(),
        };
        bounded.write_all(b"a").await.unwrap();
        let mut byte = [0];
        peer.read_exact(&mut byte).await.unwrap();
        assert_eq!(&byte, b"a");
        peer.write_all(b"b").await.unwrap();
        bounded.read_exact(&mut byte).await.unwrap();
        assert_eq!(&byte, b"b");
        // Only inject an additional denial after a genuine core check succeeds.
        // Core tests separately prove the actual lease/clock/generation causes.
        live.test_deny(Some(code));
        assert!(bounded.write_all(b"forbidden").await.is_err());
        peer.write_all(b"forbidden").await.unwrap();
        assert!(bounded.read_exact(&mut byte).await.is_err());
        assert_eq!(budget.counters.write_bytes.load(Ordering::Relaxed), 1);
        assert_eq!(budget.counters.read_bytes.load(Ordering::Relaxed), 1);
        drop(bounded);
        budget.finish_inventory().await.unwrap();
        let count = tokio::time::timeout(Duration::from_secs(1), peer.read(&mut byte))
            .await
            .unwrap();
        assert!(matches!(count, Ok(0) | Err(_)));
        assert_eq!(budget.counters.sockets.load(Ordering::Relaxed), 0);
    }
}
