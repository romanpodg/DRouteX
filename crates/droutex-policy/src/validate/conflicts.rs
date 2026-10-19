use super::PolicyPlane;
use crate::{
    DomainMatchMode, DomainRule, EvaluationError, TcpPortScope, TcpRule,
    evaluate::ordering::{RuleRank, highest, source_key},
};
use droutex_common::AdministrativeState;
use std::collections::{BTreeMap, BTreeSet};

struct Entry<'a, A> {
    rule: &'a DomainRule<A>,
    ports: TcpPortScope,
}

pub(super) fn dns<A: Eq>(rules: &[DomainRule<A>]) -> Option<EvaluationError> {
    validate(
        rules
            .iter()
            .map(|rule| Entry {
                rule,
                ports: TcpPortScope::Any,
            })
            .collect(),
        PolicyPlane::Dns,
    )
}

pub(super) fn tcp<A: Eq>(rules: &[TcpRule<A>]) -> Option<EvaluationError> {
    validate(
        rules
            .iter()
            .map(|r| Entry {
                rule: &r.rule,
                ports: r.ports,
            })
            .collect(),
        PolicyPlane::Tcp,
    )
}

struct Node<'a> {
    name: &'a str,
    labels: usize,
    children: BTreeMap<&'a str, usize>,
    entries: Vec<usize>,
}

impl<'a> Node<'a> {
    fn new(name: &'a str, labels: usize) -> Self {
        Self {
            name,
            labels,
            children: BTreeMap::new(),
            entries: Vec::new(),
        }
    }

    /// A symbolic atom for any unlisted next label. All its names have identical
    /// applicable rules. D37's canonical byte/label bounds can make it empty.
    fn unlisted_branch(&self) -> bool {
        if self.labels >= 127 || self.name.len() >= 252 {
            return false;
        }
        let label_bytes = 253 - self.name.len() - 1;
        if label_bytes >= 3 {
            // 26^3 lowercase ASCII labels are valid LTR labels even in a bidi
            // domain, and outnumber the entire 10,000-entry input. No ACE label
            // has three bytes. Thus at least one branch is necessarily unlisted.
            return true;
        }
        // Near the byte limit the alphabet is finite and small: exhaust ALL
        // one/two-byte canonical ASCII labels, using D37 itself for bidi/context
        // refusal. This proves emptiness; no sampled-name absence assumption.
        const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789-_";
        for &a in ALPHABET {
            let one = [a];
            if self.unlisted_label(&one) {
                return true;
            }
            if label_bytes == 2 {
                for &b in ALPHABET {
                    if self.unlisted_label(&[a, b]) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn unlisted_label(&self, bytes: &[u8]) -> bool {
        // These bytes come only from the literal ASCII alphabet above.
        let Ok(label) = std::str::from_utf8(bytes) else {
            return false;
        };
        !self.children.contains_key(label)
            && format!("{label}.{}", self.name)
                .parse::<crate::CanonicalDomain>()
                .is_ok()
    }
}

fn trie<'a, A>(entries: &[Entry<'a, A>]) -> Vec<Node<'a>> {
    let mut nodes = vec![Node::new("", 0)];
    for (id, entry) in entries.iter().enumerate() {
        let name = entry.rule.matcher.domain().as_str();
        let mut at = 0;
        let mut suffix_bytes = 0;
        for label in name.rsplit('.') {
            suffix_bytes += label.len() + usize::from(suffix_bytes != 0);
            let next = if let Some(&next) = nodes[at].children.get(label) {
                next
            } else {
                let next = nodes.len();
                let labels = nodes[at].labels + 1;
                nodes[at].children.insert(label, next);
                nodes.push(Node::new(&name[name.len() - suffix_bytes..], labels));
                next
            };
            at = next;
        }
        nodes[at].entries.push(id);
    }
    nodes
}

#[derive(Clone, Copy)]
enum Visit {
    Enter(usize),
    Descendants(usize),
    Leave(usize),
}

fn validate<A: Eq>(mut entries: Vec<Entry<'_, A>>, plane: PolicyPlane) -> Option<EvaluationError> {
    entries.retain(|e| e.rule.state == AdministrativeState::Enabled);
    let nodes = trie(&entries);
    let ports = (plane == PolicyPlane::Tcp).then(|| Ports::new(&entries));
    let mut active = BTreeSet::new();
    let mut version = 0;
    let mut checked = None;
    let mut stack = vec![Visit::Enter(0)];
    while let Some(visit) = stack.pop() {
        let test = match visit {
            Visit::Enter(at) => {
                let node = &nodes[at];
                for &id in &node.entries {
                    if entries[id].rule.matcher.mode() != DomainMatchMode::SubdomainsOnly {
                        active.insert(id);
                        version += 1;
                    }
                }
                stack.push(Visit::Descendants(at));
                at != 0
            }
            Visit::Descendants(at) => {
                let node = &nodes[at];
                for &id in &node.entries {
                    match entries[id].rule.matcher.mode() {
                        DomainMatchMode::Exact => {
                            active.remove(&id);
                            version += 1;
                        }
                        DomainMatchMode::SubdomainsOnly => {
                            active.insert(id);
                            version += 1;
                        }
                        DomainMatchMode::DomainAndSubdomains => {}
                    }
                }
                stack.push(Visit::Leave(at));
                stack.extend(node.children.values().rev().copied().map(Visit::Enter));
                at != 0 && node.unlisted_branch()
            }
            Visit::Leave(at) => {
                for &id in &nodes[at].entries {
                    if entries[id].rule.matcher.mode() != DomainMatchMode::Exact {
                        active.remove(&id);
                        version += 1;
                    }
                }
                continue;
            }
        };
        if test && checked != Some(version) {
            checked = Some(version);
            let conflict = if let Some(ports) = &ports {
                ports.conflict(&entries, &active)
            } else {
                highest(active.iter().map(|&id| entries[id].rule)).err()
            };
            if conflict.is_some() {
                return conflict;
            }
        }
    }
    None
}

/// Port boundary events, never individual ports. Effect classes are derived by
/// Eq once, within rank, without requiring Clone/Hash/Ord on opaque effects.
struct Ports {
    events: Vec<(u32, bool, usize)>,
    classes: Vec<usize>,
}

impl Ports {
    fn new<A: Eq>(entries: &[Entry<'_, A>]) -> Self {
        let mut classes = Vec::with_capacity(entries.len());
        let mut exemplars: BTreeMap<RuleRank, Vec<usize>> = BTreeMap::new();
        let mut events = Vec::with_capacity(entries.len() * 2);
        for (id, entry) in entries.iter().enumerate() {
            let representatives = exemplars.entry(entry.rule.rank).or_default();
            let same = representatives
                .iter()
                .copied()
                .find(|&other| entries[other].rule.action == entry.rule.action);
            classes.push(if let Some(other) = same {
                other
            } else {
                representatives.push(id);
                id
            });
            let (first, last) = match entry.ports {
                TcpPortScope::Any => (1, 65535),
                TcpPortScope::Range(range) => (u32::from(range.first), u32::from(range.last)),
            };
            events.push((first, true, id));
            events.push((last + 1, false, id));
        }
        events.sort_unstable();
        Self { events, classes }
    }

    fn conflict<A>(
        &self,
        entries: &[Entry<'_, A>],
        active: &BTreeSet<usize>,
    ) -> Option<EvaluationError> {
        let mut ranks: BTreeMap<RuleRank, BTreeMap<usize, BTreeSet<usize>>> = BTreeMap::new();
        let mut events = self.events.iter().peekable();
        while let Some(&(port, add, id)) = events.next() {
            self.event(entries, active, &mut ranks, add, id);
            while events.peek().is_some_and(|event| event.0 == port) {
                if let Some(&(_, add, id)) = events.next() {
                    self.event(entries, active, &mut ranks, add, id);
                }
            }
            if port > 65535 {
                continue;
            }
            if let Some((_, effects)) = ranks.last_key_value()
                && effects.len() > 1
            {
                let source = |&id: &usize| source_key(entries[id].rule.source);
                if let Some(&first) = effects
                    .values()
                    .flat_map(|ids| ids.iter())
                    .min_by_key(|id| source(id))
                    && let Some(&second) = effects
                        .iter()
                        .filter(|(class, _)| **class != self.classes[first])
                        .flat_map(|(_, ids)| ids.iter())
                        .min_by_key(|id| source(id))
                {
                    return Some(EvaluationError::Ambiguous {
                        first: entries[first].rule.source,
                        second: entries[second].rule.source,
                    });
                }
            }
        }
        None
    }

    fn event<A>(
        &self,
        entries: &[Entry<'_, A>],
        active: &BTreeSet<usize>,
        ranks: &mut BTreeMap<RuleRank, BTreeMap<usize, BTreeSet<usize>>>,
        add: bool,
        id: usize,
    ) {
        if !active.contains(&id) {
            return;
        }
        let rank = entries[id].rule.rank;
        let class = self.classes[id];
        if add {
            ranks
                .entry(rank)
                .or_default()
                .entry(class)
                .or_default()
                .insert(id);
        } else if let Some(effects) = ranks.get_mut(&rank) {
            if let Some(ids) = effects.get_mut(&class) {
                ids.remove(&id);
                if ids.is_empty() {
                    effects.remove(&class);
                }
            }
            if effects.is_empty() {
                ranks.remove(&rank);
            }
        }
    }
}
