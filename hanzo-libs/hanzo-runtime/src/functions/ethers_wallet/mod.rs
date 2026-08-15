pub mod create_wallet;
pub mod get_balance;
pub mod recover_wallet;

use alloy_primitives::Address;
use k256::ecdsa::SigningKey;

use crate::RunError;

/// BIP-44 account 0, external chain, first index, on SLIP-44 coin type 60 —
/// the path Ethereum wallets derive by default.
pub(crate) const PATH: &str = "m/44'/60'/0'/0/0";

/// `0x`-prefixed 32-byte secret scalar.
pub(crate) fn secret(key: &SigningKey) -> String {
    format!("0x{}", hex::encode(key.to_bytes()))
}

/// `0x`-prefixed compressed SEC1 public key (33 bytes).
pub(crate) fn public(key: &SigningKey) -> String {
    format!("0x{}", hex::encode(key.verifying_key().to_encoded_point(true).as_bytes()))
}

/// EIP-55 checksummed address.
pub(crate) fn address(key: &SigningKey) -> String {
    Address::from_private_key(key).to_checksum(None)
}

/// Read a secret scalar from hex, with or without the `0x` prefix.
pub(crate) fn key(text: &str) -> Result<SigningKey, RunError> {
    let bytes = hex::decode(text.strip_prefix("0x").unwrap_or(text))
        .map_err(|e| RunError::SerializeParamsError(format!("private key is not hex: {e}")))?;
    SigningKey::from_slice(&bytes)
        .map_err(|e| RunError::SerializeParamsError(format!("private key is not a valid secp256k1 scalar: {e}")))
}
