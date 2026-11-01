//! D54: private, whole-response cache. Public IDs and cached data grant no authority.
mod template;
use crate::{DnsError, ErrorCode, Result};
use droutex_common::{DnsUpstreamId, EndpointId, NetworkGroupId, ProfileId};
use droutex_config::snapshot::{ContentHash, SnapshotMetadata, SnapshotResource};
use droutex_core::deployment::{AdmissionScope, AuthorizedRequest};
use hickory_proto::op::Message;
use std::{
    mem::size_of,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
pub(super) use template::Template;
pub(crate) use template::UpstreamAnswer;
use tokio::time::Instant;

const ENTRIES: usize = 2048;
const SCOPE_ENTRIES: usize = 256;
const BYTES: usize = 16 * 1024 * 1024;
const SCOPE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Scope {
    endpoint: EndpointId,
    profile: ProfileId,
    group: NetworkGroupId,
}
// Only this domain is constructible. A future local/validated/origin producer
// must use a distinct domain and cannot reinterpret D53 transaction checks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Domain {
    ForwardedD53NonDnssec,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Key {
    generation: SnapshotMetadata,
    scope: Scope,
    upstream: DnsUpstreamId,
    resource: ContentHash,
    name: [u8; 255],
    name_len: usize,
    query_type: u16,
    class: u16,
    domain: Domain,
}
impl Key {
    pub fn admitted(
        auth: &AuthorizedRequest<'_>,
        upstream: DnsUpstreamId,
        request: &Message,
    ) -> Result<Self> {
        let AdmissionScope::LegacyDns(binding) = auth.scope() else {
            return Err(DnsError(ErrorCode::Scope));
        };
        let question = request
            .queries
            .first()
            .ok_or(DnsError(ErrorCode::Malformed))?;
        let mut name = [0; 255];
        let mut offset = 0;
        for label in question.name.iter() {
            let end = offset + 1 + label.len();
            if end >= name.len() || label.len() > 63 {
                return Err(DnsError(ErrorCode::Malformed));
            }
            name[offset] = u8::try_from(label.len()).map_err(|_| DnsError(ErrorCode::Malformed))?;
            for (out, input) in name[offset + 1..end].iter_mut().zip(label) {
                *out = input.to_ascii_lowercase();
            }
            offset = end;
        }
        let decision = auth.decision();
        let bytes = decision
            .artifact(SnapshotResource::Upstream(upstream))
            .ok_or(DnsError(ErrorCode::Scope))?;
        Ok(Self {
            generation: decision.metadata(),
            scope: Scope {
                endpoint: binding.endpoint,
                profile: binding.profile,
                group: auth.network_group(),
            },
            upstream,
            resource: ContentHash::of(bytes),
            name,
            name_len: offset + 1,
            query_type: u16::from(question.query_type),
            class: u16::from(question.query_class),
            domain: Domain::ForwardedD53NonDnssec,
        })
    }
}
struct Entry {
    key: Key,
    template: Template,
    order: u64,
}
struct State {
    slots: Vec<Option<Entry>>,
    next: u64,
}
pub(super) struct Cache {
    state: Mutex<Option<State>>,
    failures: AtomicU64,
}
impl Cache {
    pub fn new() -> Self {
        let mut slots = Vec::new();
        let state = if slots.try_reserve_exact(ENTRIES).is_ok()
            && slots
                .capacity()
                .checked_mul(size_of::<Option<Entry>>())
                .and_then(|n| n.checked_add(size_of::<Self>()))
                .is_some_and(|n| n <= BYTES)
        {
            slots.resize_with(ENTRIES, || None);
            Some(State { slots, next: 0 })
        } else {
            None
        };
        Self {
            failures: AtomicU64::new(u64::from(state.is_none())),
            state: Mutex::new(state),
        }
    }
    pub fn get(&self, key: &Key, now: Instant) -> Option<Template> {
        let Ok(state) = self.state.try_lock() else {
            self.failure();
            return None;
        };
        state
            .as_ref()?
            .slots
            .iter()
            .flatten()
            .find(|entry| entry.key == *key && entry.template.fresh(now))
            .map(|entry| entry.template.copy())
    }
    pub fn insert(&self, key: Key, template: Template, now: Instant) {
        if !template.fresh(now) {
            return;
        }
        let Ok(mut locked) = self.state.try_lock() else {
            self.failure();
            return;
        };
        let Some(state) = locked.as_mut() else {
            return;
        };
        // A finite reset rather than wrapped FIFO ordering. Never changes TTLs.
        let reset = state.next == u64::MAX;
        if reset {
            state.next = 0;
        }
        let mut free = None;
        let mut oldest = None;
        let mut scoped_oldest = None;
        let mut scope_count = 0usize;
        for (index, slot) in state.slots.iter_mut().enumerate() {
            if reset
                || slot.as_ref().is_some_and(|entry| {
                    !entry.template.fresh(now)
                        || (entry.key.generation.node == key.generation.node
                            && entry.key.generation.sequence < key.generation.sequence)
                })
            {
                *slot = None;
            }
            let Some(entry) = slot.as_ref() else {
                free = Some(index);
                continue;
            };
            if entry.key == key {
                return;
            } // First fresh insertion wins, without an age reset.
            if oldest.is_none_or(|(_, order)| entry.order < order) {
                oldest = Some((index, entry.order));
            }
            if entry.key.scope == key.scope {
                scope_count += 1;
                if scoped_oldest.is_none_or(|(_, order)| entry.order < order) {
                    scoped_oldest = Some((index, entry.order));
                }
            }
        }
        let scope_full = scope_count >= SCOPE_ENTRIES
            || (scope_count + 1) * size_of::<Option<Entry>>() > SCOPE_BYTES;
        let victim = if scope_full {
            scoped_oldest.map(|(index, _)| index)
        } else {
            free.or(oldest.map(|(index, _)| index))
        };
        if let Some(index) = victim {
            state.slots[index] = Some(Entry {
                key,
                template,
                order: state.next,
            });
            state.next += 1;
        }
    }
    fn failure(&self) {
        self.failures.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests;
