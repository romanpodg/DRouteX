use super::{DomainRule, EvaluationError, PolicyScope, PolicySelection, RuleMetadata, RuleSource};
use crate::{DomainMatchMode, DomainMatcher};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct RuleRank {
    layer: u8,
    priority: i32,
    labels: usize,
    mode: u8,
}

impl RuleRank {
    pub(super) fn new(metadata: RuleMetadata, matcher: &DomainMatcher) -> Self {
        // Explicit CR-0002 ranks, independent of enum declaration order.
        let layer = match metadata.source {
            RuleSource::Custom(_) => 3,
            RuleSource::Service { .. } => 2,
            RuleSource::Filter(_) => 1,
        };
        let mode = match matcher.mode() {
            DomainMatchMode::Exact => 2,
            DomainMatchMode::SubdomainsOnly => 1,
            DomainMatchMode::DomainAndSubdomains => 0,
        };
        Self {
            layer,
            priority: metadata.priority,
            labels: matcher.domain().as_str().split('.').count(),
            mode,
        }
    }
}

pub(crate) fn source_key(source: RuleSource) -> ([u8; 16], Option<[u8; 16]>) {
    match source {
        RuleSource::Custom(id) => (*id.as_bytes(), None),
        RuleSource::Service { rule, revision } => (*rule.as_bytes(), Some(*revision.as_bytes())),
        RuleSource::Filter(id) => (*id.as_bytes(), None),
    }
}

pub(crate) fn highest<'a, A: Eq>(
    rules: impl Iterator<Item = &'a DomainRule<A>> + Clone,
) -> Result<Option<&'a DomainRule<A>>, EvaluationError> {
    // Clone only the borrowed iterator position for two allocation-free passes.
    // First find the globally highest key; a lower-key conflict cannot win.
    let selected = rules.clone().max_by(|a, b| {
        a.rank
            .cmp(&b.rank)
            .then_with(|| source_key(b.source).cmp(&source_key(a.source)))
    });
    let Some(selected) = selected else {
        return Ok(None);
    };
    let conflict = rules
        .filter(|rule| rule.rank == selected.rank && rule.action != selected.action)
        .min_by_key(|rule| source_key(rule.source));
    if let Some(conflict) = conflict {
        return Err(EvaluationError::Ambiguous {
            first: selected.source,
            second: conflict.source,
        });
    }
    Ok(Some(selected))
}

pub(super) fn select<'a, A: Eq>(
    scope: PolicyScope,
    profile: droutex_common::ProfileId,
    fallback: &'a A,
    rules: impl Iterator<Item = &'a DomainRule<A>> + Clone,
) -> Result<PolicySelection<'a, A>, EvaluationError> {
    let selected = highest(rules)?;
    Ok(PolicySelection {
        scope,
        profile,
        source: selected.map(|rule| rule.source),
        action: selected.map_or(fallback, |rule| &rule.action),
    })
}
