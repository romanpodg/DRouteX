use crate::{DnsError, ErrorCode, Result};
use droutex_common::NetworkGroupId;
use hickory_proto::op::Message;
use std::{
    collections::BTreeMap,
    future::Future,
    sync::Mutex,
    task::{Context, Poll},
};
use tokio::{
    sync::{Notify, oneshot, watch},
    task::{Id, JoinSet},
};

const NODE_SLOTS: usize = 8;
const GROUP_SLOTS: usize = 2;

struct Owner {
    group: NetworkGroupId,
    cancel: watch::Sender<bool>,
}
struct State {
    accepting: bool,
    jobs: JoinSet<()>,
    owners: BTreeMap<Id, Owner>,
}

/// Root-owned registry. A cancelled processor drops only its ticket; the worker
/// keeps its transport owners and slot until their explicit cleanup has joined.
pub(crate) struct Registry {
    state: Mutex<State>,
    wake: Notify,
}
impl Registry {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                accepting: true,
                jobs: JoinSet::new(),
                owners: BTreeMap::new(),
            }),
            wake: Notify::new(),
        }
    }
    pub(super) fn start<F, Fut>(&self, group: NetworkGroupId, operation: F) -> Result<Ticket>
    where
        F: FnOnce(watch::Receiver<bool>) -> Fut,
        Fut: Future<Output = Result<Message>> + Send + 'static,
    {
        let mut state = self.state.lock().map_err(|_| DnsError(ErrorCode::State))?;
        if !state.accepting
            || state.owners.len() >= NODE_SLOTS
            || state
                .owners
                .values()
                .filter(|owner| owner.group == group)
                .count()
                >= GROUP_SLOTS
        {
            return Err(DnsError(ErrorCode::Unavailable));
        }
        let (cancel, stop) = watch::channel(false);
        let (answer, response) = oneshot::channel();
        let work = operation(stop);
        let job = state.jobs.spawn(async move {
            // Operation returns only after joining its owned transport/helper work.
            let result = work.await;
            let _ = answer.send(result);
        });
        state.owners.insert(
            job.id(),
            Owner {
                group,
                cancel: cancel.clone(),
            },
        );
        self.wake.notify_one();
        Ok(Ticket { cancel, response })
    }
    pub fn poll_complete(&self, context: &mut Context<'_>) -> Poll<Result<()>> {
        let mut state = self.state.lock().map_err(|_| DnsError(ErrorCode::State))?;
        match state.jobs.poll_join_next_with_id(context) {
            Poll::Ready(Some(result)) => {
                let (id, outcome) = match result {
                    Ok((id, ())) => (id, Ok(())),
                    Err(error) => (error.id(), Err(DnsError(ErrorCode::State))),
                };
                if state.owners.remove(&id).is_none() {
                    return Poll::Ready(Err(DnsError(ErrorCode::State)));
                }
                Poll::Ready(outcome)
            }
            Poll::Ready(None) | Poll::Pending => Poll::Pending,
        }
    }
    pub async fn completed(&self) -> Result<()> {
        loop {
            let wake = self.wake.notified();
            tokio::select! {
                result=std::future::poll_fn(|context| self.poll_complete(context))=>return result,
                _=wake=>(),
            }
        }
    }
    pub fn cancel(&self) -> Result<()> {
        let mut state = self.state.lock().map_err(|_| DnsError(ErrorCode::State))?;
        state.accepting = false;
        for owner in state.owners.values() {
            owner.cancel.send_replace(true);
        }
        self.wake.notify_one();
        Ok(())
    }
    pub async fn drain(&self) -> Result<()> {
        self.cancel()?;
        let mut failure = None;
        loop {
            if self
                .state
                .lock()
                .map_err(|_| DnsError(ErrorCode::State))?
                .owners
                .is_empty()
            {
                return failure.map_or(Ok(()), Err);
            }
            if let Err(error) = self.completed().await {
                failure.get_or_insert(error);
            }
        }
    }
}

pub(super) struct Ticket {
    cancel: watch::Sender<bool>,
    response: oneshot::Receiver<Result<Message>>,
}
impl Ticket {
    pub async fn answer(&mut self) -> Result<Message> {
        (&mut self.response)
            .await
            .map_err(|_| DnsError(ErrorCode::State))?
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::time::{Duration, timeout};
    fn group(n: u64) -> NetworkGroupId {
        format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
    }
    #[tokio::test]
    async fn cancelled_tickets_keep_all_slots_until_owned_cleanup_finishes() {
        let registry = Registry::new();
        let cleanup = Arc::new(tokio::sync::Semaphore::new(0));
        let started = Arc::new(tokio::sync::Semaphore::new(0));
        let operation = || {
            let cleanup = Arc::clone(&cleanup);
            let started = Arc::clone(&started);
            move |mut stop: watch::Receiver<bool>| async move {
                let _ = stop.wait_for(|cancelled| *cancelled).await;
                started.add_permits(1);
                cleanup.acquire().await.unwrap().forget();
                Err(DnsError(ErrorCode::Unavailable))
            }
        };
        let mut tickets = Vec::new();
        for n in 0..8 {
            if n == 2 {
                assert!(registry.start(group(0), operation()).is_err());
            }
            tickets.push(registry.start(group(n / 2), operation()).unwrap());
        }
        assert!(registry.start(group(99), operation()).is_err());
        drop(tickets);
        timeout(Duration::from_secs(1), started.acquire_many(8))
            .await
            .unwrap()
            .unwrap()
            .forget();
        assert!(registry.start(group(0), operation()).is_err());
        assert!(registry.start(group(99), operation()).is_err());
        cleanup.add_permits(1);
        timeout(Duration::from_secs(1), registry.completed())
            .await
            .unwrap()
            .unwrap();
        let ticket = registry.start(group(99), operation()).unwrap();
        assert!(registry.start(group(99), operation()).is_err());
        drop(ticket);
        cleanup.add_permits(8);
        timeout(Duration::from_secs(1), registry.drain())
            .await
            .unwrap()
            .unwrap();
        assert!(registry.start(group(99), operation()).is_err());
    }
}
