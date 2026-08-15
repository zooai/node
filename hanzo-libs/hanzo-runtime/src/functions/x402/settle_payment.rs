//! Collect a verified payment: hand the authorization to a facilitator, which
//! submits it on chain.

use serde::{Deserialize, Serialize};

use crate::RunError;

use super::{exact, facilitator};
use hanzo_messages::schemas::x402_types::{FacilitatorConfig, PaymentPayload, PaymentRequirements};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub payment: PaymentPayload,
    pub accepts: Vec<PaymentRequirements>,
    pub facilitator: FacilitatorConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidOutput {
    pub error: String,
    pub accepts: Vec<PaymentRequirements>,
    pub x402_version: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidOutput {
    pub payment_response: String,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    pub invalid: Option<InvalidOutput>,
    pub valid: Option<ValidOutput>,
}

impl Output {
    fn refused(error: impl Into<String>, accepts: Vec<PaymentRequirements>, x402_version: u32) -> Self {
        Output {
            invalid: Some(InvalidOutput {
                error: error.into(),
                accepts,
                x402_version,
            }),
            valid: None,
        }
    }
}

pub async fn settle_payment(input: Input) -> Result<Output, RunError> {
    let version = input.payment.x402_version;

    let Some(selected) = exact::matching(&input.accepts, &input.payment) else {
        return Ok(Output::refused(
            "Unable to find matching payment requirements",
            input.accepts,
            version,
        ));
    };

    let reply = match facilitator::settle(&input.facilitator, &input.payment, selected).await {
        Ok(reply) => reply,
        Err(error) => {
            return Ok(Output::refused(
                format!("Failed to settle payment - error: {error}"),
                input.accepts,
                version,
            ))
        }
    };

    if !facilitator::verdict(&reply, "success") {
        return Ok(Output::refused(
            format!(
                "Failed to settle payment - error: {}",
                facilitator::reason(&reply, "errorReason")
            ),
            input.accepts,
            version,
        ));
    }

    // The receipt travels back to the caller the same way a payment travels in.
    Ok(Output {
        invalid: None,
        valid: Some(ValidOutput {
            payment_response: exact::receipt(&reply)?,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::ethers_wallet;
    use crate::functions::x402::payment_requirements::quote;
    use crate::functions::x402::verify_payment;
    use hanzo_messages::schemas::x402_types::{Network, Price};

    fn terms(network: Network) -> Vec<PaymentRequirements> {
        quote(&verify_payment::Input {
            price: Price::Money(0.1),
            network,
            pay_to: "0x9858EfFD232B4033E47d90003D41EC34EcaEda94".to_string(),
            payment: None,
            x402_version: 1,
            facilitator: FacilitatorConfig::default(),
        })
        .unwrap()
    }

    fn payment(network: Network) -> PaymentPayload {
        let key =
            ethers_wallet::key("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80").unwrap();
        exact::sign(&key, &terms(network)[0], 1).unwrap()
    }

    /// A payment nobody offered terms for is refused without contacting a
    /// facilitator.
    #[tokio::test]
    async fn a_payment_with_no_matching_terms_is_refused() {
        let output = settle_payment(Input {
            payment: payment(Network::Base),
            accepts: terms(Network::BaseSepolia),
            facilitator: FacilitatorConfig::default(),
        })
        .await
        .unwrap();

        assert!(output.valid.is_none());
        let invalid = output.invalid.unwrap();
        assert_eq!(invalid.error, "Unable to find matching payment requirements");
        assert_eq!(invalid.x402_version, 1);
        assert!(!invalid.accepts.is_empty());
    }

    /// An unreachable facilitator is reported as a refusal carrying the terms,
    /// never as a settled payment.
    #[tokio::test]
    async fn an_unreachable_facilitator_never_reads_as_settled() {
        let output = settle_payment(Input {
            payment: payment(Network::BaseSepolia),
            accepts: terms(Network::BaseSepolia),
            facilitator: FacilitatorConfig {
                // Reserved for documentation, so it cannot answer.
                url: "http://192.0.2.1:9".to_string(),
            },
        })
        .await
        .unwrap();

        assert!(output.valid.is_none());
        assert!(
            output.invalid.unwrap().error.starts_with("Failed to settle payment"),
            "an unreachable facilitator must refuse, not settle"
        );
    }
}
