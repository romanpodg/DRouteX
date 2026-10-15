//! REQ-POL-001: the decision must not depend on the order of rules in the file.

use std::collections::BTreeSet;
use veyra_policy::{Bundle, Name, Policy};

const RULES: &str = r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: perm }
spec:
  nodes: [{ id: de-01 }, { id: us-01 }]
  profiles: [{ id: home }]
  devices: [{ id: tv, profile: home }]
  rules:
    - { id: a, scope: { profile: home }, match: { domains: ["suffix:media.example"] }, action: { type: proxy_node, node: de-01 } }
    - { id: b, scope: { profile: home }, match: { domains: ["*.media.example"] }, action: { type: proxy_node, node: us-01 } }
    - { id: c, scope: { profile: home }, match: { domains: [login.media.example] }, action: { type: direct } }
    - { id: d, scope: { device: tv }, match: { domains: ["suffix:media.example"] }, action: { type: direct } }
    - { id: e, scope: default, match: { domains: ["suffix:example"] }, action: { type: block } }
"#;

/// Every ordering of `items`, by Heap's algorithm.
fn permutations<T: Clone>(items: &mut [T], k: usize, out: &mut Vec<Vec<T>>) {
    if k <= 1 {
        out.push(items.to_vec());
        return;
    }
    permutations(items, k - 1, out);
    for i in 0..k - 1 {
        let j = if k % 2 == 0 { i } else { 0 };
        items.swap(j, k - 1);
        permutations(items, k - 1, out);
    }
}

#[test]
fn decisions_do_not_depend_on_rule_order() {
    let base = Bundle::from_yaml(RULES).unwrap();
    let mut orders = Vec::new();
    let mut rules = base.spec.rules.clone();
    let len = rules.len();
    permutations(&mut rules, len, &mut orders);
    let distinct: BTreeSet<Vec<&str>> = orders
        .iter()
        .map(|o| o.iter().map(|r| r.id.as_str()).collect())
        .collect();
    assert_eq!(distinct.len(), 120);

    let names = [
        "media.example",
        "x.media.example",
        "login.media.example",
        "other.example",
        "unrelated.test",
    ];
    let decide_all = |bundle: &Bundle| {
        let (policy, _) = Policy::compile(bundle).unwrap();
        let mut out = Vec::new();
        for (profile, device) in [(Some("home"), None), (None, Some("tv"))] {
            let subject = policy.subject(profile, device).unwrap();
            for n in names {
                out.push(policy.decide(&subject, &Name::parse(n).unwrap()));
            }
        }
        out
    };

    let expected = decide_all(&base);
    for order in orders {
        let mut bundle = base.clone();
        bundle.spec.rules = order;
        assert_eq!(decide_all(&bundle), expected);
    }
}
