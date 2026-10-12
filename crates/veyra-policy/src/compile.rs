//! Turns a parsed `Bundle` into a checked `Policy`, collecting every problem
//! instead of stopping at the first one (spec §8.2.1 validator rules, §17.2).

use crate::model::{
    API_VERSION, Action, Bundle, DefaultAction, DnsSpec, KIND, Mode, OnFailure, Scope,
};
use crate::pattern::Pattern;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCode {
    UnsupportedFormat,
    DuplicateId,
    UnknownReference,
    InvalidPattern,
    EmptyMatch,
    PatternConflict,
    TieBrokenLate,
    Shadowed,
    FailOpen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub code: IssueCode,
    pub rules: Vec<String>,
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{severity}[{:?}]", self.code)?;
        if !self.rules.is_empty() {
            write!(f, " {}", self.rules.join(", "))?;
        }
        write!(f, ": {}", self.message)
    }
}

/// An enabled rule with parsed patterns.
#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub scope: Scope,
    pub patterns: Vec<Pattern>,
    pub action: Action,
    pub dns: Option<DnsSpec>,
    pub modes: Vec<Mode>,
}

impl Rule {
    /// Spec §8.2.1 steps 3 and 4: BLOCK > PROXY > DIRECT, then the smaller rule id.
    pub(crate) fn late_rank(&self) -> (u8, Reverse<&str>) {
        let action = match self.action {
            Action::Block => 2,
            Action::ProxyNode { .. } => 1,
            Action::Direct => 0,
        };
        (action, Reverse(self.id.as_str()))
    }
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub name: String,
    pub(crate) profiles: BTreeMap<String, DefaultAction>,
    pub(crate) devices: BTreeMap<String, String>,
    pub(crate) rules: Vec<Rule>,
}

fn issue(severity: Severity, code: IssueCode, rules: &[&str], message: String) -> Issue {
    Issue {
        severity,
        code,
        rules: rules.iter().map(|r| r.to_string()).collect(),
        message,
    }
}

impl Policy {
    /// `Ok` carries warnings; `Err` carries every issue when at least one is an error.
    pub fn compile(bundle: &Bundle) -> Result<(Policy, Vec<Issue>), Vec<Issue>> {
        use IssueCode::*;
        use Severity::*;
        let mut issues = Vec::new();
        if bundle.api_version != API_VERSION || bundle.kind != KIND {
            issues.push(issue(
                Error,
                UnsupportedFormat,
                &[],
                format!(
                    "expected apiVersion {API_VERSION} and kind {KIND}, got {} and {}",
                    bundle.api_version, bundle.kind
                ),
            ));
        }
        let spec = &bundle.spec;
        let nodes = unique_ids(
            "node",
            spec.nodes.iter().map(|n| n.id.as_str()),
            &mut issues,
        );
        let profiles = unique_ids(
            "profile",
            spec.profiles.iter().map(|p| p.id.as_str()),
            &mut issues,
        );
        let devices = unique_ids(
            "device",
            spec.devices.iter().map(|d| d.id.as_str()),
            &mut issues,
        );
        unique_ids(
            "rule",
            spec.rules.iter().map(|r| r.id.as_str()),
            &mut issues,
        );

        for d in &spec.devices {
            if !profiles.contains(d.profile.as_str()) {
                let msg = format!("device {} uses unknown profile {}", d.id, d.profile);
                issues.push(issue(Error, UnknownReference, &[], msg));
            }
        }

        let mut rules = Vec::new();
        for r in &spec.rules {
            let id = [r.id.as_str()];
            match &r.scope {
                Scope::Profile(p) if !profiles.contains(p.as_str()) => {
                    issues.push(issue(
                        Error,
                        UnknownReference,
                        &id,
                        format!("unknown profile {p}"),
                    ));
                }
                Scope::Device(d) if !devices.contains(d.as_str()) => {
                    issues.push(issue(
                        Error,
                        UnknownReference,
                        &id,
                        format!("unknown device {d}"),
                    ));
                }
                _ => {}
            }
            if let Action::ProxyNode { node, on_failure } = &r.action {
                if !nodes.contains(node.as_str()) {
                    issues.push(issue(
                        Error,
                        UnknownReference,
                        &id,
                        format!("unknown node {node}"),
                    ));
                }
                if *on_failure == OnFailure::Direct {
                    let msg = format!(
                        "onFailure: direct sends traffic around the proxy when {node} is down"
                    );
                    issues.push(issue(Warning, FailOpen, &id, msg));
                }
            }
            if r.matcher.domains.is_empty() {
                issues.push(issue(
                    Error,
                    EmptyMatch,
                    &id,
                    "match.domains is empty".into(),
                ));
            }
            let mut patterns = Vec::new();
            for d in &r.matcher.domains {
                match Pattern::parse(d) {
                    Ok(p) => patterns.push(p),
                    Err(e) => issues.push(issue(Error, InvalidPattern, &id, format!("{d:?}: {e}"))),
                }
            }
            if r.enabled {
                rules.push(Rule {
                    id: r.id.clone(),
                    scope: r.scope.clone(),
                    patterns,
                    action: r.action.clone(),
                    dns: r.dns.clone(),
                    modes: r.modes_allowed.clone(),
                });
            }
        }
        check_overlaps(&rules, &mut issues);

        if issues.iter().any(|i| i.severity == Error) {
            return Err(issues);
        }
        let policy = Policy {
            name: bundle.metadata.name.clone(),
            profiles: spec
                .profiles
                .iter()
                .map(|p| (p.id.clone(), p.default_action))
                .collect(),
            devices: spec
                .devices
                .iter()
                .map(|d| (d.id.clone(), d.profile.clone()))
                .collect(),
            rules,
        };
        Ok((policy, issues))
    }

    /// Enabled rules in file order.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
}

fn unique_ids<'a>(
    kind: &str,
    ids: impl Iterator<Item = &'a str>,
    issues: &mut Vec<Issue>,
) -> BTreeSet<&'a str> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            let msg = format!("{kind} id {id} is used more than once");
            issues.push(issue(Severity::Error, IssueCode::DuplicateId, &[id], msg));
        }
    }
    seen
}

/// Pairwise checks inside one scope. Rules in different scopes are always
/// separated by step 2 of §8.2.1, so they never conflict.
// ponytail: O(rules² × patterns²); index by pattern base name if bundles reach thousands of rules.
fn check_overlaps(rules: &[Rule], issues: &mut Vec<Issue>) {
    for (i, a) in rules.iter().enumerate() {
        for b in rules
            .iter()
            .skip(i + 1)
            .filter(|b| b.scope == a.scope && b.action != a.action)
        {
            for p in &a.patterns {
                for q in &b.patterns {
                    let ids = [a.id.as_str(), b.id.as_str()];
                    if p == q {
                        let msg =
                            format!("both match {p} in the same scope with different actions");
                        issues.push(issue(
                            Severity::Error,
                            IssueCode::PatternConflict,
                            &ids,
                            msg,
                        ));
                    } else if same_tier(p, q) {
                        let msg = format!(
                            "{p} and {q} overlap at equal specificity; decided by action or rule id"
                        );
                        issues.push(issue(
                            Severity::Warning,
                            IssueCode::TieBrokenLate,
                            &ids,
                            msg,
                        ));
                    }
                }
            }
        }
    }
    for r in rules {
        // An identical pattern with another action is already a PatternConflict error.
        let hidden = |p: &Pattern| {
            rules.iter().any(|o| {
                o.id != r.id
                    && o.scope == r.scope
                    && o.late_rank() > r.late_rank()
                    && o.patterns.iter().any(|q| {
                        covers_at_same_specificity(q, p) && (q != p || o.action == r.action)
                    })
            })
        };
        if !r.patterns.is_empty() && r.patterns.iter().all(hidden) {
            let msg = "never decides: a stronger rule in the same scope matches every name at equal specificity";
            issues.push(issue(
                Severity::Warning,
                IssueCode::Shadowed,
                &[&r.id],
                msg.into(),
            ));
        }
    }
}

/// `q` matches every name `p` matches, at the same specificity. Only then can
/// `q`'s rule hide `p`'s rule completely: a broader `q` always loses step 1.
fn covers_at_same_specificity(q: &Pattern, p: &Pattern) -> bool {
    match (p, q) {
        (Pattern::Exact(a), Pattern::Exact(b)) | (Pattern::Suffix(a), Pattern::Suffix(b)) => a == b,
        (Pattern::Wildcard(a), Pattern::Wildcard(b) | Pattern::Suffix(b)) => a == b,
        _ => false,
    }
}

/// Two different patterns that overlap at equal specificity: only `*.x` and `suffix:x`.
fn same_tier(p: &Pattern, q: &Pattern) -> bool {
    matches!(
        (p, q),
        (Pattern::Wildcard(a), Pattern::Suffix(b)) | (Pattern::Suffix(a), Pattern::Wildcard(b)) if a == b
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use IssueCode::*;

    const EXAMPLE: &str = include_str!("../../../configs/examples/home.yaml");

    fn bundle(rules: &str) -> Bundle {
        let text = format!(
            "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
             nodes: [{{ id: de-01 }}, {{ id: us-01 }}]\n  profiles: [{{ id: home }}]\n  \
             devices: [{{ id: tv, profile: home }}]\n  rules:\n{rules}"
        );
        Bundle::from_yaml(&text).unwrap()
    }

    fn codes(issues: &[Issue]) -> Vec<IssueCode> {
        issues.iter().map(|i| i.code).collect()
    }

    fn errors(rules: &str) -> Vec<IssueCode> {
        codes(&Policy::compile(&bundle(rules)).unwrap_err())
    }

    fn warnings(rules: &str) -> Vec<IssueCode> {
        codes(&Policy::compile(&bundle(rules)).unwrap().1)
    }

    #[test]
    fn example_config_compiles_cleanly() {
        let (policy, issues) = Policy::compile(&Bundle::from_yaml(EXAMPLE).unwrap()).unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(policy.rules.len(), 4);
    }

    #[test]
    fn rejects_other_formats() {
        let mut b = bundle("    []\n");
        b.api_version = "veyra.io/v2".into();
        assert_eq!(
            codes(&Policy::compile(&b).unwrap_err()),
            vec![UnsupportedFormat]
        );
    }

    #[test]
    fn rejects_duplicate_rule_ids() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: a, scope: default, match: { domains: [y.example] }, action: { type: block } }\n";
        assert_eq!(errors(rules), vec![DuplicateId]);
    }

    #[test]
    fn rejects_unknown_references() {
        let rules = "    - { id: a, scope: { profile: nobody }, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: { device: phone }, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: c, scope: default, match: { domains: [x.example] }, action: { type: proxy_node, node: jp-01 } }\n";
        assert_eq!(
            errors(rules),
            vec![UnknownReference, UnknownReference, UnknownReference]
        );
    }

    #[test]
    fn rejects_bad_and_empty_patterns() {
        let rules = "    - { id: a, scope: default, match: { domains: [\"a..b\"] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [] }, action: { type: block } }\n";
        assert_eq!(errors(rules), vec![InvalidPattern, EmptyMatch]);
    }

    #[test]
    fn same_pattern_and_scope_with_different_action_is_an_error() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [x.example] }, action: { type: direct } }\n";
        assert_eq!(errors(rules), vec![PatternConflict]);
    }

    #[test]
    fn same_pattern_with_different_targets_is_an_error() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: proxy_node, node: de-01 } }\n    \
                     - { id: b, scope: default, match: { domains: [x.example] }, action: { type: proxy_node, node: us-01 } }\n";
        assert_eq!(errors(rules), vec![PatternConflict]);
    }

    #[test]
    fn same_pattern_in_different_scopes_is_fine() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: { profile: home }, match: { domains: [x.example] }, action: { type: direct } }\n";
        assert_eq!(warnings(rules), vec![]);
    }

    #[test]
    fn exact_exception_under_a_suffix_is_fine() {
        let rules = "    - { id: a, scope: default, match: { domains: [\"suffix:x.example\"] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [login.x.example] }, action: { type: direct } }\n";
        assert_eq!(warnings(rules), vec![]);
    }

    #[test]
    fn wildcard_and_suffix_of_the_same_base_warn() {
        let rules = "    - { id: a, scope: default, match: { domains: [\"*.x.example\"] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [\"suffix:x.example\"] }, action: { type: direct } }\n";
        assert_eq!(warnings(rules), vec![TieBrokenLate]);
    }

    #[test]
    fn duplicate_rule_is_reported_as_shadowed() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [x.example] }, action: { type: block } }\n";
        let (_, issues) = Policy::compile(&bundle(rules)).unwrap();
        assert_eq!(codes(&issues), vec![Shadowed]);
        assert_eq!(issues[0].rules, vec!["b".to_string()]);
    }

    #[test]
    fn fail_open_is_a_warning() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: proxy_node, node: de-01, onFailure: direct } }\n";
        assert_eq!(warnings(rules), vec![FailOpen]);
    }

    #[test]
    fn disabled_rules_do_not_conflict() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, enabled: false, scope: default, match: { domains: [x.example] }, action: { type: direct } }\n";
        let (policy, issues) = Policy::compile(&bundle(rules)).unwrap();
        assert!(issues.is_empty());
        assert_eq!(policy.rules.len(), 1);
    }

    #[test]
    fn issue_display_is_one_readable_line() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [x.example] }, action: { type: direct } }\n";
        let issues = Policy::compile(&bundle(rules)).unwrap_err();
        assert_eq!(
            issues[0].to_string(),
            "error[PatternConflict] a, b: both match x.example in the same scope with different actions"
        );
    }
}
