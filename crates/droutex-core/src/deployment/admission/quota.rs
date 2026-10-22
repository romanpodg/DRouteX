use super::{AdmissionCode as Code, AdmissionError as Error, Plane, Result};
use droutex_common::NetworkGroupId;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

const GROUP_LIMIT: usize = droutex_config::MAX_RESOURCE_ENTRIES;
struct Bucket {
    credits: u128,
    last: Duration,
}
impl Bucket {
    fn empty(now: Duration) -> Self {
        Self {
            credits: 0,
            last: now,
        }
    }
    fn replenish(&mut self, now: Duration, rate: u128, burst: u128) -> Result<()> {
        let elapsed = now
            .checked_sub(self.last)
            .ok_or(Error(Code::Clock))?
            .as_nanos();
        self.credits = self
            .credits
            .checked_add(elapsed.checked_mul(rate).ok_or(Error(Code::State))?)
            .ok_or(Error(Code::State))?
            .min(burst * 1_000_000_000);
        self.last = now;
        Ok(())
    }
}
struct Counter {
    bucket: Bucket,
    inflight: usize,
}
impl Counter {
    fn new(now: Duration) -> Self {
        Self {
            bucket: Bucket::empty(now),
            inflight: 0,
        }
    }
}
struct Group {
    dns: Counter,
    gateway: Counter,
}
pub(super) struct Quotas {
    groups: BTreeMap<NetworkGroupId, Group>,
    dns: Counter,
    gateway: Counter,
}
impl Quotas {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            dns: Counter::new(Duration::ZERO),
            gateway: Counter::new(Duration::ZERO),
        }
    }
    pub fn preflight(&self, ids: &BTreeSet<NetworkGroupId>) -> Result<()> {
        let retained = self
            .groups
            .iter()
            .filter(|(id, g)| !ids.contains(id) && (g.dns.inflight != 0 || g.gateway.inflight != 0))
            .count();
        if ids
            .len()
            .checked_add(retained)
            .is_none_or(|n| n > GROUP_LIMIT)
        {
            return Err(Error(Code::Quota));
        }
        Ok(())
    }
    pub fn reconcile(&mut self, ids: &BTreeSet<NetworkGroupId>, now: Duration) {
        self.groups
            .retain(|id, g| ids.contains(id) || g.dns.inflight != 0 || g.gateway.inflight != 0);
        for id in ids {
            self.groups.entry(*id).or_insert_with(|| Group {
                dns: Counter::new(now),
                gateway: Counter::new(now),
            });
        }
    }
    pub fn acquire(&mut self, group: NetworkGroupId, plane: Plane, now: Duration) -> Result<()> {
        let g = self.groups.get_mut(&group).ok_or(Error(Code::Scope))?;
        let (local, global, rate, burst, total_rate, total_burst, local_max, total_max) =
            match plane {
                Plane::Dns => (&mut g.dns, &mut self.dns, 100, 200, 2000, 4000, 64, 512),
                Plane::Gateway => (
                    &mut g.gateway,
                    &mut self.gateway,
                    20,
                    40,
                    200,
                    400,
                    64,
                    1024,
                ),
            };
        local.bucket.replenish(now, rate, burst)?;
        global.bucket.replenish(now, total_rate, total_burst)?;
        if local.inflight >= local_max
            || global.inflight >= total_max
            || local.bucket.credits < 1_000_000_000
            || global.bucket.credits < 1_000_000_000
        {
            return Err(Error(Code::Quota));
        }
        local.bucket.credits -= 1_000_000_000;
        global.bucket.credits -= 1_000_000_000;
        local.inflight += 1;
        global.inflight += 1;
        Ok(())
    }
    pub fn release(&mut self, group: NetworkGroupId, plane: Plane) {
        if let Some(g) = self.groups.get_mut(&group) {
            let (local, global) = match plane {
                Plane::Dns => (&mut g.dns, &mut self.dns),
                Plane::Gateway => (&mut g.gateway, &mut self.gateway),
            };
            // Only a successful acquire constructs an owner; poisoning denies subsequent access.
            local.inflight = local.inflight.saturating_sub(1);
            global.inflight = global.inflight.saturating_sub(1);
        }
    }
}
