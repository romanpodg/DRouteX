//! Who may use the DNS engine, and as which profile or device (spec §9.6, §17.2).

use std::net::IpAddr;
use veyra_policy::{Bundle, Cidr, Issue, IssueCode, Policy, Subject};

fn compile(admission: &str, dns: bool) -> Result<(Policy, Vec<Issue>), Vec<Issue>> {
    let dns = if dns {
        "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams:\n      - { id: default, servers: [\"1.1.1.1\"] }\n"
    } else {
        ""
    };
    let text = format!(
        "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
         profiles: [{{ id: home }}, {{ id: kids }}]\n  \
         devices: [{{ id: tv, profile: home }}]\n  \
         admission:\n{admission}{dns}"
    );
    Policy::compile(&Bundle::from_yaml(&text).expect("test YAML parses"))
}

fn errors(admission: &str) -> Vec<IssueCode> {
    compile(admission, false)
        .expect_err("expected errors")
        .iter()
        .map(|i| i.code)
        .collect()
}

fn ip(s: &str) -> IpAddr {
    s.parse().expect("test address parses")
}

#[test]
fn cidrs_parse_normalize_and_match() {
    let lan = Cidr::parse("192.168.1.5/24").unwrap();
    assert_eq!(lan.to_string(), "192.168.1.0/24");
    assert!(lan.contains(ip("192.168.1.77")));
    assert!(!lan.contains(ip("192.168.2.1")));
    assert!(
        lan.contains(ip("::ffff:192.168.1.7")),
        "IPv4-mapped IPv6 source"
    );
    assert_eq!(Cidr::parse("10.0.0.1").unwrap().to_string(), "10.0.0.1/32");
    let v6 = Cidr::parse("2001:db8::/32").unwrap();
    assert!(v6.contains(ip("2001:db8:1::1")));
    assert!(!v6.contains(ip("192.168.1.1")));
    assert_eq!(Cidr::parse("0.0.0.0/0").unwrap().prefix_len(), 0);
    for bad in [
        "192.168.1.0/33",
        "::/129",
        "abc",
        "10.0.0.0/-1",
        "/24",
        "10.0.0.0/",
    ] {
        assert!(Cidr::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_longest_prefix_decides_who_the_source_is() {
    let admission = "    - { cidr: 192.168.1.0/24, profile: home }\n    \
                     - { cidr: 192.168.1.50/32, device: tv }\n";
    let (policy, _) = compile(admission, false).unwrap();
    let tv = policy.admit(ip("192.168.1.50")).unwrap();
    assert_eq!(
        tv.subject,
        Subject {
            profile: "home".into(),
            device: Some("tv".into())
        }
    );
    assert_eq!(tv.binding, "192.168.1.50/32");
    let other = policy.admit(ip("192.168.1.7")).unwrap();
    assert_eq!(
        (other.subject.device, other.binding.as_str()),
        (None, "192.168.1.0/24")
    );
    assert!(policy.admit(ip("10.0.0.1")).is_none());
}

#[test]
fn overlapping_bindings_of_different_profiles_conflict() {
    let admission = "    - { cidr: 192.168.1.0/24, profile: home }\n    \
                     - { cidr: 192.168.1.128/25, profile: kids }\n";
    assert_eq!(errors(admission), vec![IssueCode::SourceBindingConflict]);
}

#[test]
fn the_same_network_twice_conflicts() {
    let admission = "    - { cidr: 192.168.1.0/24, profile: home }\n    \
                     - { cidr: 192.168.1.0/24, profile: home }\n";
    assert_eq!(errors(admission), vec![IssueCode::SourceBindingConflict]);
}

#[test]
fn rejects_bad_bindings() {
    let admission = "    - { cidr: 10.0.0.0/8 }\n    \
                     - { cidr: 10.1.0.0/16, profile: home, device: tv }\n    \
                     - { cidr: 10.2.0.0/16, profile: nobody }\n    \
                     - { cidr: 10.3.0.0/33, profile: home }\n";
    assert_eq!(
        errors(admission),
        vec![
            IssueCode::InvalidBinding,
            IssueCode::InvalidBinding,
            IssueCode::UnknownReference,
            IssueCode::InvalidAddress
        ]
    );
}

#[test]
fn warns_when_dns_admits_nobody() {
    let (_, warnings) = compile("    []\n", true).unwrap();
    let codes: Vec<IssueCode> = warnings.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::NoAdmission]);
}
