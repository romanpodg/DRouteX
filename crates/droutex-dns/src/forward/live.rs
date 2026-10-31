use crate::{
    DnsError, ErrorCode, Result,
    clock::{Checkpoint, Refresh},
    runtime,
};
use droutex_core::deployment::DnsIoCheck;
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};
use tokio::{
    sync::watch,
    time::{Instant, timeout_at},
};

type Wait = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
enum Phase {
    Checking { waited: bool },
    Waiting(Wait),
    Denied,
}
struct Gate {
    completion: watch::Receiver<Checkpoint>,
    phase: Phase,
    cancelled: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
    waiters: [Option<Waker>; 2],
}
#[derive(Clone, Copy)]
enum Caller {
    Worker,
    Io,
}

/// One original reservation and one original checkpoint wait across this query.
/// The mutex protects only synchronous polling; no guard spans an await.
pub(super) struct Live {
    check: DnsIoCheck,
    refresh: Arc<Refresh>,
    cutoff: Instant,
    lost: Arc<AtomicBool>,
    gate: Mutex<Gate>,
    #[cfg(test)]
    denial: Mutex<Option<droutex_core::deployment::AdmissionCode>>,
}
impl Live {
    pub fn new(
        check: DnsIoCheck,
        refresh: Arc<Refresh>,
        cutoff: Instant,
        lost: Arc<AtomicBool>,
        stop: Option<watch::Receiver<bool>>,
    ) -> Self {
        Self {
            check,
            gate: Mutex::new(Gate {
                completion: refresh.subscribe(),
                phase: Phase::Checking { waited: false },
                waiters: [None, None],
                cancelled: stop.map(|mut stop| {
                    Box::pin(async move {
                        let _ = stop.wait_for(|stopped| *stopped).await;
                    }) as Pin<Box<dyn Future<Output = ()> + Send>>
                }),
            }),
            refresh,
            cutoff,
            lost,
            #[cfg(test)]
            denial: Mutex::new(None),
        }
    }
    pub fn attach_cancel(&self, mut stop: watch::Receiver<bool>) -> Result<()> {
        let mut gate = self.gate.lock().map_err(|_| self.denied())?;
        if gate.cancelled.is_some() {
            return Err(self.denied());
        }
        // Joining the upstream registry must not reset this query's checkpoint
        // phase/wait flag, original authority, cutoff or remembered role wakers.
        gate.cancelled = Some(Box::pin(async move {
            let _ = stop.wait_for(|stopped| *stopped).await;
        }));
        Ok(())
    }
    fn denied(&self) -> DnsError {
        self.lost.store(true, Ordering::Release);
        DnsError(ErrorCode::Admission)
    }
    pub fn poll(&self, context: &mut Context<'_>, deadline: Instant) -> Poll<Result<()>> {
        self.poll_caller(context, deadline, Caller::Io)
    }
    fn poll_caller(
        &self,
        context: &mut Context<'_>,
        deadline: Instant,
        caller: Caller,
    ) -> Poll<Result<()>> {
        if self.lost.load(Ordering::Acquire) {
            return Poll::Ready(Err(self.denied()));
        }
        if Instant::now() >= deadline.min(self.cutoff) {
            return Poll::Ready(Err(DnsError(ErrorCode::Timeout)));
        }
        let mut gate = self.gate.lock().map_err(|_| self.denied())?;
        // One worker plus one live transport I/O task. A subsequent TCP task
        // replaces the old I/O waker only after its owner has joined that task.
        let index = match caller {
            Caller::Worker => 0,
            Caller::Io => 1,
        };
        if gate.waiters[index]
            .as_ref()
            .is_none_or(|waker| !waker.will_wake(context.waker()))
        {
            gate.waiters[index] = Some(context.waker().clone());
        }
        let mut changed = false;
        if gate
            .cancelled
            .as_mut()
            .is_some_and(|future| future.as_mut().poll(context).is_ready())
        {
            gate.cancelled = None;
            gate.phase = Phase::Denied;
            changed = true;
        }
        let outcome = loop {
            match &mut gate.phase {
                Phase::Denied => break Poll::Ready(Err(self.denied())),
                Phase::Waiting(wait) => match wait.as_mut().poll(context) {
                    Poll::Pending => break Poll::Pending,
                    Poll::Ready(Ok(())) => {
                        gate.phase = Phase::Checking { waited: true };
                        changed = true;
                    }
                    Poll::Ready(Err(_)) => {
                        gate.phase = Phase::Denied;
                        changed = true;
                    }
                },
                Phase::Checking { waited } => {
                    let waited = *waited;
                    let before = runtime::begin_check(&mut gate.completion);
                    let current = self.check.verify_current().map_err(|error| error.code());
                    #[cfg(test)]
                    let current =
                        current.and_then(|()| self.denial.lock().unwrap().map_or(Ok(()), Err));
                    match current {
                        Ok(()) => break Poll::Ready(Ok(())),
                        Err(_) if waited => {
                            gate.phase = Phase::Denied;
                            changed = true;
                        }
                        Err(error) => {
                            let refresh = Arc::clone(&self.refresh);
                            let mut completion = gate.completion.clone();
                            let deadline = self.cutoff;
                            gate.phase = Phase::Waiting(Box::pin(async move {
                                runtime::wait_checkpoint(
                                    error,
                                    &refresh,
                                    &mut completion,
                                    before,
                                    deadline,
                                )
                                .await
                            }));
                            changed = true;
                        }
                    }
                }
            }
        };
        let wake = changed.then(|| gate.waiters.clone());
        drop(gate);
        // A shared watch future remembers only its most recent poller. Broadcast
        // each transition so the other caller cannot remain stranded on it.
        if let Some(wake) = wake {
            for waker in wake.into_iter().flatten() {
                waker.wake();
            }
        }
        outcome
    }
    pub async fn current(&self, deadline: Instant) -> Result<()> {
        timeout_at(
            deadline.min(self.cutoff),
            poll_fn(|context| self.poll_caller(context, deadline, Caller::Worker)),
        )
        .await
        .map_err(|_| DnsError(ErrorCode::Timeout))?
    }
    #[cfg(test)]
    pub fn test_deny(&self, code: Option<droutex_core::deployment::AdmissionCode>) {
        *self.denial.lock().unwrap() = code;
    }
}

#[cfg(test)]
pub(crate) async fn shared_waiter_fixture(
    check: DnsIoCheck,
    owner: Arc<droutex_core::deployment::Deployment>,
    cancel: bool,
) {
    use droutex_core::deployment::{AdmissionCode, Cancellation};
    use tokio::{sync::Notify, time::Duration};
    let refresh = Arc::new(Refresh::new());
    let (stop, stopped) = watch::channel(false);
    let live = Arc::new(Live::new(
        check,
        Arc::clone(&refresh),
        Instant::now() + Duration::from_millis(1500),
        Arc::new(AtomicBool::new(false)),
        Some(stopped),
    ));
    if !cancel {
        live.test_deny(Some(AdmissionCode::CheckpointRequired));
    }
    async fn role(live: Arc<Live>, seen: Arc<Notify>, hold: bool, caller: Caller) -> Result<()> {
        let deadline = Instant::now() + tokio::time::Duration::from_millis(500);
        poll_fn(|context| {
            let outcome = live.poll_caller(context, deadline, caller);
            seen.notify_one();
            match outcome {
                Poll::Ready(Ok(())) if hold => Poll::Pending,
                other => other,
            }
        })
        .await
    }
    let first_seen = Arc::new(Notify::new());
    let second_seen = Arc::new(Notify::new());
    let mut first = tokio::spawn(role(
        Arc::clone(&live),
        Arc::clone(&first_seen),
        cancel,
        Caller::Worker,
    ));
    first_seen.notified().await;
    let mut second = tokio::spawn(role(
        Arc::clone(&live),
        Arc::clone(&second_seen),
        cancel,
        Caller::Io,
    ));
    second_seen.notified().await;
    if cancel {
        stop.send_replace(true);
    } else {
        live.test_deny(None);
        assert!(
            refresh
                .perform(move || owner.refresh_authorization_clock(&Cancellation::default()))
                .await
                .unwrap()
        );
    }
    let one = tokio::time::timeout(Duration::from_millis(200), &mut first).await;
    let two = tokio::time::timeout(Duration::from_millis(200), &mut second).await;
    if one.is_err() {
        first.abort();
        let _ = first.await;
    }
    if two.is_err() {
        second.abort();
        let _ = second.await;
    }
    let one = one
        .expect("shared checkpoint/cancel future lost the first task waker")
        .unwrap();
    let two = two
        .expect("shared checkpoint/cancel future lost the second task waker")
        .unwrap();
    if cancel {
        assert_eq!(one.unwrap_err().code(), ErrorCode::Admission);
        assert_eq!(two.unwrap_err().code(), ErrorCode::Admission);
        // Model a caller whose early lost=false load happened before the other
        // caller acquired the mutex and completed cancellation. The locked gate
        // remains denied; that cached observation must not re-poll a done future.
        live.lost.store(false, Ordering::Relaxed);
        let mut context = Context::from_waker(Waker::noop());
        assert!(matches!(
            live.poll_caller(
                &mut context,
                Instant::now() + Duration::from_millis(100),
                Caller::Worker
            ),
            Poll::Ready(Err(DnsError(ErrorCode::Admission)))
        ));
    } else {
        one.unwrap();
        two.unwrap();
    }
}

#[cfg(test)]
pub(crate) async fn cache_transition_fixture(
    check: DnsIoCheck,
    owner: Arc<droutex_core::deployment::Deployment>,
) {
    use droutex_core::deployment::{AdmissionCode, Cancellation};
    use tokio::time::Duration;
    let refresh = Arc::new(Refresh::new());
    let lost = Arc::new(AtomicBool::new(false));
    let cutoff = Instant::now() + Duration::from_millis(1500);
    let live = Live::new(check, Arc::clone(&refresh), cutoff, Arc::clone(&lost), None);
    live.test_deny(Some(AdmissionCode::CheckpointRequired));
    let mut cache_check = Box::pin(live.current(cutoff));
    poll_fn(|cx| {
        assert!(cache_check.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    live.test_deny(None);
    assert!(
        refresh
            .perform(move || owner.refresh_authorization_clock(&Cancellation::default()))
            .await
            .unwrap()
    );
    cache_check.await.unwrap();
    let (_stop, stopped) = watch::channel(false);
    live.attach_cancel(stopped).unwrap();
    live.test_deny(Some(AdmissionCode::CheckpointRequired));
    assert_eq!(
        live.current(cutoff).await.unwrap_err().code(),
        ErrorCode::Admission,
        "cache-to-miss transition granted a second checkpoint wait"
    );
    assert!(lost.load(Ordering::Acquire));
}
