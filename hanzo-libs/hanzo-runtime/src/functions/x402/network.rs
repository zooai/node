//! What each x402 network settles in.
//!
//! Every network below settles in USDC, which is a six-decimal EIP-3009 token.
//! The EIP-712 domain `name` and `version` are the values the deployed
//! contracts return from `name()` and `version()`; a domain built from the
//! wrong name yields a signature that looks fine and reverts on settlement.

use hanzo_messages::schemas::x402_types::{ERC20Asset, Network, Price, EIP712};

use crate::RunError;

/// EVM chain id, which becomes the EIP-712 domain's `chainId`.
pub fn chain(network: &Network) -> u64 {
    match network {
        Network::Base => 8453,
        Network::BaseSepolia => 84532,
        Network::Avalanche => 43114,
        Network::AvalancheFuji => 43113,
    }
}

/// The USDC deployment a network settles in by default.
pub fn asset(network: &Network) -> ERC20Asset {
    let (address, name) = match network {
        Network::Base => ("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "USD Coin"),
        Network::BaseSepolia => ("0x036CbD53842c5426634e7929541eC2318f3dCF7e", "USDC"),
        Network::Avalanche => ("0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E", "USD Coin"),
        Network::AvalancheFuji => ("0x5425890298aed601595a70AB815c96711a31Bc65", "USD Coin"),
    };
    ERC20Asset {
        address: address.to_string(),
        decimals: 6,
        eip712: EIP712 {
            name: name.to_string(),
            version: "2".to_string(),
        },
    }
}

/// Resolve a price into the atomic amount to charge and the asset to charge it in.
///
/// A [`Price::Money`] is a US dollar figure settled in the network's default
/// asset. A [`Price::ERC20TokenAmount`] already names both.
pub fn atomic(price: &Price, network: &Network) -> Result<(String, ERC20Asset), RunError> {
    match price {
        Price::Money(usd) => {
            let asset = asset(network);
            Ok((scale(*usd, asset.decimals)?, asset))
        }
        Price::ERC20TokenAmount(token) => Ok((token.amount.clone(), token.asset.clone())),
    }
}

/// Scale a decimal price into whole atomic units.
///
/// The scaling runs through a fixed-point rendering rather than a binary
/// multiply, so a price like $0.0079 becomes exactly `7900` instead of the
/// `7900.000000000001` that `0.0079 * 1e6` produces in floating point.
fn scale(usd: f64, decimals: u32) -> Result<String, RunError> {
    if !usd.is_finite() || usd < 0.0 {
        return Err(RunError::SerializeParamsError(format!(
            "price {usd} is not a payable amount"
        )));
    }
    let rendered = format!("{:.*}", decimals as usize, usd);
    let (whole, fraction) = rendered.split_once('.').unwrap_or((rendered.as_str(), ""));
    let digits = format!("{whole}{fraction}");
    let trimmed = digits.trim_start_matches('0');
    Ok(if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanzo_messages::schemas::x402_types::ERC20TokenAmount;

    #[test]
    fn chain_ids_are_the_published_ones() {
        assert_eq!(chain(&Network::Base), 8453);
        assert_eq!(chain(&Network::BaseSepolia), 84532);
        assert_eq!(chain(&Network::Avalanche), 43114);
        assert_eq!(chain(&Network::AvalancheFuji), 43113);
    }

    #[test]
    fn usdc_deployments_match_circle() {
        // Circle publishes these addresses; the node signs EIP-712 domains
        // bound to them, so a typo here is a silently invalid payment.
        assert_eq!(asset(&Network::Base).address, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
        assert_eq!(
            asset(&Network::BaseSepolia).address,
            "0x036CbD53842c5426634e7929541eC2318f3dCF7e"
        );
        assert_eq!(
            asset(&Network::Avalanche).address,
            "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E"
        );
        assert_eq!(
            asset(&Network::AvalancheFuji).address,
            "0x5425890298aed601595a70AB815c96711a31Bc65"
        );
        for network in [
            Network::Base,
            Network::BaseSepolia,
            Network::Avalanche,
            Network::AvalancheFuji,
        ] {
            assert_eq!(asset(&network).decimals, 6);
            assert_eq!(asset(&network).eip712.version, "2");
        }
    }

    #[test]
    fn domain_names_follow_each_deployment() {
        // Base Sepolia's USDC returns "USDC" from name(); the other three
        // return "USD Coin".
        assert_eq!(asset(&Network::BaseSepolia).eip712.name, "USDC");
        assert_eq!(asset(&Network::Base).eip712.name, "USD Coin");
        assert_eq!(asset(&Network::Avalanche).eip712.name, "USD Coin");
        assert_eq!(asset(&Network::AvalancheFuji).eip712.name, "USD Coin");
    }

    #[test]
    fn dollar_prices_scale_to_whole_atomic_units() {
        let at = |usd: f64| atomic(&Price::Money(usd), &Network::BaseSepolia).unwrap().0;
        assert_eq!(at(0.001), "1000");
        assert_eq!(at(1.0), "1000000");
        assert_eq!(at(0.1), "100000");
        assert_eq!(at(0.0), "0");
        assert_eq!(at(999.999999), "999999999");
    }

    #[test]
    fn scaling_does_not_leak_binary_rounding() {
        // Each of these is a price whose naive `usd * 1e6` is not an integer.
        let at = |usd: f64| atomic(&Price::Money(usd), &Network::BaseSepolia).unwrap().0;
        assert_eq!(at(0.0079), "7900");
        assert_eq!(at(0.0157), "15700");
        assert_eq!(at(0.0314), "31400");
        assert_eq!(at(1.005), "1005000");
        // Sweep the range the reference implementation misprices in.
        for step in 1..20_000u64 {
            let usd = step as f64 / 10_000.0;
            let scaled = at(usd);
            assert!(
                scaled.bytes().all(|b| b.is_ascii_digit()),
                "price {usd} scaled to {scaled}"
            );
            assert_eq!(scaled.parse::<u64>().unwrap(), step * 100, "price {usd}");
        }
    }

    #[test]
    fn token_amounts_pass_through_untouched() {
        let asset = ERC20Asset {
            address: "0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string(),
            decimals: 6,
            eip712: EIP712 {
                name: "USDC".to_string(),
                version: "2".to_string(),
            },
        };
        let price = Price::ERC20TokenAmount(ERC20TokenAmount {
            amount: "12345".to_string(),
            asset: asset.clone(),
        });
        let (amount, resolved) = atomic(&price, &Network::Base).unwrap();
        assert_eq!(amount, "12345");
        // The named asset wins over the network default.
        assert_eq!(resolved.address, asset.address);
    }

    #[test]
    fn refuses_prices_that_cannot_be_charged() {
        assert!(atomic(&Price::Money(-1.0), &Network::Base).is_err());
        assert!(atomic(&Price::Money(f64::NAN), &Network::Base).is_err());
        assert!(atomic(&Price::Money(f64::INFINITY), &Network::Base).is_err());
    }
}
