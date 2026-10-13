//! The `dns:` section (spec §9): listeners, upstream groups, the TTL of
//! synthesized answers, and the checks rules need once DNS serves them.

use crate::compile::{Issue, IssueCode, Severity, issue, unique_ids};
use crate::model::{Action, BundleSpec, Mode};
use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};

/// Rules without `dns.upstreamGroup` resolve through this group.
pub const DEFAULT_UPSTREAM_GROUP: &str = "default";

/// The checked `dns:` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsConfig {
    pub listen: Vec<SocketAddr>,
    /// TTL of synthesized answers and of the SOA in synthesized NODATA (spec §9.5).
    pub synth_ttl: u32,
    pub upstreams: BTreeMap<String, Vec<SocketAddr>>,
}

/// `None` when the bundle has no `dns:` section.
pub(crate) fn compile_dns(spec: &BundleSpec, issues: &mut Vec<Issue>) -> Option<DnsConfig> {
    use IssueCode::*;
    let dns = spec.dns.as_ref()?;
    let error =
        |code, rules: &[&str], message: String| issue(Severity::Error, code, rules, message);

    if dns.listen.is_empty() {
        issues.push(error(InvalidValue, &[], "dns.listen is empty".into()));
    }
    let listen: Vec<SocketAddr> = dns
        .listen
        .iter()
        .filter_map(|a| socket_addr(a, None, "dns.listen", issues))
        .collect();
    if !(30..=300).contains(&dns.synth_ttl) {
        let msg = format!("dns.synthTtl is {}, expected 30 to 300", dns.synth_ttl);
        issues.push(error(InvalidValue, &[], msg));
    }

    let groups = unique_ids(
        "upstream group",
        dns.upstreams.iter().map(|g| g.id.as_str()),
        issues,
    );
    if !groups.contains(DEFAULT_UPSTREAM_GROUP) {
        let msg = format!("dns.upstreams needs a group with id {DEFAULT_UPSTREAM_GROUP}");
        issues.push(error(UnknownReference, &[], msg));
    }
    let mut upstreams = BTreeMap::new();
    for g in &dns.upstreams {
        let id = [g.id.as_str()];
        if g.servers.is_empty() {
            issues.push(error(
                InvalidValue,
                &id,
                "upstream group has no servers".into(),
            ));
        }
        let servers: Vec<SocketAddr> = g
            .servers
            .iter()
            .filter_map(|s| socket_addr(s, Some(53), "dns.upstreams.servers", issues))
            .collect();
        for server in &servers {
            if let Some(own) = listen.iter().find(|l| forwards_to_itself(*server, **l)) {
                let msg = format!("{server} is this server's own listener {own}");
                issues.push(error(UpstreamLoop, &id, msg));
            }
        }
        upstreams.insert(g.id.clone(), servers);
    }

    let has_gateway: BTreeMap<&str, bool> = spec
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n.gateway.is_some()))
        .collect();
    for r in &spec.rules {
        let id = [r.id.as_str()];
        if let Some(d) = r
            .dns
            .as_ref()
            .filter(|d| !groups.contains(d.upstream_group.as_str()))
        {
            let msg = format!("unknown upstream group {}", d.upstream_group);
            issues.push(error(UnknownReference, &id, msg));
        }
        if let Action::ProxyNode { node, .. } = &r.action {
            if r.modes_allowed.contains(&Mode::SmartdnsTcp)
                && has_gateway.get(node.as_str()) == Some(&false)
            {
                let msg = format!("node {node} has no gateway, so DNS cannot send clients to it");
                issues.push(error(MissingGateway, &id, msg));
            }
        }
    }

    Some(DnsConfig {
        listen,
        synth_ttl: dns.synth_ttl,
        upstreams,
    })
}

/// `ip:port` or `[ipv6]:port`; with a `default_port`, a bare IP too.
fn socket_addr(
    text: &str,
    default_port: Option<u16>,
    field: &str,
    issues: &mut Vec<Issue>,
) -> Option<SocketAddr> {
    let parsed = text
        .parse::<SocketAddr>()
        .ok()
        .or_else(|| Some(SocketAddr::new(text.parse::<IpAddr>().ok()?, default_port?)));
    if parsed.is_none() {
        let msg = format!("{field}: {text:?} is not a valid address");
        issues.push(issue(Severity::Error, IssueCode::InvalidAddress, &[], msg));
    }
    parsed
}

/// Forwarding to our own listener would loop forever (spec §9.4, `DNS_UPSTREAM_LOOP`).
// ponytail: catches exact and loopback matches only; another local interface
// address behind an unspecified listener needs an interface scan to detect.
fn forwards_to_itself(server: SocketAddr, listen: SocketAddr) -> bool {
    server.port() == listen.port()
        && (server.ip() == listen.ip()
            || server.ip().is_unspecified()
            || (listen.ip().is_unspecified() && server.ip().is_loopback()))
}
