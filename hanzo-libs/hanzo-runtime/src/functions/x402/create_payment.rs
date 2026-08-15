//! Pay for a resource: sign an authorization for one of the offered terms and
//! encode it for the payment header.

use hanzo_messages::schemas::x402_types::PaymentRequirements;
use serde::{Deserialize, Serialize};

use super::exact;
use crate::functions::ethers_wallet;
use crate::RunError;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub accepts: Vec<PaymentRequirements>,
    pub x402_version: u32,
    // Atm it just support signer wallet
    pub private_key: String,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    pub payment: String,
}

pub async fn create_payment(input: Input) -> Result<Output, RunError> {
    let key = ethers_wallet::key(&input.private_key)?;

    // `exact` is the scheme this node pays under. Nothing here can honour a
    // different one, so say so rather than signing an authorization the payee
    // will not recognise.
    let chosen = input
        .accepts
        .iter()
        .find(|requirements| requirements.scheme == "exact")
        .ok_or_else(|| {
            RunError::SerializeParamsError(format!(
                "none of the {} accepted terms use the exact scheme",
                input.accepts.len()
            ))
        })?;

    Ok(Output {
        payment: exact::encode(&exact::sign(&key, chosen, input.x402_version)?)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::x402::exact;
    use hanzo_messages::schemas::x402_types::Network;
    use serde_json::json;

    const KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    const PAYER: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

    fn terms(scheme: &str, network: Network) -> PaymentRequirements {
        PaymentRequirements {
            scheme: scheme.to_string(),
            description: "Test payment".to_string(),
            network,
            max_amount_required: "1000".to_string(),
            resource: "https://hanzo.ai".to_string(),
            mime_type: String::new(),
            pay_to: "0x9858EfFD232B4033E47d90003D41EC34EcaEda94".to_string(),
            max_timeout_seconds: 300,
            asset: "0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string(),
            output_schema: Some(json!({})),
            extra: Some(json!({ "name": "USDC", "version": "2" })),
        }
    }

    #[tokio::test]
    async fn test_create_payment() {
        let requirements = terms("exact", Network::BaseSepolia);
        let output = create_payment(Input {
            accepts: vec![requirements.clone()],
            x402_version: 1,
            private_key: KEY.to_string(),
        })
        .await
        .unwrap();
        assert!(!output.payment.is_empty());

        // The header must decode to an authorization signed by the key we
        // handed in, for the terms we handed in.
        let payment = exact::decode(&output.payment).unwrap();
        assert_eq!(payment.payload.authorization.from, PAYER);
        assert_eq!(payment.payload.authorization.to, requirements.pay_to);
        assert_eq!(payment.payload.authorization.value, "1000");
        assert_eq!(payment.x402_version, 1);
        assert_eq!(
            exact::payer(&payment, &requirements).unwrap().to_checksum(None),
            PAYER
        );
    }

    #[tokio::test]
    async fn picks_the_exact_terms_out_of_the_offer() {
        let output = create_payment(Input {
            accepts: vec![terms("upto", Network::Base), terms("exact", Network::BaseSepolia)],
            x402_version: 1,
            private_key: KEY.to_string(),
        })
        .await
        .unwrap();
        assert_eq!(exact::decode(&output.payment).unwrap().network, Network::BaseSepolia);
    }

    #[tokio::test]
    async fn refuses_terms_it_cannot_honour() {
        let refused = create_payment(Input {
            accepts: vec![terms("upto", Network::Base)],
            x402_version: 1,
            private_key: KEY.to_string(),
        })
        .await;
        assert!(refused.is_err(), "a scheme we cannot sign must not produce a payment");

        assert!(create_payment(Input {
            accepts: vec![],
            x402_version: 1,
            private_key: KEY.to_string(),
        })
        .await
        .is_err());

        assert!(create_payment(Input {
            accepts: vec![terms("exact", Network::BaseSepolia)],
            x402_version: 1,
            private_key: "not-a-key".to_string(),
        })
        .await
        .is_err());
    }
}
