//! Client admission (spec §9.6): which source networks may use the DNS engine,
//! and which profile or device each one is. This is the static CIDR part;
//! link-IP, DDNS and DoT/DoH tokens arrive in later plans.

use crate::compile::{Issue, IssueCode, Policy, Severity, issue};
use crate::decide::Subject;
use crate::model::BundleSpec;
use std::fmt;
use std::net::IpAddr;

/// An IP network such as `192.168.1.0/24` or `2001:db8::/32`. A bare address is
/// one host. Host bits are cleared, so `192.168.1.5/24` is `192.168.1.0/24`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix_len: u8,
}

impl Cidr {
    pub fn parse(text: &str) -> Result<Cidr, String> {
        let bad = || format!("{text:?} is not an IP network like 192.168.1.0/24");
        let (addr, len) = match text.split_once('/') {
            Some((addr, len)) => (addr, Some(len)),
            None => (text, None),
        };
        let addr: IpAddr = addr.parse().map_err(|_| bad())?;
        let max = if addr.is_ipv4() { 32 } else { 128 };
        let prefix_len = match len {
            None => max,
            Some(len) => len
                .parse::<u8>()
                .ok()
                .filter(|len| *len <= max)
                .ok_or_else(bad)?,
        };
        Ok(Cidr {
            network: mask(addr, prefix_len),
            prefix_len,
        })
    }

    pub fn prefix_len(&self) -> u8 {
        self.prefix_len
    }

    /// IPv4-mapped IPv6 sources (`::ffff:a.b.c.d`), as a dual-stack socket
    /// reports IPv4 clients, match IPv4 networks.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        ip.is_ipv4() == self.network.is_ipv4() && mask(ip, self.prefix_len) == self.network
    }

    fn overlaps(&self, other: &Cidr) -> bool {
        self.contains(other.network) || other.contains(self.network)
    }
}

impl fmt::Display for Cidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix_len)
    }
}

fn mask(ip: IpAddr, prefix_len: u8) -> IpAddr {
    let len = u32::from(prefix_len);
    match ip {
        IpAddr::V4(v4) => {
            let mask = u32::MAX.checked_shl(32 - len).unwrap_or(0);
            IpAddr::V4((u32::from(v4) & mask).into())
        }
        IpAddr::V6(v6) => {
            let mask = u128::MAX.checked_shl(128 - len).unwrap_or(0);
            IpAddr::V6((u128::from(v6) & mask).into())
        }
    }
}

/// An admitted source: who it is, and the `admission` network that said so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    pub subject: Subject,
    pub binding: String,
}

impl Policy {
    /// Who a source address is (spec §9.6): the longest matching `admission`
    /// network wins. `None` means not admitted, and DNS drops the query.
    pub fn admit(&self, ip: IpAddr) -> Option<Admitted> {
        self.bindings
            .iter()
            .filter(|(cidr, _)| cidr.contains(ip))
            .max_by_key(|(cidr, _)| cidr.prefix_len())
            .map(|(cidr, subject)| Admitted {
                subject: subject.clone(),
                binding: cidr.to_string(),
            })
    }
}

pub(crate) fn compile_admission(
    spec: &BundleSpec,
    issues: &mut Vec<Issue>,
) -> Vec<(Cidr, Subject)> {
    use IssueCode::*;
    let error = |code, message: String| issue(Severity::Error, code, &[], message);
    let mut bindings = Vec::new();
    for a in &spec.admission {
        let subject = match (&a.profile, &a.device) {
            (Some(p), None) if spec.profiles.iter().any(|known| &known.id == p) => Subject {
                profile: p.clone(),
                device: None,
            },
            (Some(p), None) => {
                let msg = format!("admission {}: unknown profile {p}", a.cidr);
                issues.push(error(UnknownReference, msg));
                continue;
            }
            (None, Some(d)) => match spec.devices.iter().find(|known| &known.id == d) {
                Some(device) => Subject {
                    profile: device.profile.clone(),
                    device: Some(d.clone()),
                },
                None => {
                    let msg = format!("admission {}: unknown device {d}", a.cidr);
                    issues.push(error(UnknownReference, msg));
                    continue;
                }
            },
            _ => {
                let msg = format!(
                    "admission {}: give exactly one of profile or device",
                    a.cidr
                );
                issues.push(error(InvalidBinding, msg));
                continue;
            }
        };
        match Cidr::parse(&a.cidr) {
            Ok(cidr) => bindings.push((cidr, subject)),
            Err(e) => issues.push(error(InvalidAddress, format!("admission: {e}"))),
        }
    }

    // One source address must map to exactly one profile (spec §17.2).
    for (i, (a, first)) in bindings.iter().enumerate() {
        for (b, second) in &bindings[i + 1..] {
            if a == b {
                issues.push(error(
                    SourceBindingConflict,
                    format!("admission lists {a} twice"),
                ));
            } else if a.overlaps(b) && first.profile != second.profile {
                let msg = format!(
                    "{a} ({}) and {b} ({}) overlap; one source must map to one profile",
                    first.profile, second.profile
                );
                issues.push(error(SourceBindingConflict, msg));
            }
        }
    }
    if spec.dns.is_some() && spec.admission.is_empty() {
        let msg = "dns is configured but admission is empty, so every query is dropped";
        issues.push(issue(Severity::Warning, NoAdmission, &[], msg.into()));
    }
    bindings
}
