//! Generate a fresh wallet: a 12-word BIP-39 mnemonic and the key BIP-32
//! derives from it at the default Ethereum path.

use coins_bip39::{English, Mnemonic};
use serde::{Deserialize, Serialize};

use super::{address, public, secret, PATH};
use crate::RunError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedWallet {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
    pub mnemonic: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub wallet: CreatedWallet,
}

pub async fn create_wallet(_input: Input) -> Result<Output, RunError> {
    // 12 words is 128 bits of entropy, drawn from the OS.
    let mnemonic = Mnemonic::<English>::new_with_count(&mut rand::thread_rng(), 12)
        .map_err(|e| RunError::CodeExecutionError(format!("could not generate a mnemonic: {e}")))?;
    let key = mnemonic
        .derive_key(PATH, None)
        .map_err(|e| RunError::CodeExecutionError(format!("could not derive {PATH}: {e}")))?;
    let key = key.as_ref();

    Ok(Output {
        wallet: CreatedWallet {
            private_key: secret(key),
            public_key: public(key),
            address: address(key),
            mnemonic: Some(mnemonic.to_phrase()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::ethers_wallet::recover_wallet::{recover_wallet, RecoverySource};

    #[tokio::test]
    async fn test_create_wallet() {
        let result = create_wallet(Input {}).await.unwrap();

        // Check that wallet fields are present and in expected format
        assert!(!result.wallet.private_key.is_empty());
        assert!(!result.wallet.public_key.is_empty());
        assert!(!result.wallet.address.is_empty());

        // Check that address starts with "0x" and has correct length (42 chars = 0x + 40 hex chars)
        assert!(result.wallet.address.starts_with("0x"));
        assert_eq!(result.wallet.address.len(), 42);

        // Check that private key starts with "0x" and has correct length (66 chars = 0x + 64 hex chars)
        assert!(result.wallet.private_key.starts_with("0x"));
        assert_eq!(result.wallet.private_key.len(), 66);

        // Compressed SEC1 public key: 0x + 33 bytes, leading byte 02 or 03.
        assert_eq!(result.wallet.public_key.len(), 68);
        let tag = &result.wallet.public_key[2..4];
        assert!(tag == "02" || tag == "03", "unexpected public key tag {tag}");

        // Check that mnemonic is present and is a string
        assert!(result.wallet.mnemonic.is_some());
        assert!(!result.wallet.mnemonic.clone().unwrap().is_empty());
        assert_eq!(result.wallet.mnemonic.clone().unwrap().split_whitespace().count(), 12);
    }

    /// The mnemonic must actually reproduce the wallet it was reported with.
    #[tokio::test]
    async fn test_created_mnemonic_recovers_the_same_wallet() {
        let created = create_wallet(Input {}).await.unwrap().wallet;
        let recovered = recover_wallet(super::super::recover_wallet::Input {
            source: RecoverySource::Mnemonic(created.mnemonic.clone().unwrap()),
        })
        .await
        .unwrap()
        .wallet;

        assert_eq!(recovered.address, created.address);
        assert_eq!(recovered.private_key, created.private_key);
        assert_eq!(recovered.public_key, Some(created.public_key));
    }

    /// Two calls must not return the same wallet.
    #[tokio::test]
    async fn test_wallets_are_distinct() {
        let a = create_wallet(Input {}).await.unwrap().wallet;
        let b = create_wallet(Input {}).await.unwrap().wallet;
        assert_ne!(a.address, b.address);
        assert_ne!(a.mnemonic, b.mnemonic);
    }
}
