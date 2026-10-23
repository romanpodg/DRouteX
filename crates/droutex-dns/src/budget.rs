use crate::{DnsError, ErrorCode, MAX_WIRE_BYTES, Result, ShutdownReport};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

struct Bucket {
    last: Instant,
    credits: u128,
    rate: u128,
    burst: u128,
}

impl Bucket {
    fn new(rate: u128, burst: u128) -> Self {
        Self {
            last: Instant::now(),
            credits: 0,
            rate,
            burst,
        }
    }
    fn take(&mut self) -> bool {
        self.take_at(Instant::now())
    }
    fn take_at(&mut self, now: Instant) -> bool {
        let Some(elapsed) = now.checked_duration_since(self.last) else {
            return false;
        };
        let Some(credits) = elapsed
            .as_nanos()
            .checked_mul(self.rate)
            .and_then(|n| self.credits.checked_add(n))
        else {
            return false;
        };
        self.last = now;
        self.credits = credits.min(self.burst * 1_000_000_000);
        if self.credits < 1_000_000_000 {
            return false;
        }
        self.credits -= 1_000_000_000;
        true
    }
}
pub(super) struct Budget {
    udp: Mutex<Bucket>,
    tcp: Mutex<Bucket>,
    pub contexts: Arc<Semaphore>,
    pub connections: Arc<Semaphore>,
    wire: Arc<Semaphore>,
    pub answered: AtomicU64,
    pub denied: AtomicU64,
    pub malformed: AtomicU64,
    pub timed_out: AtomicU64,
    pub clock_failures: AtomicU64,
}
impl Budget {
    pub fn new() -> Self {
        Self {
            udp: Mutex::new(Bucket::new(2000, 4000)),
            tcp: Mutex::new(Bucket::new(100, 200)),
            contexts: Arc::new(Semaphore::new(512)),
            connections: Arc::new(Semaphore::new(64)),
            wire: Arc::new(Semaphore::new(MAX_WIRE_BYTES)),
            answered: AtomicU64::new(0),
            denied: AtomicU64::new(0),
            malformed: AtomicU64::new(0),
            timed_out: AtomicU64::new(0),
            clock_failures: AtomicU64::new(0),
        }
    }
    pub fn intake(&self, tcp: bool) -> bool {
        let bucket = if tcp { &self.tcp } else { &self.udp };
        bucket.lock().is_ok_and(|mut b| b.take())
    }
    pub fn wire(&self, bytes: usize) -> Result<OwnedSemaphorePermit> {
        let amount = u32::try_from(bytes).map_err(|_| DnsError(ErrorCode::State))?;
        Arc::clone(&self.wire)
            .try_acquire_many_owned(amount)
            .map_err(|_| DnsError(ErrorCode::Unavailable))
    }
    pub fn report(&self) -> ShutdownReport {
        // Statistics publish no authority; relaxed ordering is sufficient.
        ShutdownReport {
            answered: self.answered.load(Ordering::Relaxed),
            denied: self.denied.load(Ordering::Relaxed),
            malformed: self.malformed.load(Ordering::Relaxed),
            timed_out: self.timed_out.load(Ordering::Relaxed),
            clock_failures: self.clock_failures.load(Ordering::Relaxed),
        }
    }
    pub fn refusal(&self, error: DnsError) {
        let counter = match error.0 {
            ErrorCode::Timeout => &self.timed_out,
            ErrorCode::Malformed => &self.malformed,
            _ => &self.denied,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn monotonic_buckets_refuse_empty_cap_burst_and_never_refund() {
        for (rate, burst) in [(2000, 4000), (100, 200)] {
            let mut bucket = Bucket::new(rate, burst);
            let start = bucket.last;
            assert!(!bucket.take_at(start));
            let filled = start + Duration::from_secs(100);
            for _ in 0..burst {
                assert!(bucket.take_at(filled));
            }
            assert!(!bucket.take_at(filled));
            assert!(!bucket.take_at(start));
            assert!(bucket.take_at(filled + Duration::from_secs(1)));
        }
    }
    #[test]
    fn global_wire_context_and_connection_credits_release_on_drop() {
        let budget = Budget::new();
        let all = budget.wire(MAX_WIRE_BYTES).unwrap();
        assert!(budget.wire(1).is_err());
        assert!(budget.wire(usize::MAX).is_err());
        drop(all);
        assert!(budget.wire(MAX_WIRE_BYTES).is_ok());
        let contexts: Vec<_> = (0..512)
            .map(|_| Arc::clone(&budget.contexts).try_acquire_owned().unwrap())
            .collect();
        assert!(Arc::clone(&budget.contexts).try_acquire_owned().is_err());
        drop(contexts);
        let connections: Vec<_> = (0..64)
            .map(|_| Arc::clone(&budget.connections).try_acquire_owned().unwrap())
            .collect();
        assert!(Arc::clone(&budget.connections).try_acquire_owned().is_err());
        drop(connections);
        assert_eq!(budget.contexts.available_permits(), 512);
        assert_eq!(budget.connections.available_permits(), 64);
    }
}
