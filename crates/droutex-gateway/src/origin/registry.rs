//! Owner of blocking inventory workers across cancellation of individual requests.
use super::inventory::InventoryError;
use std::{
    collections::BTreeMap,
    future::poll_fn,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::Poll,
};
use tokio::{
    sync::oneshot,
    task::{Id, JoinSet},
};

const MAX_TRACKED: usize = 8;
type ProbeResult = Result<(), InventoryError>;
struct State {
    accepting: bool,
    workers: JoinSet<()>,
    cancellations: BTreeMap<Id, Arc<AtomicBool>>,
    failure: bool,
}

/// Owned by the resolver until `shutdown` has joined all blocking workers.
/// Dropping a request cancels its probe but never removes it from this registry.
pub(super) struct Registry {
    state: Mutex<State>,
    drain: tokio::sync::Mutex<()>,
}
impl Registry {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                accepting: true,
                workers: JoinSet::new(),
                cancellations: BTreeMap::new(),
                failure: false,
            }),
            drain: tokio::sync::Mutex::new(()),
        }
    }
    pub fn start<F>(
        &self,
        cancel: Arc<AtomicBool>,
        work: F,
    ) -> Result<oneshot::Receiver<ProbeResult>, InventoryError>
    where
        F: FnOnce() -> ProbeResult + Send + 'static,
    {
        let runtime =
            tokio::runtime::Handle::try_current().map_err(|_| InventoryError::Unavailable)?;
        let mut state = self.state.lock().map_err(|_| InventoryError::State)?;
        while let Some(result) = state.workers.try_join_next_with_id() {
            Self::completed(&mut state, result);
        }
        if !state.accepting || state.failure {
            return Err(InventoryError::Unavailable);
        }
        if state.cancellations.len() >= MAX_TRACKED {
            return Err(InventoryError::Busy);
        }
        let (sender, receiver) = oneshot::channel();
        let task = state.workers.spawn_blocking_on(
            move || {
                let result = work();
                let _ = sender.send(result);
            },
            &runtime,
        );
        state.cancellations.insert(task.id(), cancel);
        Ok(receiver)
    }
    fn completed(state: &mut State, result: Result<(Id, ()), tokio::task::JoinError>) {
        let id = match result {
            Ok((id, ())) => id,
            Err(error) => {
                state.failure = true;
                error.id()
            }
        };
        if state.cancellations.remove(&id).is_none() {
            state.failure = true;
        }
    }
    /// Stop new workers, cancel outstanding probes and join every worker.
    /// Cancellation of this future leaves shutdown in effect and permits retry.
    pub async fn shutdown(&self) -> Result<(), InventoryError> {
        let _drain = self.drain.lock().await;
        {
            let mut state = self.state.lock().map_err(|_| InventoryError::State)?;
            state.accepting = false;
            for signal in state.cancellations.values() {
                signal.store(true, Ordering::Release);
            }
        }
        poll_fn(|cx| {
            let mut state = match self.state.lock() {
                Ok(state) => state,
                Err(_) => return Poll::Ready(Err(InventoryError::State)),
            };
            loop {
                match state.workers.poll_join_next_with_id(cx) {
                    Poll::Ready(Some(result)) => Self::completed(&mut state, result),
                    Poll::Ready(None) => {
                        return Poll::Ready(if state.failure || !state.cancellations.is_empty() {
                            Err(InventoryError::State)
                        } else {
                            Ok(())
                        });
                    }
                    Poll::Pending => return Poll::Pending,
                }
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shutdown_keeps_cancelled_worker_owned_until_joined() {
        let registry = Arc::new(Registry::new());
        let signal = Arc::new(AtomicBool::new(false));
        let worker_signal = Arc::clone(&signal);
        let (started_tx, started_rx) = oneshot::channel();
        let (cancelled_tx, cancelled_rx) = oneshot::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let receiver = registry
            .start(Arc::clone(&signal), move || {
                let _ = started_tx.send(());
                while !worker_signal.load(Ordering::Acquire) {
                    std::thread::yield_now();
                }
                let _ = cancelled_tx.send(());
                finish_rx.recv().map_err(|_| InventoryError::State)?;
                Ok(())
            })
            .unwrap();
        timeout(Duration::from_secs(2), started_rx)
            .await
            .unwrap()
            .unwrap();
        drop(receiver); // Losing the requester cannot detach the owned worker.
        let root = Arc::clone(&registry);
        let mut shutdown = tokio::spawn(async move { root.shutdown().await });
        timeout(Duration::from_secs(2), cancelled_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            registry.start(Arc::new(AtomicBool::new(false)), || Ok(())),
            Err(InventoryError::Unavailable)
        ));
        assert!(
            timeout(Duration::from_millis(50), &mut shutdown)
                .await
                .is_err(),
            "shutdown returned before the worker finished"
        );
        shutdown.abort();
        assert!(shutdown.await.unwrap_err().is_cancelled());
        let root = Arc::clone(&registry);
        let retry = tokio::spawn(async move { root.shutdown().await });
        finish_tx.send(()).unwrap();
        timeout(Duration::from_secs(2), retry)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        registry.shutdown().await.unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn tracked_workers_have_a_hard_limit_even_after_request_drop() {
        let registry = Registry::new();
        let release = Arc::new(AtomicBool::new(false));
        let mut receivers = Vec::new();
        for _ in 0..MAX_TRACKED {
            let gate = Arc::clone(&release);
            receivers.push(
                registry
                    .start(Arc::new(AtomicBool::new(false)), move || {
                        while !gate.load(Ordering::Acquire) {
                            std::thread::yield_now();
                        }
                        Ok(())
                    })
                    .unwrap(),
            );
        }
        drop(receivers);
        let refused = matches!(
            registry.start(Arc::new(AtomicBool::new(false)), || Ok(())),
            Err(InventoryError::Busy)
        );
        release.store(true, Ordering::Release);
        assert!(refused, "the ninth worker must be refused");
        timeout(Duration::from_secs(2), registry.shutdown())
            .await
            .unwrap()
            .unwrap();
    }
}
