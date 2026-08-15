//! Check a caller's payment against the terms for a resource.
//!
//! A payment that does not hold up is an ordinary answer, not an error: the
//! caller gets back the terms it should have met so it can pay and retry.

use serde::{Deserialize, Serialize};

use crate::RunError;

use super::{exact, facilitator, payment_requirements};
use hanzo_messages::schemas::x402_types::{
    FacilitatorConfig, Network, PaymentPayload, PaymentRequirements, Price,
};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub price: Price,
    pub network: Network,
    // 0x... Address
    pub pay_to: String,
    pub payment: Option<String>,
    pub x402_version: u32,
    pub facilitator: FacilitatorConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidOutput {
    pub error: String,
    pub accepts: Vec<PaymentRequirements>,
    pub x402_version: u32,
    pub payer: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidOutput {
    pub decoded_payment: PaymentPayload,
    pub selected_payment_requirements: PaymentRequirements,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    pub invalid: Option<InvalidOutput>,
    pub valid: Option<ValidOutput>,
}

impl Output {
    fn refused(
        error: impl Into<String>,
        accepts: Vec<PaymentRequirements>,
        x402_version: u32,
        payer: Option<String>,
    ) -> Self {
        Output {
            invalid: Some(InvalidOutput {
                error: error.into(),
                accepts,
                x402_version,
                payer,
            }),
            valid: None,
        }
    }
}

pub async fn verify_payment(input: Input) -> Result<Output, RunError> {
    let accepts = payment_requirements::quote(&input)?;
    let version = input.x402_version;

    let Some(header) = input.payment.as_deref() else {
        return Ok(Output::refused("No payment provided", accepts, version, None));
    };

    let mut payment = match exact::decode(header) {
        Ok(payment) => payment,
        Err(error) => {
            return Ok(Output::refused(
                format!("Invalid or malformed payment header - error: {error}"),
                accepts,
                version,
                None,
            ))
        }
    };
    payment.x402_version = version;

    let Some(selected) = exact::matching(&accepts, &payment) else {
        return Ok(Output::refused(
            "Unable to find matching payment requirements",
            accepts,
            version,
            None,
        ));
    };
    let selected = selected.clone();

    let reply = match facilitator::verify(&input.facilitator, &payment, &selected).await {
        Ok(reply) => reply,
        Err(error) => {
            return Ok(Output::refused(
                format!("unhandled error verifying payment - error: {error}"),
                accepts,
                version,
                None,
            ))
        }
    };

    if !facilitator::verdict(&reply, "isValid") {
        return Ok(Output::refused(
            format!("Invalid payment - {}", facilitator::reason(&reply, "invalidReason")),
            accepts,
            version,
            reply.get("payer").and_then(|payer| payer.as_str()).map(str::to_string),
        ));
    }

    Ok(Output {
        invalid: None,
        valid: Some(ValidOutput {
            decoded_payment: payment,
            selected_payment_requirements: selected,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(payment: Option<String>) -> Input {
        Input {
            price: Price::Money(0.001),
            network: Network::BaseSepolia,
            pay_to: "0x9858EfFD232B4033E47d90003D41EC34EcaEda94".to_string(),
            payment,
            x402_version: 1,
            facilitator: FacilitatorConfig::default(),
        }
    }

    /// No payment is answered with the terms, not with an error — this is the
    /// path that turns into an HTTP 402 for the caller.
    #[tokio::test]
    async fn test_verify_payment() {
        let output = verify_payment(input(None)).await.unwrap();
        assert!(output.valid.is_none());
        let invalid = output.invalid.unwrap();
        assert_eq!(invalid.error, "No payment provided");
        assert_eq!(invalid.x402_version, 1);
        assert_eq!(invalid.accepts.first().unwrap().max_amount_required, "1000");
        assert_eq!(invalid.accepts.first().unwrap().network, Network::BaseSepolia);
    }

    #[tokio::test]
    async fn a_header_that_is_not_a_payment_is_refused_with_the_terms() {
        let output = verify_payment(input(Some("!!! not base64 !!!".to_string())))
            .await
            .unwrap();
        assert!(output.valid.is_none());
        let invalid = output.invalid.unwrap();
        assert!(
            invalid.error.starts_with("Invalid or malformed payment header"),
            "{}",
            invalid.error
        );
        assert!(!invalid.accepts.is_empty());
    }

    /// A well-formed payment for another network cannot be matched to these
    /// terms, and must not reach the facilitator.
    #[tokio::test]
    async fn a_payment_for_another_network_finds_no_matching_terms() {
        let elsewhere = super::super::payment_requirements::quote(&Input {
            network: Network::Base,
            ..input(None)
        })
        .unwrap();
        let key = crate::functions::ethers_wallet::key(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        )
        .unwrap();
        let header = exact::encode(&exact::sign(&key, &elsewhere[0], 1).unwrap()).unwrap();

        let output = verify_payment(input(Some(header))).await.unwrap();
        assert!(output.valid.is_none());
        assert_eq!(
            output.invalid.unwrap().error,
            "Unable to find matching payment requirements"
        );
    }

    #[tokio::test]
    async fn an_unpayable_price_is_an_error() {
        assert!(verify_payment(Input {
            price: Price::Money(f64::NAN),
            ..input(None)
        })
        .await
        .is_err());
    }
}
