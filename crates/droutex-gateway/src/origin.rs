//! Conservative screening of one supplied resolution and self-address inventory.
//! These pure comparisons do not establish DNS provenance, freshness, current
//! authorization or permission to connect. The network adapter must still obtain
//! trusted fresh input and validate and pin the actual address at final connect.
use std::{collections::BTreeSet, error::Error, fmt, net::IpAddr};
pub mod inventory;
mod registry;
pub mod resolver;
pub mod response;

pub const MAX_ANSWER_ADDRESSES: usize = 32;
pub const MAX_SELF_ADDRESSES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScreeningError {
    EmptyAnswer,
    TooManyAddresses,
    UnsafeAddress,
    SelfAddress,
}
impl fmt::Display for ScreeningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "origin address screening {self:?}")
    }
}
impl Error for ScreeningError {}

/// Immutable numeric candidates that passed *only* this static comparison.
/// Not a live connection permit or evidence that its inputs are trustworthy.
#[derive(Debug, Eq, PartialEq)]
pub struct ScreenedAddresses(Vec<IpAddr>);
impl ScreenedAddresses {
    pub fn addresses(&self) -> &[IpAddr] {
        &self.0
    }
}

/// Reject the entire answer if any candidate is unsafe or matches the supplied
/// self inventory. Never silently select a public entry from a mixed unsafe answer.
pub fn screen_resolution(
    answers: &[IpAddr],
    self_addresses: &[IpAddr],
) -> Result<ScreenedAddresses, ScreeningError> {
    if answers.is_empty() {
        return Err(ScreeningError::EmptyAnswer);
    }
    if answers.len() > MAX_ANSWER_ADDRESSES || self_addresses.len() > MAX_SELF_ADDRESSES {
        return Err(ScreeningError::TooManyAddresses);
    }
    let own: BTreeSet<_> = self_addresses
        .iter()
        .copied()
        .map(normalize_mapped)
        .collect();
    let mut seen = BTreeSet::new();
    let mut checked = Vec::with_capacity(answers.len());
    for &address in answers {
        if !droutex_config::upstream::public_unicast_address(address) {
            return Err(ScreeningError::UnsafeAddress);
        }
        if own.contains(&normalize_mapped(address)) {
            return Err(ScreeningError::SelfAddress);
        }
        if seen.insert(address) {
            checked.push(address);
        }
    }
    Ok(ScreenedAddresses(checked))
}

fn normalize_mapped(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map_or(address, IpAddr::V4),
        _ => address,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    #[test]
    fn mixed_unsafe_self_and_mapped_answers_fail_closed() {
        for bad in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "224.0.0.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "::ffff:1.1.1.1",
        ] {
            assert_eq!(
                screen_resolution(&[ip("1.1.1.1"), ip(bad)], &[]),
                Err(ScreeningError::UnsafeAddress),
                "{bad}"
            );
        }
        assert_eq!(
            screen_resolution(&[ip("1.1.1.1")], &[ip("::ffff:1.1.1.1")]),
            Err(ScreeningError::SelfAddress)
        );
        assert_eq!(
            screen_resolution(&[ip("2606:4700:4700::1111")], &[ip("2606:4700:4700::1111")]),
            Err(ScreeningError::SelfAddress)
        );
        assert_eq!(
            screen_resolution(&[], &[]),
            Err(ScreeningError::EmptyAnswer)
        );
    }

    #[test]
    fn bounded_answers_retain_only_unique_supplied_numeric_addresses() {
        let screened = screen_resolution(
            &[ip("1.1.1.1"), ip("2606:4700:4700::1111"), ip("1.1.1.1")],
            &[],
        )
        .unwrap();
        assert_eq!(
            screened.addresses(),
            &[ip("1.1.1.1"), ip("2606:4700:4700::1111")]
        );
        assert_eq!(
            screen_resolution(&vec![ip("1.1.1.1"); MAX_ANSWER_ADDRESSES + 1], &[]),
            Err(ScreeningError::TooManyAddresses)
        );
        assert_eq!(
            screen_resolution(
                &[ip("1.1.1.1")],
                &vec![ip("8.8.8.8"); MAX_SELF_ADDRESSES + 1]
            ),
            Err(ScreeningError::TooManyAddresses)
        );
    }
}
