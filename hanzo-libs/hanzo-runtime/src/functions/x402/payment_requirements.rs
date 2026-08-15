//! What a payee asks for: the terms a caller must satisfy to be served.

use hanzo_messages::schemas::x402_types::PaymentRequirements;
use serde::Deserialize;
use serde_json::json;

use super::{network, verify_payment::Input};
use crate::RunError;

pub type PaymentRequirementsInput = Input;

/// The resource these terms are quoted against.
pub const RESOURCE: &str = "https://hanzo.ai";

/// How long a payer has to present an authorization.
pub const TIMEOUT: u64 = 300;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequirementsOutput {
    pub payment_requirements: Vec<PaymentRequirements>,
}

/// Quote the terms for a price on a network.
///
/// `extra` carries the asset's EIP-712 domain name and version, which the
/// payer needs to sign an authorization the asset contract will honour.
pub fn quote(input: &PaymentRequirementsInput) -> Result<Vec<PaymentRequirements>, RunError> {
    let (max_amount_required, asset) = network::atomic(&input.price, &input.network)?;
    Ok(vec![PaymentRequirements {
        scheme: "exact".to_string(),
        network: input.network.clone(),
        max_amount_required,
        resource: RESOURCE.to_string(),
        description: String::new(),
        mime_type: String::new(),
        pay_to: input.pay_to.clone(),
        max_timeout_seconds: TIMEOUT,
        asset: asset.address,
        output_schema: Some(json!({})),
        extra: Some(json!({ "name": asset.eip712.name, "version": asset.eip712.version })),
    }])
}

pub async fn get_payment_requirements(input: PaymentRequirementsInput) -> Result<PaymentRequirementsOutput, RunError> {
    Ok(PaymentRequirementsOutput {
        payment_requirements: quote(&input)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanzo_messages::schemas::x402_types::{FacilitatorConfig, Network, Price};

    fn input(price: Price, network: Network) -> PaymentRequirementsInput {
        PaymentRequirementsInput {
            price,
            network,
            pay_to: "0x9858EfFD232B4033E47d90003D41EC34EcaEda94".to_string(),
            payment: None,
            x402_version: 1,
            facilitator: FacilitatorConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_payment_requirements() {
        let price_in_raw_usd = 0.001;
        let output = get_payment_requirements(input(Price::Money(price_in_raw_usd), Network::BaseSepolia))
            .await
            .unwrap();
        assert!(!output.payment_requirements.is_empty());

        let requirements = &output.payment_requirements[0];
        assert_eq!(requirements.max_amount_required, "1000");
        assert_eq!(requirements.scheme, "exact");
        assert_eq!(requirements.network, Network::BaseSepolia);
        assert_eq!(requirements.pay_to, "0x9858EfFD232B4033E47d90003D41EC34EcaEda94");
        assert_eq!(requirements.max_timeout_seconds, TIMEOUT);
        assert_eq!(requirements.resource, RESOURCE);
        assert_eq!(requirements.asset, "0x036CbD53842c5426634e7929541eC2318f3dCF7e");
    }

    #[tokio::test]
    async fn quotes_carry_the_domain_a_payer_must_sign_under() {
        let output = get_payment_requirements(input(Price::Money(1.0), Network::Base))
            .await
            .unwrap();
        let extra = output.payment_requirements[0].extra.clone().unwrap();
        assert_eq!(extra["name"], "USD Coin");
        assert_eq!(extra["version"], "2");
    }

    #[tokio::test]
    async fn a_rejected_price_is_an_error_not_a_quote() {
        assert!(get_payment_requirements(input(Price::Money(-1.0), Network::Base))
            .await
            .is_err());
    }
}
