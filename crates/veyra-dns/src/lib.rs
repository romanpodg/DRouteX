//! Veyra DNS engine (spec §9): admission, answer synthesis and forwarding.
//! Policy decisions come from `veyra-policy`; this crate owns the wire format.

mod cache;
mod engine;
mod synth;
mod upstream;

pub use engine::{Engine, Transport};
