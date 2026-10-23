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

pub(super) struct Refresh {
    pub wake: Notify,
    status: watch::Sender<bool>,
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
            status: watch::channel(false).0,
            pending: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(Cancellation::default()),
        }
    }
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.status.subscribe()
    }
    pub async fn perform(
        &self,
        operation: impl FnOnce() -> std::result::Result<(), droutex_core::deployment::DeploymentError>
        + Send
        + 'static,
    ) -> Result<bool> {
        self.pending.store(true, Ordering::Release);
        let pending = Pending(Arc::clone(&self.pending));
        let status = self.status.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let _pending = pending;
            let outcome = operation();
            // Publish before the worker-owned pending guard drops, even if the
            // async JoinHandle waiter has not resumed or has been cancelled.
            // Completion is only a signal; waiters recheck actual D46 facts.
            status.send_modify(|value| *value = !*value);
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
