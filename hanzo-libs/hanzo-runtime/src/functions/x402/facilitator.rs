//! Calls to an x402 facilitator — the service that checks an authorization and,
//! on settlement, submits it on chain.

use std::time::Duration;

use hanzo_messages::schemas::x402_types::{FacilitatorConfig, PaymentPayload, PaymentRequirements};
use serde_json::{json, Value};

use crate::RunError;

/// Facilitators sit behind edge proxies that refuse unidentified clients.
const AGENT: &str = concat!("hanzo-runtime/", env!("CARGO_PKG_VERSION"));

async fn ask(
    config: &FacilitatorConfig,
    route: &str,
    payment: &PaymentPayload,
    requirements: &PaymentRequirements,
) -> Result<Value, RunError> {
    let url = format!("{}/{route}", config.url.trim_end_matches('/'));
    let body = json!({
        "x402Version": payment.x402_version,
        "paymentPayload": payment,
        "paymentRequirements": requirements,
    });

    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(AGENT)
        .build()
        .map_err(|e| RunError::CodeExecutionError(format!("http client: {e}")))?
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| RunError::CodeExecutionError(format!("{route} at {url}: {e}")))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| RunError::CodeExecutionError(format!("{route} at {url}: {e}")))?;
    if !status.is_success() {
        return Err(RunError::CodeExecutionError(format!(
            "{route} at {url}: status {status} - {text}"
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| RunError::ParseOutputError(format!("{route} at {url} answered with {text}: {e}")))
}

/// Ask whether an authorization would settle. A `false` verdict is an answer,
/// not an error, and arrives as `isValid: false` with a reason.
pub async fn verify(
    config: &FacilitatorConfig,
    payment: &PaymentPayload,
    requirements: &PaymentRequirements,
) -> Result<Value, RunError> {
    ask(config, "verify", payment, requirements).await
}

/// Submit an authorization for settlement.
pub async fn settle(
    config: &FacilitatorConfig,
    payment: &PaymentPayload,
    requirements: &PaymentRequirements,
) -> Result<Value, RunError> {
    ask(config, "settle", payment, requirements).await
}

/// A reason string from a facilitator reply. The set of reasons is open —
/// deployed facilitators emit values outside the published list — so it stays
/// text rather than becoming an enum.
pub fn reason(reply: &Value, field: &str) -> String {
    reply
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

/// Whether a reply carries a positive verdict under `field`.
pub fn verdict(reply: &Value, field: &str) -> bool {
    reply.get(field).and_then(Value::as_bool).unwrap_or(false)
}
