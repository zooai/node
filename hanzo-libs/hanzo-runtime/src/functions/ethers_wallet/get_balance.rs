//! Read a wallet's balance over JSON-RPC: an ERC-20 balance when a token
//! address is given, the chain's native balance otherwise.

use std::time::Duration;

use alloy_primitives::Address;
use alloy_sol_types::SolCall;
use serde::{Deserialize, Serialize};

use crate::{rpc::Rpc, units, RunError};

alloy_sol_types::sol! {
    function balanceOf(address owner) external view returns (uint256);
    function decimals() external view returns (uint8);
    function symbol() external view returns (string);
    function name() external view returns (string);
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub token_address: Option<String>,
    pub wallet_address: String,
    pub rpc_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub balance: String,
    pub formatted_balance: String,
    pub token_info: TokenInfo,
}

fn parse(text: &str, what: &str) -> Result<Address, RunError> {
    text.parse::<Address>()
        .map_err(|e| RunError::SerializeParamsError(format!("{what} is not an address: {e}")))
}

/// Decode a call's ABI-encoded return value, naming the call in any error.
fn returns<C: SolCall>(data: &[u8], call: &str) -> Result<C::Return, RunError> {
    C::abi_decode_returns(data).map_err(|e| RunError::ParseOutputError(format!("{call} returned unexpected data: {e}")))
}

pub async fn get_balance(input: Input) -> Result<Output, RunError> {
    let wallet = parse(&input.wallet_address, "wallet address")?;
    let rpc = Rpc::new(&input.rpc_url, Duration::from_secs(30))?;

    let (balance, token_info) = match &input.token_address {
        Some(token) => {
            let token = parse(token, "token address")?;
            let (of, dec, sym, nam) = (
                balanceOfCall { owner: wallet }.abi_encode(),
                decimalsCall {}.abi_encode(),
                symbolCall {}.abi_encode(),
                nameCall {}.abi_encode(),
            );
            // The four reads are independent; issue them together.
            let (balance, decimals, symbol, name) = tokio::try_join!(
                rpc.call(token, &of),
                rpc.call(token, &dec),
                rpc.call(token, &sym),
                rpc.call(token, &nam),
            )?;
            (
                returns::<balanceOfCall>(&balance, "balanceOf")?,
                TokenInfo {
                    name: returns::<nameCall>(&name, "name")?,
                    symbol: returns::<symbolCall>(&symbol, "symbol")?,
                    decimals: returns::<decimalsCall>(&decimals, "decimals")?,
                },
            )
        }
        None => (
            rpc.balance(wallet).await?,
            TokenInfo {
                name: "Ether".to_string(),
                symbol: "ETH".to_string(),
                decimals: 18,
            },
        ),
    };

    Ok(Output {
        balance: balance.to_string(),
        formatted_balance: units::format(balance, token_info.decimals),
        token_info,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;
    use alloy_sol_types::SolValue;

    #[test]
    fn erc20_selectors_match_the_published_signatures() {
        // The four ERC-20 selectors are fixed, published values; if the ABI
        // encoding drifted, every read above would silently address the wrong
        // function.
        assert_eq!(hex::encode(balanceOfCall::SELECTOR), "70a08231");
        assert_eq!(hex::encode(decimalsCall::SELECTOR), "313ce567");
        assert_eq!(hex::encode(symbolCall::SELECTOR), "95d89b41");
        assert_eq!(hex::encode(nameCall::SELECTOR), "06fdde03");
    }

    #[test]
    fn balance_call_encodes_the_address_argument() {
        let owner: Address = "0x0000000000000000000000000000000000000000".parse().unwrap();
        let encoded = balanceOfCall { owner }.abi_encode();
        // selector + one 32-byte word
        assert_eq!(encoded.len(), 36);
        assert_eq!(hex::encode(&encoded[..4]), "70a08231");
    }

    #[test]
    fn decodes_an_erc20_reply() {
        // A `decimals()` reply of 6, and a `balanceOf` reply of 1_500_000,
        // both left-padded into a 32-byte word as the ABI requires.
        let mut word = [0u8; 32];
        word[31] = 6;
        assert_eq!(returns::<decimalsCall>(&word, "decimals").unwrap(), 6u8);

        let balance = U256::from(1_500_000u64);
        let encoded = balance.abi_encode();
        assert_eq!(returns::<balanceOfCall>(&encoded, "balanceOf").unwrap(), balance);
        assert_eq!(units::format(balance, 6), "1.5");
    }

    #[test]
    fn rejects_a_malformed_address() {
        assert!(parse("not-an-address", "wallet address").is_err());
    }

    #[tokio::test]
    #[ignore = "Requires a live Base Sepolia RPC endpoint"]
    async fn test_get_token_balance() {
        // Using real USDC contract address on Base Sepolia
        let input = Input {
            token_address: Some("0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string()),
            wallet_address: "0x0000000000000000000000000000000000000000".to_string(), // Burn address
            rpc_url: "https://sepolia.base.org".to_string(),
        };

        let res = get_balance(input).await.unwrap();
        assert_eq!(res.token_info.symbol, "USDC");
        assert_eq!(res.token_info.decimals, 6);
        // Burn address should have 0 balance
        assert_eq!(res.balance, "0");
        assert_eq!(res.formatted_balance, "0.0");
    }
}
