//! Recover a wallet from a BIP-39 mnemonic or from a bare secret scalar.
//!
//! A mnemonic carries its derivation, so the recovered wallet reports the
//! public key and the phrase alongside the secret. A bare private key carries
//! neither, and both come back empty.

use coins_bip39::{English, Mnemonic};
use serde::{Deserialize, Serialize};

use super::{address, key, public, secret, PATH};
use crate::RunError;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateKeySource {
    pub private_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoverySource {
    Mnemonic(String),
    PrivateKey(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub source: RecoverySource,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveredWallet {
    pub private_key: String,
    pub public_key: Option<String>,
    pub address: String,
    pub mnemonic: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub wallet: RecoveredWallet,
}

pub async fn recover_wallet(input: Input) -> Result<Output, RunError> {
    let wallet = match input.source {
        RecoverySource::Mnemonic(phrase) => {
            let mnemonic = Mnemonic::<English>::new_from_phrase(&phrase)
                .map_err(|e| RunError::SerializeParamsError(format!("invalid mnemonic: {e}")))?;
            let derived = mnemonic
                .derive_key(PATH, None)
                .map_err(|e| RunError::CodeExecutionError(format!("could not derive {PATH}: {e}")))?;
            let derived = derived.as_ref();
            RecoveredWallet {
                private_key: secret(derived),
                public_key: Some(public(derived)),
                address: address(derived),
                mnemonic: Some(mnemonic.to_phrase()),
            }
        }
        RecoverySource::PrivateKey(text) => {
            let derived = key(&text)?;
            RecoveredWallet {
                private_key: secret(&derived),
                public_key: None,
                address: address(&derived),
                mnemonic: None,
            }
        }
    };

    Ok(Output { wallet })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Published vector: the BIP-39 specification's all-zero-entropy mnemonic,
    /// at the default Ethereum path. This address and secret are the values
    /// every BIP-44 implementation is expected to reproduce.
    #[tokio::test]
    async fn test_recover_wallet_bip39_zero_entropy_vector() {
        let result = recover_wallet(Input {
            source: RecoverySource::Mnemonic(
                "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
                    .to_string(),
            ),
        })
        .await
        .unwrap();

        assert_eq!(result.wallet.address, "0x9858EfFD232B4033E47d90003D41EC34EcaEda94");
        assert_eq!(
            result.wallet.private_key,
            "0x1ab42cc412b618bdea3a599e3c9bae199ebf030895b039e9db1e30dafb12b727"
        );
    }

    /// Published vector: the Hardhat / Foundry default development mnemonic.
    /// Account 0 of this phrase is documented in their tooling.
    #[tokio::test]
    async fn test_recover_wallet_hardhat_vector() {
        let result = recover_wallet(Input {
            source: RecoverySource::Mnemonic("test test test test test test test test test test test junk".to_string()),
        })
        .await
        .unwrap();

        assert_eq!(result.wallet.address, "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        assert_eq!(
            result.wallet.private_key,
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        );
        assert!(result.wallet.public_key.is_some());
        assert!(result.wallet.mnemonic.is_some());
    }

    /// The same published account, reached from its secret instead of its
    /// phrase, must land on the same address — and report neither.
    #[tokio::test]
    async fn test_recover_wallet_hardhat_vector_from_private_key() {
        let result = recover_wallet(Input {
            source: RecoverySource::PrivateKey(
                "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string(),
            ),
        })
        .await
        .unwrap();

        assert_eq!(result.wallet.address, "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        assert_eq!(result.wallet.public_key, None);
        assert_eq!(result.wallet.mnemonic, None);
    }

    #[tokio::test]
    async fn test_recover_wallet_from_private_key() {
        // Nothing important, just a random generated wallet
        // privateKey: "0xda1abaf1622435f554d80ba2436dbbfb18a8697ef63c4c26a782baaf82334211",
        // publicKey: "0x03e220eaea3b2006a0bd67a62d44130deaa7b608c976844baedef13ce067fbcec9",
        // address: "0x023251Ef2dF395ed0ad5D3771abfEC23ac40e7cD"

        let input = Input {
            source: RecoverySource::PrivateKey(
                "0xda1abaf1622435f554d80ba2436dbbfb18a8697ef63c4c26a782baaf82334211".to_string(),
            ),
        };
        let result = recover_wallet(input).await.unwrap();
        println!("result: {:?}", result);
        assert_eq!(result.wallet.address, "0x023251Ef2dF395ed0ad5D3771abfEC23ac40e7cD");
        assert_eq!(
            result.wallet.private_key,
            "0xda1abaf1622435f554d80ba2436dbbfb18a8697ef63c4c26a782baaf82334211"
        );
        assert_eq!(result.wallet.public_key, None);
    }

    #[tokio::test]
    async fn test_recover_wallet_from_mnemonic() {
        // Nothing important, just a random generated wallet
        // privateKey: "0x53840710bca86bcc8e331dd3c2483becea1d5dc65731ade8f3276813a1b2ba04",
        // publicKey: "0x024c3c73ac45e1ecb3dfa269d72cba48e5cf012c6936488b2893379c754593612e",
        // address: "0x84310102F55C513EdB2795A5384bC674521AD6f3",
        // mnemonic: "envelope same educate win over stuff ghost fly exercise tissue reform remember"

        let input = Input {
            source: RecoverySource::Mnemonic(
                "envelope same educate win over stuff ghost fly exercise tissue reform remember".to_string(),
            ),
        };
        let result = recover_wallet(input).await.unwrap();
        println!("result: {:?}", result);
        assert_eq!(result.wallet.address, "0x84310102F55C513EdB2795A5384bC674521AD6f3");
        assert_eq!(
            result.wallet.private_key,
            "0x53840710bca86bcc8e331dd3c2483becea1d5dc65731ade8f3276813a1b2ba04"
        );
        assert_eq!(
            result.wallet.public_key,
            Some("0x024c3c73ac45e1ecb3dfa269d72cba48e5cf012c6936488b2893379c754593612e".to_string())
        );
    }

    #[tokio::test]
    async fn test_recover_wallet_rejects_bad_input() {
        assert!(recover_wallet(Input {
            source: RecoverySource::Mnemonic("not actually a mnemonic".to_string()),
        })
        .await
        .is_err());

        // Valid words, wrong checksum.
        assert!(recover_wallet(Input {
            source: RecoverySource::Mnemonic(
                "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon"
                    .to_string()
            ),
        })
        .await
        .is_err());

        assert!(recover_wallet(Input {
            source: RecoverySource::PrivateKey("0xnothex".to_string()),
        })
        .await
        .is_err());

        // Right shape, but zero is not a valid secp256k1 scalar.
        assert!(recover_wallet(Input {
            source: RecoverySource::PrivateKey(format!("0x{}", "00".repeat(32))),
        })
        .await
        .is_err());
    }
}
