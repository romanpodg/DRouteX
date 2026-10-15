//! Where the gateway may connect (spec §10.1, §19.2 item 4): never to loopback,
//! private, link-local, multicast, metadata or other special-purpose networks,
//! nor to Veyra's own endpoints. Every resolved address is checked, so a name
//! that starts resolving to an internal address is refused too (REQ-GW-003).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use veyra_policy::{Cidr, Policy};

/// Networks that are not destinations on the public internet: the IANA IPv4
/// and IPv6 special-purpose address registries, plus multicast.
pub const SPECIAL_PURPOSE: [&str; 27] = [
    "0.0.0.0/8",       // this network
    "10.0.0.0/8",      // private
    "100.64.0.0/10",   // carrier-grade NAT
    "127.0.0.0/8",     // loopback
    "169.254.0.0/16",  // link-local, cloud metadata (169.254.169.254)
    "172.16.0.0/12",   // private
    "192.0.0.0/24",    // IETF protocol assignments
    "192.0.2.0/24",    // documentation
    "192.88.99.0/24",  // 6to4 relay anycast (deprecated)
    "192.168.0.0/16",  // private
    "198.18.0.0/15",   // benchmarking
    "198.51.100.0/24", // documentation
    "203.0.113.0/24",  // documentation
    "224.0.0.0/4",     // multicast
    "240.0.0.0/4",     // reserved, broadcast
    "::/96",           // unspecified, loopback, IPv4-compatible (deprecated)
    "64:ff9b:1::/48",  // local-use NAT64
    "100::/64",        // discard-only
    "2001::/23",       // IETF protocol assignments (Teredo, benchmarking, ORCHID)
    "2001:db8::/32",   // documentation
    "2002::/16",       // 6to4 (deprecated)
    "3fff::/20",       // documentation
    "5f00::/16",       // segment routing (SRv6) SIDs
    "fc00::/7",        // unique local, cloud metadata (fd00:ec2::254)
    "fe80::/10",       // link-local
    "fec0::/10",       // site-local (deprecated)
    "ff00::/8",        // multicast
];

/// The well-known NAT64 prefix (RFC 6052): judged by the IPv4 address inside.
const NAT64: Ipv6Addr = Ipv6Addr::new(0x64, 0xff9b, 0, 0, 0, 0, 0, 0);

/// The addresses the gateway refuses to dial.
#[derive(Debug, Clone)]
pub struct Egress {
    deny: Vec<Cidr>,
    own: Vec<IpAddr>,
    allow: Vec<Cidr>,
}

impl Egress {
    /// Special-purpose networks, every node's gateway addresses, and the
    /// addresses this data plane listens on.
    pub fn new(policy: &Policy) -> Egress {
        let gateways = policy
            .gateways()
            .values()
            .flat_map(|g| [Some(IpAddr::V4(g.ipv4)), g.ipv6.map(IpAddr::V6)])
            .flatten();
        let dns = policy
            .dns()
            .map(|d| d.listen.as_slice())
            .unwrap_or_default();
        let (tls, http) = policy
            .gateway_config()
            .map(|g| (g.tls.as_slice(), g.http.as_slice()))
            .unwrap_or_default();
        let listeners = dns
            .iter()
            .chain(tls)
            .chain(http)
            .map(|a| a.ip())
            .filter(|ip| !ip.is_unspecified());
        Egress {
            deny: SPECIAL_PURPOSE
                .iter()
                .filter_map(|n| Cidr::parse(n).ok())
                .collect(),
            own: gateways
                .chain(listeners)
                .map(|ip| ip.to_canonical())
                .collect(),
            allow: Vec::new(),
        }
    }

    /// Let the gateway dial `network` after all, such as a test origin on loopback.
    pub fn allow(&mut self, network: Cidr) {
        self.allow.push(network);
    }

    /// May the gateway connect to `ip`?
    pub fn allows(&self, ip: IpAddr) -> bool {
        let refused = |ip: IpAddr| {
            (self.deny.iter().any(|n| n.contains(ip)) || self.own.contains(&ip))
                && !self.allow.iter().any(|n| n.contains(ip))
        };
        let ip = ip.to_canonical();
        !refused(ip) && nat64_v4(ip).is_none_or(|v4| !refused(v4))
    }
}

/// The IPv4 address inside a well-known-prefix NAT64 address (RFC 6052 §2.1).
fn nat64_v4(ip: IpAddr) -> Option<IpAddr> {
    let IpAddr::V6(v6) = ip else {
        return None;
    };
    let bits = u128::from(v6);
    (bits >> 32 == u128::from(NAT64) >> 32).then(|| IpAddr::V4(Ipv4Addr::from(bits as u32)))
}
