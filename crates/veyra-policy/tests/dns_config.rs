//! The `dns:` section and node gateways (spec §9, §10.2, §17.2).

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use veyra_policy::{Bundle, GatewaySpec, Issue, IssueCode, Policy, Severity};

/// A bundle with one gateway node, one node without a gateway, and `extra`
/// appended under `spec:` (more rules, a `dns:` section, ...).
fn compile(rules: &str, extra: &str) -> Result<(Policy, Vec<Issue>), Vec<Issue>> {
    let text = format!(
        "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
         nodes:\n    \
         - {{ id: de-01, gateway: {{ ipv4: 203.0.113.10, ipv6: \"2001:db8::10\" }} }}\n    \
         - {{ id: us-01 }}\n  \
         profiles: [{{ id: home }}]\n  \
         rules:\n    \
         - {{ id: video, scope: default, match: {{ domains: [\"suffix:media.example\"] }}, action: {{ type: proxy_node, node: de-01 }} }}\n\
         {rules}{extra}"
    );
    Policy::compile(&Bundle::from_yaml(&text).expect("test YAML parses"))
}

const DNS: &str = "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams:\n      - { id: default, servers: [\"1.1.1.1\"] }\n";

/// Error codes only: warnings from other sections do not belong to these tests.
fn errors(rules: &str, extra: &str) -> Vec<IssueCode> {
    compile(rules, extra)
        .expect_err("expected errors")
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.code)
        .collect()
}

fn addr(s: &str) -> SocketAddr {
    s.parse().expect("test address parses")
}

#[test]
fn parses_listeners_upstreams_gateways_and_defaults() {
    let dns = "  dns:\n    listen: [\"127.0.0.1:5353\", \"[::1]:5353\"]\n    upstreams:\n      \
               - { id: default, servers: [\"1.1.1.1\", \"[2606:4700:4700::1111]:53\"] }\n      \
               - { id: corp, servers: [\"10.0.0.53:5300\"] }\n";
    let (policy, _warnings) = compile("", dns).unwrap();
    let config = policy.dns().unwrap();
    assert_eq!(
        config.listen,
        vec![addr("127.0.0.1:5353"), addr("[::1]:5353")]
    );
    assert_eq!(config.synth_ttl, 60);
    assert_eq!(
        config.upstreams["default"],
        vec![addr("1.1.1.1:53"), addr("[2606:4700:4700::1111]:53")]
    );
    assert_eq!(config.upstreams["corp"], vec![addr("10.0.0.53:5300")]);
    assert_eq!(
        policy.gateway("de-01"),
        Some(&GatewaySpec {
            ipv4: Ipv4Addr::new(203, 0, 113, 10),
            ipv6: Some(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x10)),
        })
    );
    assert_eq!(policy.gateway("us-01"), None);
}

#[test]
fn without_a_dns_section_the_bundle_is_rules_only() {
    let rules = "    - { id: us, scope: default, match: { domains: [us.example] }, action: { type: proxy_node, node: us-01 } }\n    \
                 - { id: corp, scope: default, match: { domains: [corp.example] }, dns: { upstreamGroup: nope }, action: { type: direct } }\n";
    let (policy, issues) = compile(rules, "").unwrap();
    assert!(issues.is_empty(), "{issues:?}");
    assert!(policy.dns().is_none());
}

#[test]
fn rejects_bad_addresses() {
    let dns = "  dns:\n    listen: [\"localhost:53\"]\n    upstreams:\n      - { id: default, servers: [\"1.1.1\"] }\n";
    assert_eq!(
        errors("", dns),
        vec![IssueCode::InvalidAddress, IssueCode::InvalidAddress]
    );
}

#[test]
fn requires_a_default_upstream_group() {
    let dns = "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams:\n      - { id: corp, servers: [\"10.0.0.53\"] }\n";
    assert_eq!(errors("", dns), vec![IssueCode::UnknownReference]);
}

#[test]
fn rejects_unknown_upstream_group_references() {
    let rules = "    - { id: corp, scope: default, match: { domains: [corp.example] }, dns: { upstreamGroup: nope }, action: { type: direct } }\n";
    assert_eq!(errors(rules, DNS), vec![IssueCode::UnknownReference]);
}

#[test]
fn rejects_synth_ttl_outside_30_to_300() {
    for ttl in ["5", "301"] {
        let dns = DNS.replace(
            "    upstreams:",
            &format!("    synthTtl: {ttl}\n    upstreams:"),
        );
        assert_eq!(errors("", &dns), vec![IssueCode::InvalidValue], "{ttl}");
    }
    let dns = DNS.replace("    upstreams:", "    synthTtl: 300\n    upstreams:");
    assert_eq!(compile("", &dns).unwrap().0.dns().unwrap().synth_ttl, 300);
}

#[test]
fn rejects_empty_listener_and_server_lists() {
    let dns = "  dns:\n    listen: []\n    upstreams:\n      - { id: default, servers: [] }\n";
    assert_eq!(
        errors("", dns),
        vec![IssueCode::InvalidValue, IssueCode::InvalidValue]
    );
}

#[test]
fn proxy_rules_served_over_dns_need_a_gateway() {
    let rules = "    - { id: us, scope: default, match: { domains: [us.example] }, action: { type: proxy_node, node: us-01 } }\n";
    assert_eq!(errors(rules, DNS), vec![IssueCode::MissingGateway]);
    let tunnel_only = "    - { id: us, scope: default, match: { domains: [us.example] }, action: { type: proxy_node, node: us-01 }, modesAllowed: [tunnel] }\n";
    assert!(compile(tunnel_only, DNS).is_ok());
}

#[test]
fn rejects_forwarding_to_itself() {
    for (listen, server) in [
        ("0.0.0.0:53", "127.0.0.1"),
        ("127.0.0.1:5353", "127.0.0.1:5353"),
    ] {
        let dns = format!(
            "  dns:\n    listen: [\"{listen}\"]\n    upstreams:\n      - {{ id: default, servers: [\"{server}\"] }}\n"
        );
        assert_eq!(
            errors("", &dns),
            vec![IssueCode::UpstreamLoop],
            "{listen} {server}"
        );
    }
    let other_port = "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams:\n      - { id: default, servers: [\"127.0.0.1\"] }\n";
    assert!(compile("", other_port).is_ok());
}
