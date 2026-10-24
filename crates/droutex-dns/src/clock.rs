use crate::{DnsError, ErrorCode, Result, budget::Budget};
use droutex_core::deployment::{Cancellation, Deployment};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{Notify, watch},
    time::{MissedTickBehavior, interval},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Checkpoint {
    Idle,
    Running(u64),
    Succeeded(u64),
    Failed(u64),
}
impl Checkpoint {
    pub fn sequence(self) -> u64 {
        match self {
            Self::Idle => 0,
            Self::Running(value) | Self::Succeeded(value) | Self::Failed(value) => value,
        }
    }
    pub fn next_sequence(self) -> Result<u64> {
        self.sequence()
            .checked_add(1)
            .ok_or(DnsError(ErrorCode::State))
    }
}
pub(super) struct Refresh {
    pub wake: Notify,
    status: watch::Sender<Checkpoint>,
    pub pending: Arc<AtomicBool>,
    pub cancel: Arc<Cancellation>,
}
struct Pending(Arc<AtomicBool>);
impl Drop for Pending {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl Refresh {
    pub fn new() -> Self {
        Self {
            wake: Notify::new(),
            status: watch::channel(Checkpoint::Idle).0,
            pending: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(Cancellation::default()),
        }
    }
    pub fn subscribe(&self) -> watch::Receiver<Checkpoint> {
        self.status.subscribe()
    }
    pub async fn perform(
        &self,
        operation: impl FnOnce() -> std::result::Result<(), droutex_core::deployment::DeploymentError>
        + Send
        + 'static,
    ) -> Result<bool> {
        let sequence = self.status.borrow().next_sequence()?;
        self.pending.store(true, Ordering::Release);
        self.status.send_replace(Checkpoint::Running(sequence));
        let pending = Pending(Arc::clone(&self.pending));
        let status = self.status.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let _pending = pending;
            let outcome = operation();
            // Publish before the worker-owned pending guard drops, even if the
            // async JoinHandle waiter has not resumed or has been cancelled.
            // Completion is only a signal; waiters recheck actual D46 facts.
            status.send_replace(if outcome.is_ok() {
                Checkpoint::Succeeded(sequence)
            } else {
                Checkpoint::Failed(sequence)
            });
            outcome
        })
        .await
        .map_err(|_| DnsError(ErrorCode::State))?;
        Ok(outcome.is_ok())
    }
    pub async fn run(
        self: Arc<Self>,
        deployment: Arc<Deployment>,
        budget: Arc<Budget>,
        mut stop: watch::Receiver<bool>,
    ) -> Result<()> {
        let mut tick = interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tokio::select! { biased;
                _=stop.changed()=>if *stop.borrow() || stop.has_changed().is_err() { break },
                _=self.wake.notified()=>(),
                _=tick.tick()=>(),
            }
            if *stop.borrow() {
                break;
            }
            // Only this worker starts maintenance, and it awaits each outcome.
            let owner = Arc::clone(&deployment);
            let cancel = Arc::clone(&self.cancel);
            let successful = self
                .perform(move || owner.refresh_authorization_clock(&cancel))
                .await?;
            if !successful {
                budget.clock_failures.fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(())
    }
}
