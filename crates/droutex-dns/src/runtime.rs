use crate::{
    DnsError, ErrorCode, ListenerSocket, MAX_LISTENERS, MAX_TCP_BYTES, MAX_UDP_BYTES, Result,
    ShutdownReport,
    budget::Budget,
    clock::Refresh,
    wire::{self, Processor},
};
use droutex_common::ListenerId;
use droutex_config::ListenerTransport;
use droutex_core::deployment::{AdmissionAttempt, AdmissionCode, Decision, Deployment};
use std::{
    collections::BTreeSet,
    future::{Future, poll_fn},
    net::SocketAddr,
    sync::{Arc, atomic::Ordering},
    task::Poll,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::{OwnedSemaphorePermit, mpsc, watch},
    task::JoinSet,
    time::{Instant, sleep_until},
};

const QUERY_TIME: Duration = Duration::from_secs(2);
const REPLY_RESERVE: Duration = Duration::from_millis(500);
const FRAME_TIME: Duration = Duration::from_secs(5);
const IDLE_TIME: Duration = Duration::from_secs(15);
const LIFETIME: Duration = Duration::from_secs(120);
const DRAIN_TIME: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
struct Binding {
    id: ListenerId,
    local: SocketAddr,
    tcp: bool,
}
impl Binding {
    fn check(&self, decision: &Decision<'_>) -> Result<()> {
        // D49: recv_from/send_to cannot preserve a wildcard datagram's local
        // destination. Refuse this unavailable capability before starting I/O.
        if !self.tcp && self.local.ip().is_unspecified() {
            return Err(DnsError(ErrorCode::Configuration));
        }
        let document = decision.dns_configuration().document();
        let transport = if self.tcp {
            ListenerTransport::DnsTcp
        } else {
            ListenerTransport::DnsUdp
        };
        if document.schema_version != 2
            || !document.listeners.iter().any(|listener| {
                listener.id == self.id
                    && listener.enabled
                    && listener.bind == self.local
                    && listener.transport == transport
            })
        {
            return Err(DnsError(ErrorCode::Configuration));
        }
        Ok(())
    }
}
struct Slots {
    _context: OwnedSemaphorePermit,
    _wire: OwnedSemaphorePermit,
}
enum Job {
    Udp {
        binding: Binding,
        socket: Arc<UdpSocket>,
        peer: SocketAddr,
        bytes: Vec<u8>,
        received: Instant,
        _slots: Slots,
    },
    Tcp {
        binding: Binding,
        socket: TcpStream,
        peer: SocketAddr,
        accepted: Instant,
        _slots: Slots,
        _connection: OwnedSemaphorePermit,
    },
}
enum Task {
    Worker(Result<()>),
    Request(Result<()>),
}
#[derive(Clone, Copy)]
enum FrameStart {
    First(Instant),
    Idle(Instant),
}
impl FrameStart {
    fn deadline(self) -> Instant {
        match self {
            Self::First(deadline) | Self::Idle(deadline) => deadline,
        }
    }
}

pub(super) async fn serve<P: Processor>(
    deployment: Arc<Deployment>,
    sockets: Vec<ListenerSocket>,
    mut shutdown: watch::Receiver<bool>,
    processor: P,
) -> Result<ShutdownReport> {
    if sockets.is_empty() || sockets.len() > MAX_LISTENERS {
        return Err(DnsError(ErrorCode::Configuration));
    }
    let mut ids = BTreeSet::new();
    {
        let current = deployment
            .pin()
            .map_err(|_| DnsError(ErrorCode::Unavailable))?;
        for socket in &sockets {
            let binding = match socket {
                ListenerSocket::Udp { id, socket } => Binding {
                    id: *id,
                    local: socket.local_addr().map_err(io_error)?,
                    tcp: false,
                },
                ListenerSocket::Tcp { id, socket } => Binding {
                    id: *id,
                    local: socket.local_addr().map_err(io_error)?,
                    tcp: true,
                },
            };
            if !ids.insert(binding.id) {
                return Err(DnsError(ErrorCode::Configuration));
            }
            binding.check(&current)?;
        }
    }
    let budget = Arc::new(Budget::new());
    let refresh = Arc::new(Refresh::new());
    let processor = Arc::new(processor);
    let (stop, stop_rx) = watch::channel(false);
    let (jobs, mut receive) = mpsc::channel(512);
    let mut tasks = JoinSet::new();
    for socket in sockets {
        let budget = Arc::clone(&budget);
        let jobs = jobs.clone();
        let stop = stop_rx.clone();
        tasks.spawn(async move {
            Task::Worker(match socket {
                ListenerSocket::Udp { id, socket } => {
                    udp_listener(id, socket, budget, jobs, stop).await
                }
                ListenerSocket::Tcp { id, socket } => {
                    tcp_listener(id, socket, budget, jobs, stop).await
                }
            })
        });
    }
    drop(jobs);
    let clock = Arc::clone(&refresh);
    let owner = Arc::clone(&deployment);
    let stats = Arc::clone(&budget);
    let clock_stop = stop_rx.clone();
    tasks.spawn(async move { Task::Worker(clock.run(owner, stats, clock_stop).await) });
    let mut failure = None;
    while !*shutdown.borrow() {
        tokio::select! { biased;
            changed=shutdown.changed()=>if changed.is_err() || *shutdown.borrow() { break },
            completed=tasks.join_next()=>match completed {
                Some(Ok(Task::Request(result)))=>if let Err(error)=result { budget.refusal(error); },
                Some(Ok(Task::Worker(result)))=>{ failure=Some(result.err().unwrap_or(DnsError(ErrorCode::State)));break; },
                _=>{ failure=Some(DnsError(ErrorCode::State));break; },
            },
            job=receive.recv()=>match job {
                Some(job)=>{
                    let owner=Arc::clone(&deployment);let stats=Arc::clone(&budget);
                    let clock=Arc::clone(&refresh);let process=Arc::clone(&processor);let stop=stop_rx.clone();
                    tasks.spawn(async move { Task::Request(job.run(owner,stats,clock,process,stop).await) });
                },
                None=>{failure=Some(DnsError(ErrorCode::State));break;},
            },
        }
    }
    stop.send_replace(true);
    refresh.cancel.cancel();
    receive.close();
    while receive.try_recv().is_ok() {} // Drop every queued socket/reservation.
    finish(&mut tasks, &refresh, &budget, failure).await
}

async fn finish(
    tasks: &mut JoinSet<Task>,
    refresh: &Refresh,
    budget: &Budget,
    mut failure: Option<DnsError>,
) -> Result<ShutdownReport> {
    let drain_deadline = Instant::now() + DRAIN_TIME;
    let drain = async {
        while let Some(result) = tasks.join_next().await {
            check_deadline(drain_deadline)?;
            match result {
                Ok(Task::Request(Err(error))) => budget.refusal(error),
                Ok(Task::Worker(Err(error))) => {
                    failure.get_or_insert(error);
                }
                Err(_) => {
                    failure.get_or_insert(DnsError(ErrorCode::State));
                }
                _ => (),
            }
        }
        Ok(())
    };
    if within_deadline(drain_deadline, drain).await.is_err() {
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }
    // A blocking atomic transaction is not cancelled by aborting its async waiter.
    if refresh.pending.load(Ordering::Acquire) {
        return Err(DnsError(ErrorCode::ShutdownIncomplete));
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(budget.report()),
    }
}

fn slots(budget: &Budget, tcp: bool) -> Result<Slots> {
    let context = Arc::clone(&budget.contexts)
        .try_acquire_owned()
        .map_err(|_| DnsError(ErrorCode::Unavailable))?;
    let wire = budget.wire(2 * if tcp { MAX_TCP_BYTES } else { MAX_UDP_BYTES })?;
    Ok(Slots {
        _context: context,
        _wire: wire,
    })
}
async fn udp_listener(
    id: ListenerId,
    socket: UdpSocket,
    budget: Arc<Budget>,
    jobs: mpsc::Sender<Job>,
    mut stop: watch::Receiver<bool>,
) -> Result<()> {
    let binding = Binding {
        id,
        local: socket.local_addr().map_err(io_error)?,
        tcp: false,
    };
    let socket = Arc::new(socket);
    let mut bytes = [0u8; MAX_UDP_BYTES + 1];
    loop {
        let (length, peer) = tokio::select! { biased;
            _=stop.changed()=>return Ok(()),
            result=socket.recv_from(&mut bytes)=>match result {
                Ok(received)=>received,
                // A truncated UDP datagram is reported as WSAEMSGSIZE on Windows.
                Err(error) if error.raw_os_error()==Some(10040)=>{budget.refusal(DnsError(ErrorCode::Malformed));continue;},
                // ICMP from a departed UDP client must not kill every listener.
                Err(error) if transient(&error)=>{budget.refusal(DnsError(ErrorCode::Io));continue;},
                Err(error)=>return Err(io_error(error)),
            }
        };
        if !budget.intake(false) {
            budget.refusal(DnsError(ErrorCode::Unavailable));
            continue;
        }
        if !(12..=MAX_UDP_BYTES).contains(&length) {
            budget.refusal(DnsError(ErrorCode::Malformed));
            continue;
        }
        let Ok(slots) = slots(&budget, false) else {
            budget.refusal(DnsError(ErrorCode::Unavailable));
            continue;
        };
        let job = Job::Udp {
            binding,
            socket: Arc::clone(&socket),
            peer,
            bytes: bytes[..length].to_vec(),
            received: Instant::now(),
            _slots: slots,
        };
        if jobs.try_send(job).is_err() {
            budget.refusal(DnsError(ErrorCode::Unavailable));
        }
    }
}
async fn tcp_listener(
    id: ListenerId,
    socket: TcpListener,
    budget: Arc<Budget>,
    jobs: mpsc::Sender<Job>,
    mut stop: watch::Receiver<bool>,
) -> Result<()> {
    let binding = Binding {
        id,
        local: socket.local_addr().map_err(io_error)?,
        tcp: true,
    };
    loop {
        let (socket, peer) = tokio::select! { biased;
            _=stop.changed()=>return Ok(()),
            result=socket.accept()=>match result {
                Ok(connection)=>connection,
                Err(error) if transient(&error)=>continue,
                Err(error)=>return Err(io_error(error)),
            },
        };
        if !budget.intake(true) {
            budget.refusal(DnsError(ErrorCode::Unavailable));
            continue;
        }
        let Ok(connection) = Arc::clone(&budget.connections).try_acquire_owned() else {
            budget.refusal(DnsError(ErrorCode::Unavailable));
            continue;
        };
        let Ok(slots) = slots(&budget, true) else {
            budget.refusal(DnsError(ErrorCode::Unavailable));
            continue;
        };
        let job = Job::Tcp {
            binding,
            socket,
            peer,
            accepted: Instant::now(),
            _slots: slots,
            _connection: connection,
        };
        if jobs.try_send(job).is_err() {
            budget.refusal(DnsError(ErrorCode::Unavailable));
        }
    }
}

async fn prepare<'a>(
    deployment: &'a Deployment,
    binding: Binding,
    peer: SocketAddr,
    refresh: &Refresh,
    deadline: Instant,
) -> Result<AdmissionAttempt<'a>> {
    check_deadline(deadline)?;
    binding.check(
        &deployment
            .pin()
            .map_err(|_| DnsError(ErrorCode::Unavailable))?,
    )?;
    let mut completion = refresh.subscribe();
    let attempt = match deployment.prepare_legacy_dns(binding.id, peer.ip()) {
        Ok(attempt) => Ok(attempt),
        Err(error) if error.code() == AdmissionCode::CheckpointRequired => {
            refresh.wake.notify_one();
            within_deadline(deadline, async {
                completion
                    .changed()
                    .await
                    .map_err(|_| DnsError(ErrorCode::State))
            })
            .await?;
            deployment
                .prepare_legacy_dns(binding.id, peer.ip())
                .map_err(|_| DnsError(ErrorCode::Unavailable))
        }
        Err(_) => Err(DnsError(ErrorCode::Unavailable)),
    }?;
    check_deadline(deadline)?;
    Ok(attempt)
}
impl Job {
    async fn run<P: Processor>(
        self,
        deployment: Arc<Deployment>,
        budget: Arc<Budget>,
        refresh: Arc<Refresh>,
        processor: Arc<P>,
        mut stop: watch::Receiver<bool>,
    ) -> Result<()> {
        match self {
            Self::Udp {
                binding,
                socket,
                peer,
                bytes,
                received,
                _slots,
            } => {
                let deadline = received + QUERY_TIME;
                within_deadline(deadline, async {
                    check_deadline(deadline)?;
                    let auth = prepare(&deployment, binding, peer, &refresh, deadline)
                        .await?
                        .consume()
                        .map_err(|_| DnsError(ErrorCode::Unavailable))?;
                    binding.check(auth.decision())?;
                    let request = wire::parse(&bytes)?;
                    let reply =
                        process_reply(&request, &auth, processor.as_ref(), &budget, deadline)
                            .await?;
                    let output = wire::encode(&request, &reply, bytes.len(), false)?;
                    let sent = submit(&auth, deadline, socket.send_to(&output, peer)).await?;
                    if sent != output.len() {
                        return Err(DnsError(ErrorCode::Io));
                    }
                    budget.answered.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                })
                .await
            }
            Self::Tcp {
                binding,
                mut socket,
                peer,
                accepted,
                _slots,
                _connection,
            } => {
                let lifetime = accepted + LIFETIME;
                for index in 0..100 {
                    // Admission maintenance consumes the existing idle/frame
                    // budget; it must not start a new15s wait after returning.
                    let start = if index == 0 {
                        FrameStart::First(accepted + FRAME_TIME)
                    } else {
                        FrameStart::Idle(Instant::now() + IDLE_TIME)
                    };
                    let attempt = tokio::select! { biased;
                        _=stop.changed()=>return Ok(()),
                        result=prepare(&deployment,binding,peer,&refresh,(Instant::now()+QUERY_TIME).min(lifetime).min(start.deadline()))=>result?,
                    };
                    let bytes = tokio::select! { biased;
                        _=stop.changed()=>return Ok(()),
                        result=read_frame(&mut socket,start,lifetime)=>result?,
                    };
                    let deadline = (Instant::now() + QUERY_TIME).min(lifetime);
                    within_deadline(deadline, async {
                        check_deadline(deadline)?;
                        let auth = attempt
                            .consume()
                            .map_err(|_| DnsError(ErrorCode::Unavailable))?;
                        binding.check(auth.decision())?;
                        let request = wire::parse(&bytes)?;
                        let reply =
                            process_reply(&request, &auth, processor.as_ref(), &budget, deadline)
                                .await?;
                        let output = wire::encode(&request, &reply, bytes.len(), true)?;
                        let prefix = u16::try_from(output.len())
                            .map_err(|_| DnsError(ErrorCode::State))?
                            .to_be_bytes();
                        submit(&auth, deadline, socket.write_all(&prefix)).await?;
                        submit(&auth, deadline, socket.write_all(&output)).await?;
                        budget.answered.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    })
                    .await?;
                }
                Ok(())
            }
        }
    }
}
async fn read_frame(
    socket: &mut TcpStream,
    start: FrameStart,
    lifetime: Instant,
) -> Result<Vec<u8>> {
    let mut prefix = [0; 2];
    let first_deadline = start.deadline().min(lifetime);
    check_deadline(first_deadline)?;
    within_deadline(first_deadline, async {
        socket.read_exact(&mut prefix[..1]).await.map_err(io_error)
    })
    .await?;
    check_deadline(first_deadline)?;
    let frame_deadline = match start {
        FrameStart::First(deadline) => deadline,
        FrameStart::Idle(_) => Instant::now() + FRAME_TIME,
    }
    .min(lifetime);
    within_deadline(frame_deadline, async {
        check_deadline(frame_deadline)?;
        socket
            .read_exact(&mut prefix[1..])
            .await
            .map_err(io_error)?;
        let length = usize::from(u16::from_be_bytes(prefix));
        if length == 0 {
            return Err(DnsError(ErrorCode::Malformed));
        }
        let mut bytes = vec![0; length];
        socket.read_exact(&mut bytes).await.map_err(io_error)?;
        check_deadline(frame_deadline)?;
        Ok(bytes)
    })
    .await
}
async fn process_reply<P: Processor>(
    request: &hickory_proto::op::Message,
    auth: &droutex_core::deployment::AuthorizedRequest<'_>,
    processor: &P,
    budget: &Budget,
    deadline: Instant,
) -> Result<hickory_proto::op::Message> {
    // D48: no processor work beyond1500ms from the original query start.
    // The500ms reserve remains inside D47's original total two-second bound.
    match within_deadline(
        deadline - REPLY_RESERVE,
        wire::answer(request, auth, processor),
    )
    .await
    {
        Err(DnsError(ErrorCode::Timeout)) => {
            budget.timed_out.fetch_add(1, Ordering::Relaxed);
            Ok(wire::response(
                request,
                hickory_proto::op::ResponseCode::ServFail,
            ))
        }
        result => result,
    }
}

async fn within_deadline<T>(
    deadline: Instant,
    future: impl Future<Output = Result<T>>,
) -> Result<T> {
    check_deadline(deadline)?;
    tokio::pin!(future);
    tokio::select! { biased;
        _=sleep_until(deadline)=>Err(DnsError(ErrorCode::Timeout)),
        result=poll_fn(|context| {
            // Tokio Timeout polls its inner future first. Check monotonic time on
            // EVERY resume, before a pending socket operation can submit bytes.
            if let Err(error)=check_deadline(deadline) {Poll::Ready(Err(error))}
            else {future.as_mut().poll(context)}
        })=>result,
    }
}
pub(super) async fn submit<T>(
    auth: &droutex_core::deployment::AuthorizedRequest<'_>,
    deadline: Instant,
    operation: impl Future<Output = std::io::Result<T>>,
) -> Result<T> {
    tokio::pin!(operation);
    within_deadline(
        deadline,
        poll_fn(|context| {
            // Pending I/O has not submitted bytes. Recheck actual current authority
            // on each resume, including between TCP prefix and payload writes.
            if auth.verify_current().is_err() {
                return Poll::Ready(Err(DnsError(ErrorCode::Unavailable)));
            }
            operation
                .as_mut()
                .poll(context)
                .map(|result| result.map_err(io_error))
        }),
    )
    .await
}
fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(DnsError(ErrorCode::Timeout))
    } else {
        Ok(())
    }
}
fn io_error(_: std::io::Error) -> DnsError {
    DnsError(ErrorCode::Io)
}
fn transient(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionRefused
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn buffered_frame_cannot_outlive_an_already_expired_absolute_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        client.write_all(b"\x00\x04data").await.unwrap();
        server.readable().await.unwrap();
        let past = Instant::now() - Duration::from_secs(1);
        assert_eq!(
            read_frame(&mut server, FrameStart::First(past), past + LIFETIME)
                .await
                .unwrap_err()
                .code(),
            ErrorCode::Timeout
        );
    }

    #[tokio::test]
    async fn expired_resume_never_polls_a_ready_operation() {
        let notify = tokio::sync::Notify::new();
        let entered = std::sync::atomic::AtomicBool::new(false);
        let submitted = std::sync::atomic::AtomicBool::new(false);
        let deadline = Instant::now() + Duration::from_millis(100);
        // This future stands at the socket's Pending/Ready boundary. The wake
        // arrives after expiry, when its submission side effect is ready too.
        let operation = async {
            entered.store(true, Ordering::Relaxed);
            notify.notified().await;
            submitted.store(true, Ordering::Relaxed);
            Ok(())
        };
        let mut guarded = Box::pin(within_deadline(deadline, operation));
        poll_fn(|context| {
            assert!(guarded.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        // Keep the guarded future unpolled until both timer and I/O are ready.
        tokio::time::sleep(Duration::from_millis(120)).await;
        notify.notify_one();
        assert_eq!(guarded.await.unwrap_err().code(), ErrorCode::Timeout);
        assert!(entered.load(Ordering::Relaxed));
        assert!(!submitted.load(Ordering::Relaxed));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn forced_join_releases_async_sockets_and_reports_unfinished_blocking_work() {
        let refresh = Arc::new(Refresh::new());
        let budget = Budget::new();
        let mut tasks = JoinSet::new();
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let permit = Arc::clone(&budget.contexts).try_acquire_owned().unwrap();
        tasks.spawn(async move {
            std::future::pending::<()>().await;
            drop(socket);
            drop(permit);
            Task::Request(Ok(()))
        });
        let (release, blocked) = std::sync::mpsc::channel::<()>();
        let clock = Arc::clone(&refresh);
        let entered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let signal = Arc::clone(&entered);
        tasks.spawn(async move {
            Task::Worker(
                clock
                    .perform(move || {
                        signal.store(true, Ordering::Release);
                        // Bounded even if an assertion panics: no detached permanent thread.
                        let _ = blocked.recv_timeout(Duration::from_secs(8));
                        Ok(())
                    })
                    .await
                    .map(|_| ()),
            )
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            while !entered.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            finish(&mut tasks, &refresh, &budget, None)
                .await
                .unwrap_err()
                .code(),
            ErrorCode::ShutdownIncomplete
        );
        assert!(tasks.is_empty());
        assert_eq!(budget.contexts.available_permits(), 512);
        let _rebound = TcpListener::bind(address).await.unwrap();
        assert!(refresh.pending.load(Ordering::Acquire));
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), async {
            while refresh.pending.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}
