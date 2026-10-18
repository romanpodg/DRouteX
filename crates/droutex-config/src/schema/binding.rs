use super::{ConfigError, ConfigErrorCode as Code, model::*, validation::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

pub(super) fn validate_bindings(listeners: &[Listener]) -> Result<(), ConfigError> {
    let mut sockets = BTreeMap::<(bool, u16), Addresses>::new();
    for (i, listener) in listeners.iter().enumerate() {
        named(&listener.name, &field("listeners", i, "name"))?;
        require(
            listener.bind.port() != 0,
            Code::InvalidValue,
            &field("listeners", i, "bind"),
        )?;
        require(
            !listener.dual_stack
                || matches!(listener.bind, SocketAddr::V6(a) if a.ip().is_unspecified() && a.scope_id() == 0),
            Code::InvalidScope,
            &field("listeners", i, "dualStack"),
        )?;
        require(
            !matches!(listener.bind.ip(), IpAddr::V6(ip) if ip.to_ipv4_mapped().is_some()),
            Code::InvalidScope,
            &field("listeners", i, "bind"),
        )?;
        if listener.enabled {
            let key = (
                listener.transport == ListenerTransport::DnsUdp,
                listener.bind.port(),
            );
            require(
                sockets.entry(key).or_default().insert(listener),
                Code::BindingConflict,
                &field("listeners", i, "bind"),
            )?;
        }
    }
    Ok(())
}

#[derive(Default)]
struct Addresses {
    ipv4: BTreeSet<Ipv4Addr>,
    ipv6: BTreeMap<Ipv6Addr, BTreeSet<u32>>,
    ipv4_wildcard: bool,
    ipv6_wildcard: bool,
    dual_stack: bool,
}

impl Addresses {
    fn insert(&mut self, listener: &Listener) -> bool {
        match listener.bind {
            SocketAddr::V4(address) => self.insert_v4(*address.ip()),
            SocketAddr::V6(address) => {
                self.insert_v6(*address.ip(), address.scope_id(), listener.dual_stack)
            }
        }
    }

    fn insert_v4(&mut self, ip: Ipv4Addr) -> bool {
        if self.dual_stack || self.ipv4_wildcard || (ip.is_unspecified() && !self.ipv4.is_empty()) {
            return false;
        }
        self.ipv4_wildcard = ip.is_unspecified();
        self.ipv4.insert(ip)
    }

    fn insert_v6(&mut self, ip: Ipv6Addr, scope: u32, dual_stack: bool) -> bool {
        if self.ipv6_wildcard
            || (ip.is_unspecified() && !self.ipv6.is_empty())
            || (dual_stack && !self.ipv4.is_empty())
        {
            return false;
        }
        let scopes = self.ipv6.entry(ip).or_default();
        if scopes.contains(&0) || (scope == 0 && !scopes.is_empty()) {
            return false;
        }
        self.ipv6_wildcard = ip.is_unspecified();
        self.dual_stack = dual_stack;
        scopes.insert(scope)
    }
}
