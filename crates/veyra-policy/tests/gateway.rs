//! The `gateway:` section and which names a node's gateway passes (spec §10).

use std::net::SocketAddr;
use veyra_policy::{Bundle, Issue, IssueCode, Name, Policy, Severity};

/// Two gateway nodes and one without, two profiles and a device; `extra` is
/// appended under `spec:`.
fn compile(extra: &str) -> Result<(Policy, Vec<Issue>), Vec<Issue>> {
    let text = format!(
        "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
         nodes:\n    \
         - {{ id: de-01, gateway: {{ ipv4: 203.0.113.10 }} }}\n    \
         - {{ id: us-01, gateway: {{ ipv4: 198.51.100.20, ipv6: \"2001:db8::20\" }} }}\n    \
         - {{ id: bare }}\n  \
         profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]\n  \
         devices: [{{ id: tv, profile: home }}]\n  \
         rules:\n    \
         - {{ id: media, scope: {{ profile: home }}, match: {{ domains: [\"suffix:media.example\"] }}, action: {{ type: proxy_node, node: de-01 }} }}\n    \
         - {{ id: login, scope: {{ profile: home }}, match: {{ domains: [login.media.example] }}, action: {{ type: direct }} }}\n    \
         - {{ id: video, scope: {{ profile: strict }}, match: {{ domains: [\"suffix:video.example\"] }}, action: {{ type: proxy_node, node: de-01 }} }}\n    \
         - {{ id: tv-sport, scope: {{ device: tv }}, match: {{ domains: [sport.example] }}, action: {{ type: proxy_node, node: de-01 }} }}\n    \
         - {{ id: us, scope: default, match: {{ domains: [\"suffix:us.example\"] }}, action: {{ type: proxy_node, node: us-01 }} }}\n    \
         - {{ id: tunnel-only, scope: default, match: {{ domains: [tunnel.example] }}, action: {{ type: proxy_node, node: de-01 }}, modesAllowed: [tunnel] }}\n    \
         - {{ id: ads, scope: default, match: {{ domains: [\"suffix:ads.example\"] }}, action: {{ type: block }} }}\n\
         {extra}"
    );
    Policy::compile(&Bundle::from_yaml(&text).expect("test YAML parses"))
}

const DNS: &str = "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams: [{ id: default, servers: [\"1.1.1.1\"] }]\n";

/// Error codes only: warnings from other sections do not belong to these tests.
fn errors(extra: &str) -> Vec<IssueCode> {
    compile(extra)
        .expect_err("expected errors")
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.code)
        .collect()
}

fn addr(s: &str) -> SocketAddr {
    s.parse().expect("test address parses")
}

fn name(s: &str) -> Name {
    Name::parse(s).expect("test name parses")
}

#[test]
fn parses_the_gateway_section_with_default_ports() {
    let gateway = "  gateway:\n    node: de-01\n    tls: [\"0.0.0.0\", \"[::]:8443\"]\n    http: [\"127.0.0.1:8080\"]\n";
    let (policy, _warnings) = compile(&format!("{DNS}{gateway}")).unwrap();
    let config = policy.gateway_config().unwrap();
    assert_eq!(config.node, "de-01");
    assert_eq!(config.tls, vec![addr("0.0.0.0:443"), addr("[::]:8443")]);
    assert_eq!(config.http, vec![addr("127.0.0.1:8080")]);

    let gateways = policy.gateways();
    assert_eq!(
        gateways.keys().collect::<Vec<_>>(),
        ["de-01", "us-01"],
        "nodes with a gateway"
    );
    assert_eq!(
        gateways["us-01"].ipv6,
        Some("2001:db8::20".parse().unwrap())
    );
    assert!(compile(DNS).unwrap().0.gateway_config().is_none());
}

#[test]
fn rejects_bad_gateway_sections() {
    use IssueCode::*;
    let gateway = |body: &str| format!("{DNS}  gateway:\n{body}");
    assert_eq!(
        errors(&gateway("    node: nowhere\n    tls: [\"0.0.0.0\"]\n")),
        vec![UnknownReference]
    );
    assert_eq!(
        errors(&gateway("    node: bare\n    tls: [\"0.0.0.0\"]\n")),
        vec![MissingGateway]
    );
    assert_eq!(
        errors(&gateway("    node: de-01\n")),
        vec![InvalidValue],
        "no listeners"
    );
    assert_eq!(
        errors(&gateway(
            "    node: de-01\n    tls: [\"localhost:443\"]\n    http: [\"0.0.0.0:http\"]\n"
        )),
        vec![InvalidAddress, InvalidAddress]
    );
    assert_eq!(
        errors("  gateway:\n    node: de-01\n    tls: [\"0.0.0.0\"]\n"),
        vec![InvalidValue],
        "origins resolve through the dns section"
    );
}

#[test]
fn a_gateway_passes_what_dns_sends_it_for_any_profile_or_device() {
    let (policy, _warnings) = compile(DNS).unwrap();
    let passes = |node: &str, n: &str| policy.routes_to(node, &name(n));
    assert!(passes("de-01", "media.example"));
    assert!(passes("de-01", "www.media.example"));
    assert!(
        passes("de-01", "video.example"),
        "only strict routes it here; the gateway cannot tell profiles apart"
    );
    assert!(passes("de-01", "sport.example"), "a device rule routes it");
    assert!(passes("us-01", "www.us.example"));
    assert!(
        !passes("de-01", "login.media.example"),
        "home sends it direct, strict blocks it"
    );
    assert!(!passes("de-01", "us.example"), "routed to another node");
    assert!(!passes("de-01", "tunnel.example"), "not allowed in mode B");
    assert!(!passes("de-01", "ads.example"));
    assert!(
        !passes("de-01", "example.org"),
        "no rule: a profile default never proxies"
    );
}
