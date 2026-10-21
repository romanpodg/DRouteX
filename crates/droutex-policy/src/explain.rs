//! D44 pure selection diagnostics, without generation or admission authority.
use crate::{
    CanonicalDomain, DomainMatcher, DomainRule, EvaluationError, PolicySelection, PolicySet,
    TcpPortScope,
    evaluate::{ordering, scope},
};
use droutex_common::{AdministrativeState, DnsProfileBinding, GatewayNetworkIdentity};
use std::{fmt, num::NonZeroU16};

/// Why one prepared entry does or does not determine the selected action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceDisposition {
    Disabled,
    DomainMismatch,
    PortMismatch,
    Selected,
    EquivalentTie,
    LowerRank,
}

/// Borrowed metadata only; opaque actions and confidential configuration are excluded.
pub struct TraceEntry<'a> {
    rule: &'a DomainMatcher,
    source: crate::RuleSource,
    rank: ordering::RuleRank,
    state: AdministrativeState,
    ports: Option<TcpPortScope>,
    domain_matches: bool,
    port_matches: Option<bool>,
    disposition: TraceDisposition,
}
impl TraceEntry<'_> {
    pub fn matcher(&self) -> &DomainMatcher {
        self.rule
    }
    pub fn source(&self) -> crate::RuleSource {
        self.source
    }
    pub fn priority(&self) -> i32 {
        self.rank.priority()
    }
    pub fn state(&self) -> AdministrativeState {
        self.state
    }
    pub fn domain_matches(&self) -> bool {
        self.domain_matches
    }
    pub fn port_matches(&self) -> Option<bool> {
        self.port_matches
    }
    /// None is any port for TCP; DNS has no port eligibility dimension.
    pub fn port_range(&self) -> Option<(u16, u16)> {
        match self.ports {
            Some(TcpPortScope::Range(range)) => Some((range.first, range.last)),
            _ => None,
        }
    }
    pub fn disposition(&self) -> TraceDisposition {
        self.disposition
    }
}
impl fmt::Debug for TraceEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TraceEntry")
            .field("disposition", &self.disposition)
            .finish_non_exhaustive()
    }
}

/// Unbound diagnostics of one selected Profile/plane, not an authenticated report.
/// The entries and selected action cannot outlive the immutable policy owner.
/// ```compile_fail
/// use droutex_policy::{PolicySet, PolicyTrace, CanonicalDomain};
/// use droutex_common::DnsProfileBinding;
/// fn escape<'a>(policy: PolicySet<u8,u8>, binding: DnsProfileBinding, name: &CanonicalDomain) -> PolicyTrace<'a,u8> {
///     policy.explain_dns(&binding,name).unwrap()
/// }
/// ```
pub struct PolicyTrace<'a, A> {
    selection: PolicySelection<'a, A>,
    entries: Vec<TraceEntry<'a>>,
}
impl<'a, A> PolicyTrace<'a, A> {
    pub fn selection(&self) -> &PolicySelection<'a, A> {
        &self.selection
    }
    pub fn entries(&self) -> &[TraceEntry<'a>] {
        &self.entries
    }
}
impl<A> fmt::Debug for PolicyTrace<'_, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PolicyTrace")
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}

impl<D, T> PolicySet<D, T> {
    pub fn explain_dns(
        &self,
        binding: &DnsProfileBinding,
        query: &CanonicalDomain,
    ) -> Result<PolicyTrace<'_, D>, EvaluationError>
    where
        D: Eq,
    {
        // The actual evaluator owns all association/refusal and winner semantics.
        let selection = self.evaluate_dns(binding, query)?;
        let profile = self
            .profiles
            .get(&selection.profile())
            .ok_or(EvaluationError::UnknownProfile)?;
        let entries = trace(
            profile.dns_rules.iter().map(|rule| (rule, None)),
            query,
            None,
        )?;
        Ok(PolicyTrace { selection, entries })
    }
    pub fn explain_tcp(
        &self,
        identity: &GatewayNetworkIdentity,
        query: &CanonicalDomain,
        port: NonZeroU16,
    ) -> Result<PolicyTrace<'_, T>, EvaluationError>
    where
        T: Eq,
    {
        let selection = self.evaluate_tcp(identity, query, port)?;
        let profile = self
            .profiles
            .get(&selection.profile())
            .ok_or(EvaluationError::UnknownProfile)?;
        let entries = trace(
            profile
                .tcp_rules
                .iter()
                .map(|entry| (&entry.rule, Some(entry.ports))),
            query,
            Some(port),
        )?;
        Ok(PolicyTrace { selection, entries })
    }
}

fn trace<'a, A: Eq + 'a>(
    entries: impl Iterator<Item = (&'a DomainRule<A>, Option<TcpPortScope>)> + Clone,
    query: &CanonicalDomain,
    port: Option<NonZeroU16>,
) -> Result<Vec<TraceEntry<'a>>, EvaluationError> {
    // Reuse exactly the evaluator's highest-rank/conflict routine, including the
    // stable-source tie only between equivalent actions. No alternate winner.
    let selected = ordering::highest(entries.clone().filter_map(|(rule, ports)| {
        (scope::applicable(rule, query) && eligible(ports, port)).then_some(rule)
    }))?;
    let mut result: Vec<_> = entries
        .map(|(rule, ports)| {
            let domain_matches = rule.matcher.matches(query);
            let port_matches = port.map(|p| eligible(ports, Some(p)));
            let disposition = if rule.state != AdministrativeState::Enabled {
                TraceDisposition::Disabled
            } else if !domain_matches {
                TraceDisposition::DomainMismatch
            } else if port_matches == Some(false) {
                TraceDisposition::PortMismatch
            } else if let Some(selected) = selected {
                if rule.rank < selected.rank {
                    TraceDisposition::LowerRank
                } else if rule.source == selected.source {
                    TraceDisposition::Selected
                } else {
                    TraceDisposition::EquivalentTie
                }
            } else {
                // An eligible rule would have been returned by the actual selector.
                // This branch is unreachable for immutable inputs, without a panic.
                TraceDisposition::LowerRank
            };
            TraceEntry {
                rule: &rule.matcher,
                source: rule.source,
                rank: rule.rank,
                state: rule.state,
                ports,
                domain_matches,
                port_matches,
                disposition,
            }
        })
        .collect();
    result.sort_by(|a, b| {
        b.rank
            .cmp(&a.rank)
            .then_with(|| ordering::source_key(a.source).cmp(&ordering::source_key(b.source)))
            .then_with(|| a.rule.domain().cmp(b.rule.domain()))
            .then_with(|| a.port_range().cmp(&b.port_range()))
            .then_with(|| {
                (b.state == AdministrativeState::Enabled)
                    .cmp(&(a.state == AdministrativeState::Enabled))
            })
    });
    Ok(result)
}
fn eligible(scope: Option<TcpPortScope>, port: Option<NonZeroU16>) -> bool {
    match (scope, port) {
        (Some(scope), Some(port)) => scope::port_matches(scope, port),
        (None, None) => true,
        _ => false,
    }
}
