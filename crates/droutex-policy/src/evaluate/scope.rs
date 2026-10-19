use super::*;
use crate::CanonicalDomain;
use droutex_common::{DnsProfileBinding, GatewayNetworkIdentity};
use std::num::NonZeroU16;

impl<D, T> PolicySet<D, T> {
    pub fn new(
        profiles: Vec<PreparedProfile<D, T>>,
        endpoints: Vec<DnsEndpointAssignment>,
        groups: Vec<GatewayProfileAssignment>,
    ) -> Result<Self, PolicyBuildError> {
        check_budget(&profiles, endpoints.len(), groups.len())?;
        let profiles = unique(profiles, |p| p.id, PolicyBuildError::DuplicateProfile)?;
        let endpoints = unique(
            endpoints,
            |e| e.endpoint,
            PolicyBuildError::DuplicateEndpoint,
        )?;
        let groups = unique(groups, |g| g.group, PolicyBuildError::DuplicateGroup)?;
        for profile in endpoints
            .values()
            .map(|e| e.profile)
            .chain(groups.values().map(|g| g.profile))
        {
            if !profiles.contains_key(&profile) {
                return Err(PolicyBuildError::UnknownProfile(profile));
            }
        }
        Ok(Self {
            profiles,
            endpoints,
            groups: groups.into_iter().map(|(id, g)| (id, g.profile)).collect(),
        })
    }

    /// The caller must independently authenticate the request and establish its
    /// current binding/generation. This checks the immutable policy association.
    pub fn evaluate_dns(
        &self,
        binding: &DnsProfileBinding,
        query: &CanonicalDomain,
    ) -> Result<PolicySelection<'_, D>, EvaluationError>
    where
        D: Eq,
    {
        let endpoint = self
            .endpoints
            .get(&binding.endpoint)
            .ok_or(EvaluationError::UnknownEndpoint)?;
        if endpoint.state != AdministrativeState::Enabled {
            return Err(EvaluationError::UnavailableEndpoint);
        }
        if endpoint.profile != binding.profile {
            return Err(EvaluationError::EndpointBindingMismatch);
        }
        let profile = self
            .profiles
            .get(&endpoint.profile)
            .ok_or(EvaluationError::UnknownProfile)?;
        ordering::select(
            PolicyScope::DnsEndpoint(binding.endpoint),
            profile.id,
            &profile.dns_default,
            profile
                .dns_rules
                .iter()
                .filter(|rule| applicable(rule, query)),
        )
    }

    /// Requires independently established gateway identity and the actual TCP
    /// destination port. Selection never grants source/destination authorization.
    /// DNS identity cannot be upgraded to a gateway identity:
    /// ```compile_fail
    /// use droutex_policy::{CanonicalDomain, PolicySet};
    /// use droutex_common::DnsProfileBinding;
    /// use std::num::NonZeroU16;
    /// fn wrong(policy: &PolicySet<u8,u8>, dns: &DnsProfileBinding,
    ///          name: &CanonicalDomain, port: NonZeroU16) {
    ///     policy.evaluate_tcp(dns, name, port);
    /// }
    /// ```
    /// Missing port metadata cannot select a fallback implicitly:
    /// ```compile_fail
    /// use droutex_policy::{CanonicalDomain, PolicySet};
    /// use droutex_common::GatewayNetworkIdentity;
    /// fn missing(policy: &PolicySet<u8,u8>, identity: &GatewayNetworkIdentity,
    ///            name: &CanonicalDomain) {
    ///     policy.evaluate_tcp(identity, name, None);
    /// }
    /// ```
    pub fn evaluate_tcp(
        &self,
        identity: &GatewayNetworkIdentity,
        query: &CanonicalDomain,
        port: NonZeroU16,
    ) -> Result<PolicySelection<'_, T>, EvaluationError>
    where
        T: Eq,
    {
        let profile_id = self
            .groups
            .get(&identity.group)
            .ok_or(EvaluationError::UnknownGatewayGroup)?;
        let profile = self
            .profiles
            .get(profile_id)
            .ok_or(EvaluationError::UnknownProfile)?;
        ordering::select(
            PolicyScope::GatewayGroup(identity.group),
            *profile_id,
            &profile.tcp_default,
            profile
                .tcp_rules
                .iter()
                .filter(|entry| port_matches(entry.ports, port) && applicable(&entry.rule, query))
                .map(|entry| &entry.rule),
        )
    }
}

fn applicable<A>(rule: &DomainRule<A>, query: &CanonicalDomain) -> bool {
    rule.state == AdministrativeState::Enabled && rule.matcher.matches(query)
}

fn port_matches(scope: TcpPortScope, port: NonZeroU16) -> bool {
    match scope {
        TcpPortScope::Any => true,
        TcpPortScope::Range(range) => (range.first..=range.last).contains(&port.get()),
    }
}

fn check_budget<D, T>(
    profiles: &[PreparedProfile<D, T>],
    endpoints: usize,
    groups: usize,
) -> Result<(), PolicyBuildError> {
    let mut remaining = MAX_POLICY_ENTRIES;
    let counts = [profiles.len(), endpoints, groups].into_iter().chain(
        profiles
            .iter()
            .flat_map(|p| [p.dns_rules.len(), p.tcp_rules.len()]),
    );
    for count in counts {
        remaining = remaining
            .checked_sub(count)
            .ok_or(PolicyBuildError::EntryLimit)?;
    }
    Ok(())
}

fn unique<T, K: Copy + Ord>(
    items: Vec<T>,
    key: impl Fn(&T) -> K,
    duplicate: impl Fn(K) -> PolicyBuildError,
) -> Result<BTreeMap<K, T>, PolicyBuildError> {
    let mut table = BTreeMap::new();
    for item in items {
        let id = key(&item);
        if table.insert(id, item).is_some() {
            return Err(duplicate(id));
        }
    }
    Ok(table)
}
