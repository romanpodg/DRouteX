//! Rule selection (spec §8.2.1) and the explanation `veyra explain` prints (spec §8.4).

use crate::compile::{Policy, Rule};
use crate::model::{Action, DefaultAction, Scope};
use crate::name::Name;
use crate::pattern::{Pattern, Specificity};
use serde::Serialize;
use std::cmp::Reverse;

/// The profile, and optionally the device, a decision is made for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    pub profile: String,
    pub device: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubjectError {
    #[error("give a profile or a device")]
    Missing,
    #[error("unknown profile {0:?}")]
    UnknownProfile(String),
    #[error("unknown device {0:?}")]
    UnknownDevice(String),
    #[error("device {device:?} belongs to profile {actual:?}, not {requested:?}")]
    ProfileMismatch {
        device: String,
        actual: String,
        requested: String,
    },
}

/// The precedence step that separated the winning rule from the runner-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecidedBy {
    Default,
    OnlyMatch,
    Specificity,
    Scope,
    Action,
    RuleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub policy: String,
    pub domain: String,
    pub profile: String,
    pub device: Option<String>,
    pub route: Action,
    pub upstream_group: Option<String>,
    pub matched_rule_id: Option<String>,
    pub matched_pattern: Option<String>,
    pub decided_by: DecidedBy,
}

/// Sort key for spec §8.2.1: specificity, then scope, then action and rule id.
type Key<'a> = (Specificity, u8, (u8, Reverse<&'a str>));

impl Policy {
    pub fn subject(
        &self,
        profile: Option<&str>,
        device: Option<&str>,
    ) -> Result<Subject, SubjectError> {
        match (profile, device) {
            (requested, Some(d)) => {
                let actual = self
                    .devices
                    .get(d)
                    .ok_or_else(|| SubjectError::UnknownDevice(d.into()))?;
                if let Some(p) = requested.filter(|p| p != actual) {
                    return Err(SubjectError::ProfileMismatch {
                        device: d.into(),
                        actual: actual.clone(),
                        requested: p.into(),
                    });
                }
                Ok(Subject {
                    profile: actual.clone(),
                    device: Some(d.into()),
                })
            }
            (Some(p), None) if self.profiles.contains_key(p) => Ok(Subject {
                profile: p.into(),
                device: None,
            }),
            (Some(p), None) => Err(SubjectError::UnknownProfile(p.into())),
            (None, None) => Err(SubjectError::Missing),
        }
    }

    pub fn decide(&self, subject: &Subject, name: &Name) -> Decision {
        let mut candidates: Vec<(Key<'_>, &Rule, &Pattern)> = self
            .rules
            .iter()
            .filter(|r| applies(&r.scope, subject))
            .filter_map(|r| {
                let best = r
                    .patterns
                    .iter()
                    .filter(|p| p.matches(name))
                    .max_by_key(|p| p.specificity())?;
                Some((
                    (best.specificity(), scope_rank(&r.scope), r.late_rank()),
                    r,
                    best,
                ))
            })
            .collect();
        candidates.sort_by(|a, b| b.0.cmp(&a.0));

        let mut decision = Decision {
            policy: self.name.clone(),
            domain: name.to_string(),
            profile: subject.profile.clone(),
            device: subject.device.clone(),
            route: match self.profiles.get(&subject.profile) {
                Some(DefaultAction::Block) => Action::Block {},
                _ => Action::Direct {},
            },
            upstream_group: None,
            matched_rule_id: None,
            matched_pattern: None,
            decided_by: DecidedBy::Default,
        };
        if let Some((best, rule, pattern)) = candidates.first() {
            decision.route = rule.action.clone();
            decision.upstream_group = rule.dns.as_ref().map(|d| d.upstream_group.clone());
            decision.matched_rule_id = Some(rule.id.clone());
            decision.matched_pattern = Some(pattern.to_string());
            decision.decided_by = match candidates.get(1) {
                None => DecidedBy::OnlyMatch,
                Some((second, ..)) if best.0 != second.0 => DecidedBy::Specificity,
                Some((second, ..)) if best.1 != second.1 => DecidedBy::Scope,
                Some((second, ..)) if best.2.0 != second.2.0 => DecidedBy::Action,
                Some(_) => DecidedBy::RuleId,
            };
        }
        decision
    }
}

fn applies(scope: &Scope, subject: &Subject) -> bool {
    match scope {
        Scope::Default => true,
        Scope::Profile(p) => *p == subject.profile,
        Scope::Device(d) => subject.device.as_deref() == Some(d.as_str()),
    }
}

fn scope_rank(scope: &Scope) -> u8 {
    match scope {
        Scope::Default => 0,
        Scope::Profile(_) => 1,
        Scope::Device(_) => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bundle, OnFailure};

    const EXAMPLE: &str = include_str!("../../../configs/examples/home.yaml");

    fn policy(rules: &str) -> Policy {
        let text = format!(
            "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
             nodes: [{{ id: de-01 }}, {{ id: us-01 }}]\n  \
             profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]\n  \
             devices: [{{ id: tv, profile: home }}, {{ id: phone, profile: home }}]\n  rules:\n{rules}"
        );
        Policy::compile(&Bundle::from_yaml(&text).unwrap())
            .unwrap()
            .0
    }

    fn example() -> Policy {
        Policy::compile(&Bundle::from_yaml(EXAMPLE).unwrap())
            .unwrap()
            .0
    }

    fn decide(p: &Policy, profile: Option<&str>, device: Option<&str>, domain: &str) -> Decision {
        p.decide(
            &p.subject(profile, device).unwrap(),
            &Name::parse(domain).unwrap(),
        )
    }

    fn proxy(node: &str) -> Action {
        Action::ProxyNode {
            node: node.into(),
            on_failure: OnFailure::Block,
        }
    }

    #[test]
    fn exact_exception_beats_suffix() {
        let d = decide(&example(), Some("home"), None, "login.media.example");
        assert_eq!(d.route, Action::Direct {});
        assert_eq!(d.matched_rule_id.as_deref(), Some("media-login-direct"));
        assert_eq!(d.decided_by, DecidedBy::Specificity);
    }

    #[test]
    fn suffix_matches_the_apex() {
        let d = decide(&example(), Some("home"), None, "media.example");
        assert_eq!(d.route, proxy("de-01"));
        assert_eq!(d.matched_pattern.as_deref(), Some("suffix:media.example"));
        assert_eq!(d.decided_by, DecidedBy::OnlyMatch);
    }

    #[test]
    fn wildcard_does_not_match_the_apex() {
        let p = policy(
            "    - { id: a, scope: default, match: { domains: [\"*.x.example\"] }, action: { type: block } }\n",
        );
        assert_eq!(
            decide(&p, Some("home"), None, "x.example").decided_by,
            DecidedBy::Default
        );
        assert_eq!(
            decide(&p, Some("home"), None, "a.x.example").route,
            Action::Block {}
        );
    }

    #[test]
    fn device_scope_beats_profile_scope() {
        let p = policy(
            "    - { id: a, scope: { profile: home }, match: { domains: [x.example] }, action: { type: proxy_node, node: de-01 } }\n    \
             - { id: b, scope: { device: tv }, match: { domains: [x.example] }, action: { type: proxy_node, node: us-01 } }\n",
        );
        let tv = decide(&p, None, Some("tv"), "x.example");
        assert_eq!(tv.route, proxy("us-01"));
        assert_eq!(tv.decided_by, DecidedBy::Scope);
        assert_eq!(
            decide(&p, None, Some("phone"), "x.example").route,
            proxy("de-01")
        );
    }

    #[test]
    fn block_beats_proxy_at_equal_specificity_and_scope() {
        let p = policy(
            "    - { id: a, scope: default, match: { domains: [\"suffix:x.example\"] }, action: { type: proxy_node, node: de-01 } }\n    \
             - { id: b, scope: default, match: { domains: [\"*.x.example\"] }, action: { type: block } }\n",
        );
        let d = decide(&p, Some("home"), None, "a.x.example");
        assert_eq!(d.route, Action::Block {});
        assert_eq!(d.decided_by, DecidedBy::Action);
    }

    #[test]
    fn smaller_rule_id_wins_the_last_tie() {
        let p = policy(
            "    - { id: b, scope: default, match: { domains: [\"suffix:x.example\"] }, action: { type: proxy_node, node: us-01 } }\n    \
             - { id: a, scope: default, match: { domains: [\"*.x.example\"] }, action: { type: proxy_node, node: de-01 } }\n",
        );
        let d = decide(&p, Some("home"), None, "a.x.example");
        assert_eq!(d.matched_rule_id.as_deref(), Some("a"));
        assert_eq!(d.decided_by, DecidedBy::RuleId);
    }

    #[test]
    fn no_match_uses_the_profile_default() {
        let p = policy("    []\n");
        let home = decide(&p, Some("home"), None, "x.example");
        assert_eq!(
            (home.route, home.decided_by),
            (Action::Direct {}, DecidedBy::Default)
        );
        assert_eq!(
            decide(&p, Some("strict"), None, "x.example").route,
            Action::Block {}
        );
    }

    #[test]
    fn device_rules_carry_the_upstream_group() {
        let d = decide(&example(), None, Some("work-laptop"), "git.corp.example");
        assert_eq!(d.profile, "home");
        assert_eq!(d.upstream_group.as_deref(), Some("corp"));
        assert_eq!(
            decide(&example(), None, Some("tv"), "git.corp.example").upstream_group,
            None
        );
    }

    #[test]
    fn idn_rules_match_idn_queries() {
        let p = policy(
            "    - { id: a, scope: default, match: { domains: [\"suffix:пример.рф\"] }, action: { type: block } }\n",
        );
        let d = decide(&p, Some("home"), None, "WWW.Пример.РФ.");
        assert_eq!(d.route, Action::Block {});
        assert_eq!(d.domain, "www.xn--e1afmkfd.xn--p1ai");
    }

    #[test]
    fn subject_errors() {
        let p = example();
        assert_eq!(p.subject(None, None), Err(SubjectError::Missing));
        assert_eq!(
            p.subject(Some("nobody"), None),
            Err(SubjectError::UnknownProfile("nobody".into()))
        );
        assert_eq!(
            p.subject(None, Some("toaster")),
            Err(SubjectError::UnknownDevice("toaster".into()))
        );
        assert!(matches!(
            p.subject(Some("other"), Some("tv")),
            Err(SubjectError::ProfileMismatch { .. })
        ));
        assert_eq!(
            p.subject(Some("home"), Some("tv"))
                .unwrap()
                .device
                .as_deref(),
            Some("tv")
        );
    }
}
