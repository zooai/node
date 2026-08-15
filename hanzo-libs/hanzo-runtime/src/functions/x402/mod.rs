//! x402: paying for a resource over HTTP.
//!
//! A payee quotes terms ([`payment_requirements`]), a payer signs an
//! authorization against them ([`create_payment`]), the payee checks it
//! ([`verify_payment`]) and then collects ([`settle_payment`]).

pub mod create_payment;
pub mod payment_requirements;
pub mod settle_payment;
pub mod verify_payment;

mod exact;
mod facilitator;
mod network;
