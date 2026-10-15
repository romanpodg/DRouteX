//! Where the gateway may connect (spec §19.2 item 4, REQ-GW-003).

use std::net::IpAddr;
use veyra_gateway::{Egress, SPECIAL_PURPOSE};
use veyra_policy::{Bundle, Cidr, Policy};

/// Two nodes with gateways, DNS on a specific and an unspecified address, and
/// a gateway on a specific TLS address.
fn policy() -> Policy {
    let yaml = r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: egress }
spec:
  nodes:
    - { id: de-01, gateway: { ipv4: 5.6.7.8, ipv6: "2a01:4f8::8" } }
    - { id: us-01, gateway: { ipv4: 9.9.8.8 } }
  profiles: [{ id: home }]
  admission: [{ cidr: 127.0.0.1/32, profile: home }]
  dns:
    listen: ["0.0.0.0:53", "5.6.7.9:53"]
    upstreams: [{ id: default, servers: ["1.1.1.1"] }]
  gateway: { node: de-01, tls: ["5.6.7.10"], http: ["[::]:80"] }
"#;
    let bundle = Bundle::from_yaml(yaml).expect("test YAML parses");
    Policy::compile(&bundle).expect("test bundle compiles").0
}

fn ip(s: &str) -> IpAddr {
    s.parse().expect("test address parses")
}

#[test]
fn every_built_in_network_parses() {
    for network in SPECIAL_PURPOSE {
        assert!(Cidr::parse(network).is_ok(), "{network}");
    }
}

#[test]
fn refuses_special_purpose_addresses() {
    let egress = Egress::new(&policy());
    for addr in [
        "0.0.0.0",
        "10.1.2.3",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.31.255.255",
        "192.168.1.1",
        "198.18.0.1",
        "224.0.0.251",
        "255.255.255.255",
        "::",
        "::1",
        "fe80::1",
        "fd00:ec2::254",
        "ff02::1",
        "2001:db8::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "64:ff9b::a00:1",
        "64:ff9b::7f00:1",
    ] {
        assert!(!egress.allows(ip(addr)), "{addr}");
    }
}

#[test]
fn allows_public_addresses() {
    let egress = Egress::new(&policy());
    for addr in [
        "1.1.1.1",
        "93.184.215.14",
        "2606:4700:4700::1111",
        "::ffff:1.1.1.1",
        "64:ff9b::101:101",
    ] {
        assert!(egress.allows(ip(addr)), "{addr}");
    }
}

#[test]
fn refuses_veyras_own_endpoints() {
    let egress = Egress::new(&policy());
    for own in ["5.6.7.8", "2a01:4f8::8", "9.9.8.8", "5.6.7.9", "5.6.7.10"] {
        assert!(!egress.allows(ip(own)), "{own}");
    }
    assert!(egress.allows(ip("5.6.7.11")));
}

#[test]
fn an_allowed_network_is_carved_out() {
    let mut egress = Egress::new(&policy());
    egress.allow(Cidr::parse("127.0.0.1/32").unwrap());
    assert!(egress.allows(ip("127.0.0.1")));
    assert!(!egress.allows(ip("127.0.0.2")));
}
