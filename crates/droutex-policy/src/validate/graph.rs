use std::collections::{BTreeMap, BTreeSet};

/// Iterative, deterministic DFS. Each bounded edge is visited once; long valid
/// chains cannot exhaust the call stack. Keys are public IDs or borrowed names.
pub(super) fn cycle<K: Copy + Ord>(edges: &BTreeMap<K, BTreeSet<K>>) -> Option<(K, K)> {
    let mut active = BTreeSet::new();
    let mut done = BTreeSet::new();
    for &root in edges.keys() {
        let mut stack = vec![(root, root, false)];
        while let Some((from, key, leaving)) = stack.pop() {
            if leaving {
                active.remove(&key);
                done.insert(key);
            } else if !done.contains(&key) {
                if !active.insert(key) {
                    return Some((from, key));
                }
                stack.push((key, key, true));
                if let Some(next) = edges.get(&key) {
                    stack.extend(next.iter().rev().map(|&child| (key, child, false)));
                }
            }
        }
    }
    None
}
