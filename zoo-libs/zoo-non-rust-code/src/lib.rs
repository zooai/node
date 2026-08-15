//! Zoo's name for the node's wallet, x402 payment and document functions.
//!
//! These functions have one implementation, in `hanzo-runtime`, and this crate
//! is the Zoo-facing name for it. `zoo_non_rust_code::functions::…` and
//! `hanzo_runtime::functions::…` are the same items.

pub use hanzo_runtime::{functions, RunError};
