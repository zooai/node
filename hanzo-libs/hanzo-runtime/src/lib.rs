//! Wallet, x402 payment and document parsing functions for the Hanzo node.
//!
//! Every function here is implemented natively in Rust against its published
//! standard: BIP-39/BIP-32 and EIP-55 for key material, EIP-712 and EIP-3009
//! for payment authorizations, the x402 protocol for payment negotiation, and
//! the OOXML/PDF container formats for document text.

pub mod functions;
mod rpc;
mod units;

/// How a function failed.
///
/// The variants predate the native implementations and are kept as-is so that
/// callers matching on them do not change:
///
/// - [`RunError::SerializeConfigurationsError`] — a configuration value was
///   malformed (an unparseable contract ABI, an unknown network).
/// - [`RunError::SerializeParamsError`] — an argument was malformed (a bad
///   private key, an address that is not valid hex).
/// - [`RunError::CodeExecutionError`] — the work itself failed (an RPC or HTTP
///   error, an unreadable file, a signing failure).
/// - [`RunError::ParseOutputError`] — a response could not be decoded (ABI
///   output that does not match the declared type, unexpected JSON).
#[derive(Debug)]
pub enum RunError {
    CodeExecutionError(String),
    SerializeConfigurationsError(String),
    SerializeParamsError(String),
    ParseOutputError(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::CodeExecutionError(err) => write!(f, "code execution error: {}", err),
            RunError::SerializeConfigurationsError(err) => write!(f, "failed to serialize configurations: {}", err),
            RunError::SerializeParamsError(err) => write!(f, "failed to serialize parameters: {}", err),
            RunError::ParseOutputError(err) => write!(f, "failed to parse output: {}", err),
        }
    }
}

impl std::error::Error for RunError {}
