//! REQ-DNS-002: what DNS answers for each route and query kind (spec §9.5).

use std::net::IpAddr;
use veyra_policy::{Bundle, DnsPlan, Mode, Name, Policy, QueryKind};

const BUNDLE: &str = r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: plan }
spec:
  nodes:
    - { id: de-01, gateway: { ipv4: 203.0.113.10, ipv6: "2001:db8::10" } }
    - { id: us-01, gateway: { ipv4: 198.51.100.7 } }
  profiles: [{ id: home }]
  admission: [{ cidr: 127.0.0.0/8, profile: home }]
  rules:
    - { id: corp, scope: default, match: { domains: [corp.example] }, dns: { upstreamGroup: corp }, action: { type: direct } }
    - { id: ads, scope: default, match: { domains: [ads.example] }, action: { type: block } }
    - { id: dual, scope: default, match: { domains: [dual.example] }, action: { type: proxy_node, node: de-01 } }
    - { id: v4, scope: default, match: { domains: [v4.example] }, action: { type: proxy_node, node: us-01 } }
  dns:
    listen: ["127.0.0.1:5353"]
    upstreams:
      - { id: default, servers: ["1.1.1.1"] }
      - { id: corp, servers: ["10.0.0.53"] }
"#;

fn plans(domain: &str) -> [DnsPlan; 5] {
    let (policy, _) = Policy::compile(&Bundle::from_yaml(BUNDLE).expect("test YAML parses"))
        .expect("test bundle compiles");
    let subject = policy.subject(Some("home"), None).expect("home exists");
    let decision = policy.decide_in(
        &subject,
        &Name::parse(domain).expect("test domain parses"),
        Some(Mode::SmartdnsTcp),
    );
    [
        QueryKind::A,
        QueryKind::Aaaa,
        QueryKind::Https,
        QueryKind::Svcb,
        QueryKind::Other,
    ]
    .map(|kind| policy.dns_plan(&decision, kind))
}

fn forward(group: &str) -> DnsPlan {
    DnsPlan::Forward {
        group: group.into(),
    }
}

fn addresses(ip: &str) -> DnsPlan {
    DnsPlan::Addresses(vec![ip.parse::<IpAddr>().expect("test address parses")])
}

#[test]
fn direct_forwards_every_kind_to_the_default_group() {
    assert_eq!(plans("other.example"), [(); 5].map(|_| forward("default")));
}

#[test]
fn direct_with_an_upstream_group_forwards_there() {
    assert_eq!(plans("corp.example"), [(); 5].map(|_| forward("corp")));
}

#[test]
fn block_answers_null_addresses_and_nodata() {
    assert_eq!(
        plans("ads.example"),
        [
            addresses("0.0.0.0"),
            addresses("::"),
            DnsPlan::NoData,
            DnsPlan::NoData,
            DnsPlan::NoData,
        ]
    );
}

#[test]
fn proxy_answers_the_gateway_and_hides_https_records() {
    assert_eq!(
        plans("dual.example"),
        [
            addresses("203.0.113.10"),
            addresses("2001:db8::10"),
            DnsPlan::NoData,
            DnsPlan::NoData,
            forward("default"),
        ]
    );
}

#[test]
fn proxy_without_gateway_ipv6_never_leaks_the_origin_aaaa() {
    assert_eq!(
        plans("v4.example"),
        [
            addresses("198.51.100.7"),
            DnsPlan::NoData,
            DnsPlan::NoData,
            DnsPlan::NoData,
            forward("default"),
        ]
    );
}
