//! Veyra gateway (spec §10): TCP 443 routed by SNI and 80 by `Host`, passed
//! through to the origin without terminating TLS. Decisions come from
//! `veyra-policy`; this crate owns the sockets and the protocol sniffing.

mod sniff;

pub use sniff::{MAX_HEAD, Service, Sniff, sniff};
