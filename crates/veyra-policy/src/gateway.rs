//! The `gateway:` section (spec §10) and which names a node's gateway passes (§10.2).

use crate::compile::{Issue, IssueCode, Policy, Severity, issue};
use crate::decide::Subject;
use crate::dns_config::socket_addr;
use crate::model::{Action, BundleSpec, Mode};
use crate::name::Name;
use std::net::SocketAddr;

/// The checked `gateway:` section: this data plane is the gateway of `node`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    pub node: String,
    /// Where TLS clients connect; the destination comes from SNI.
    pub tls: Vec<SocketAddr>,
    /// Where HTTP clients connect; the destination comes from `Host`.
    pub http: Vec<SocketAddr>,
}

/// `None` when the bundle has no `gateway:` section.
pub(crate) fn compile_gateway(spec: &BundleSpec, issues: &mut Vec<Issue>) -> Option<GatewayConfig> {
    use IssueCode::*;
    let gateway = spec.gateway.as_ref()?;
    let error = |code, message: String| issue(Severity::Error, code, &[], message);

    match spec.nodes.iter().find(|n| n.id == gateway.node) {
        None => {
            let msg = format!("gateway.node: unknown node {}", gateway.node);
            issues.push(error(UnknownReference, msg));
        }
        Some(node) if node.gateway.is_none() => {
            let msg = format!(
                "gateway.node: node {} has no gateway addresses for DNS to hand out",
                node.id
            );
            issues.push(error(MissingGateway, msg));
        }
        Some(_) => {}
    }
    // Origins resolve through dns.upstreams, never through the policy (spec §10.3).
    if spec.dns.is_none() {
        let msg = "gateway needs a dns section: origins resolve through its default upstream group";
        issues.push(error(InvalidValue, msg.into()));
    }
    if gateway.tls.is_empty() && gateway.http.is_empty() {
        let msg = "gateway listens nowhere: give tls or http addresses";
        issues.push(error(InvalidValue, msg.into()));
    }
    let tls = gateway
        .tls
        .iter()
        .filter_map(|a| socket_addr(a, Some(443), "gateway.tls", issues))
        .collect();
    let http = gateway
        .http
        .iter()
        .filter_map(|a| socket_addr(a, Some(80), "gateway.http", issues))
        .collect();
    Some(GatewayConfig {
        node: gateway.node.clone(),
        tls,
        http,
    })
}

impl Policy {
    /// The checked `gateway:` section, if the bundle has one.
    pub fn gateway_config(&self) -> Option<&GatewayConfig> {
        self.gateway_config.as_ref()
    }

    /// Spec §10.2: the gateway of `node` passes `name` when DNS, in mode B, sends
    /// some profile or device to `node` for it. A connection does not say which
    /// profile it belongs to, so the gateway serves all of them.
    // ponytail: one decision per profile and device on every connection; index
    // names by node if bundles grow to thousands of profiles and devices.
    pub fn routes_to(&self, node: &str, name: &Name) -> bool {
        let profiles = self.profiles.keys().map(|p| Subject {
            profile: p.clone(),
            device: None,
        });
        let devices = self.devices.iter().map(|(d, p)| Subject {
            profile: p.clone(),
            device: Some(d.clone()),
        });
        profiles.chain(devices).any(|subject| {
            matches!(
                self.decide_in(&subject, name, Some(Mode::SmartdnsTcp)).route,
                Action::ProxyNode { node: to, .. } if to == node
            )
        })
    }
}
