//! D52 tests-only I/O adapters. Tokio owns sockets; Hickory still owns DNS framing.

use hickory_net::runtime::{
    DnsUdpSocket, RuntimeProvider, TokioHandle, TokioRuntimeProvider, TokioTime,
    iocompat::AsyncIoTokioAsStd,
};
use std::{
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, ready},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpStream, UdpSocket},
    sync::watch,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HoldAt {
    Connect,
    Read,
    Write,
}
pub(super) struct Hold {
    pub at: HoldAt,
    pub reached: watch::Sender<bool>,
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
    hold: Option<Hold>,
}
impl Budget {
    pub fn new(peer: SocketAddr) -> Arc<Self> {
        Self::with_hold(peer, None)
    }
    pub fn with_hold(peer: SocketAddr, hold: Option<Hold>) -> Arc<Self> {
        Arc::new(Self {
            peer,
            deadline: Instant::now() + ATTEMPT,
            counters: Counters::default(),
            hold,
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
    fn held(&self, at: HoldAt) -> bool {
        if let Some(hold) = &self.hold
            && hold.at == at
        {
            hold.reached.send_replace(true);
            return true;
        }
        false
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
            let stream = timeout_at(
                budget.deadline,
                TokioRuntimeProvider::new().connect_tcp(server, bind, Some(ATTEMPT)),
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
            if stream.budget.held(HoldAt::Connect) {
                // Controlled connection-stage cancellation: the real socket is owned
                // here but is not exposed to the client until this future completes.
                std::future::pending::<()>().await;
            }
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
            let inner = timeout_at(budget.deadline, UdpSocket::bind(local))
                .await
                .map_err(|_| denied(Violation::Deadline))??;
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
        self.budget.check()?;
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
        self.budget.check()?;
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
        this.budget.check()?;
        let mut capacity = this.read.capacity(TCP_READ_BYTES, Violation::ReadBudget)?;
        if this
            .budget
            .hold
            .as_ref()
            .is_some_and(|hold| hold.at == HoldAt::Read)
        {
            if this.read.bytes != 0 && this.budget.held(HoldAt::Read) {
                return Poll::Pending;
            }
            capacity = capacity.min(1);
        }
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
        this.budget.check()?;
        let mut capacity = this.write.capacity(WRITE_BYTES, Violation::WriteBudget)?;
        if this
            .budget
            .hold
            .as_ref()
            .is_some_and(|hold| hold.at == HoldAt::Write)
        {
            if this.write.bytes != 0 && this.budget.held(HoldAt::Write) {
                return Poll::Pending;
            }
            capacity = capacity.min(1);
        }
        let count =
            ready!(Pin::new(&mut this.inner).poll_write(cx, &bytes[..bytes.len().min(capacity)]))?;
        this.write.record(
            count,
            &this.budget.counters.write_bytes,
            &this.budget.counters.write_operations,
        );
        this.budget.check()?;
        Poll::Ready(Ok(count))
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.budget.check()?;
        Pin::new(&mut this.inner).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}
impl Drop for BoundedTcp {
    fn drop(&mut self) {
        self.budget.counters.sockets.fetch_sub(1, Ordering::Relaxed);
    }
}
