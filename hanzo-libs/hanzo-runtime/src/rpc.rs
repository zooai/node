//! A minimal Ethereum JSON-RPC 2.0 caller.
//!
//! Only the two reads these functions need: `eth_call` and `eth_getBalance`,
//! both against the latest block.

use std::time::Duration;

use alloy_primitives::{Address, U256};
use serde_json::{json, Value};

use crate::RunError;

pub struct Rpc {
    url: String,
    client: reqwest::Client,
}

impl Rpc {
    pub fn new(url: impl Into<String>, timeout: Duration) -> Result<Self, RunError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| RunError::CodeExecutionError(format!("http client: {e}")))?;
        Ok(Self {
            url: url.into(),
            client,
        })
    }

    async fn send(&self, method: &str, params: Value) -> Result<Value, RunError> {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let response: Value = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| RunError::CodeExecutionError(format!("{method} to {}: {e}", self.url)))?
            .json()
            .await
            .map_err(|e| RunError::ParseOutputError(format!("{method} response from {}: {e}", self.url)))?;

        if let Some(error) = response.get("error") {
            return Err(RunError::CodeExecutionError(format!(
                "{method} to {}: {error}",
                self.url
            )));
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| RunError::ParseOutputError(format!("{method} response has no result")))
    }

    /// Read a hex-quantity or hex-data result into bytes.
    fn bytes(value: &Value, method: &str) -> Result<Vec<u8>, RunError> {
        let text = value
            .as_str()
            .ok_or_else(|| RunError::ParseOutputError(format!("{method} result is not a string")))?;
        let digits = text.strip_prefix("0x").unwrap_or(text);
        // Quantities are minimally encoded and may have an odd digit count.
        let padded = if digits.len() % 2 == 1 {
            format!("0{digits}")
        } else {
            digits.to_string()
        };
        hex::decode(&padded).map_err(|e| RunError::ParseOutputError(format!("{method} result is not hex: {e}")))
    }

    /// `eth_call` at the latest block, returning the raw ABI-encoded return data.
    pub async fn call(&self, to: Address, data: &[u8]) -> Result<Vec<u8>, RunError> {
        let result = self
            .send(
                "eth_call",
                json!([{ "to": to.to_string(), "data": format!("0x{}", hex::encode(data)) }, "latest"]),
            )
            .await?;
        Self::bytes(&result, "eth_call")
    }

    /// `eth_getBalance` at the latest block.
    pub async fn balance(&self, who: Address) -> Result<U256, RunError> {
        let result = self.send("eth_getBalance", json!([who.to_string(), "latest"])).await?;
        let bytes = Self::bytes(&result, "eth_getBalance")?;
        if bytes.len() > 32 {
            return Err(RunError::ParseOutputError("eth_getBalance result exceeds 32 bytes".to_string()));
        }
        Ok(U256::from_be_slice(&bytes))
    }
}
