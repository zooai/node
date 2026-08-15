//! Look up an identity in the on-chain registry.
//!
//! The registry's `getIdentityData` returns a record of zeroes for a name it
//! does not know, and an endpoint can be lagging or unreachable, so the
//! endpoints are tried in turn until one answers with a real record.

use std::time::Duration;

use alloy_dyn_abi::{DynSolValue, FunctionExt, JsonAbiExt};
use alloy_json_abi::{Function, JsonAbi};
use alloy_primitives::{Address, U256};
use serde::Deserialize;

use crate::{rpc::Rpc, RunError};

/// How long a single endpoint gets to answer before the next one is tried.
const TIMEOUT: Duration = Duration::from_secs(5);

const CALL: &str = "getIdentityData";

#[derive(Debug, Deserialize)]
pub struct IdentityData {
    #[serde(rename = "boundNft")]
    pub bound_nft: String,
    #[serde(rename = "stakedTokens")]
    pub staked_tokens: String,
    #[serde(rename = "encryptionKey")]
    pub encryption_key: String,
    #[serde(rename = "signatureKey")]
    pub signature_key: String,
    #[serde(rename = "routing")]
    pub routing: bool,
    #[serde(rename = "addressOrProxyNodes")]
    pub address_or_proxy_nodes: Vec<String>,
    #[serde(rename = "delegatedTokens")]
    pub delegated_tokens: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: u64,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    #[serde(rename = "identityData")]
    pub identity_data: Option<IdentityData>,
}

/// Find `getIdentityData` in a contract ABI.
///
/// The ABI arrives as a compiler artifact, which wraps the entry list under
/// `abi`, or as the bare entry list.
fn declared(contract_abi: &str, name: &str) -> Result<Function, RunError> {
    let document: serde_json::Value = serde_json::from_str(contract_abi)
        .map_err(|e| RunError::SerializeConfigurationsError(format!("contract abi is not json: {e}")))?;
    let entries = match document {
        serde_json::Value::Object(mut fields) => fields
            .remove("abi")
            .ok_or_else(|| RunError::SerializeConfigurationsError("contract abi has no abi entry".to_string()))?,
        entries => entries,
    };
    let abi: JsonAbi = serde_json::from_value(entries)
        .map_err(|e| RunError::SerializeConfigurationsError(format!("contract abi is malformed: {e}")))?;
    abi.function(name)
        .and_then(|overloads| overloads.first())
        .cloned()
        .ok_or_else(|| RunError::SerializeConfigurationsError(format!("contract abi declares no {name}")))
}

fn number(value: &DynSolValue) -> Option<U256> {
    match value {
        DynSolValue::Uint(number, _) => Some(*number),
        _ => None,
    }
}

fn text(value: &DynSolValue) -> Option<String> {
    match value {
        DynSolValue::String(text) => Some(text.clone()),
        _ => None,
    }
}

fn flag(value: &DynSolValue) -> Option<bool> {
    match value {
        DynSolValue::Bool(flag) => Some(*flag),
        _ => None,
    }
}

/// Read the returned record, or `None` when the registry answered with the
/// all-zero record it gives for an unknown name.
fn record(function: &Function, returned: &[u8]) -> Result<Option<IdentityData>, RunError> {
    let decoded = function
        .abi_decode_output(returned)
        .map_err(|e| RunError::ParseOutputError(format!("{CALL} returned unexpected data: {e}")))?;

    // The record is a struct, which the ABI carries as a single tuple.
    let fields = match decoded.as_slice() {
        [DynSolValue::Tuple(fields)] => fields.clone(),
        fields => fields.to_vec(),
    };
    let [bound_nft, staked, encryption, signature, routing, nodes, delegated, updated] = fields.as_slice() else {
        return Err(RunError::ParseOutputError(format!(
            "{CALL} returned {} fields, expected 8",
            fields.len()
        )));
    };

    let missing = |what: &str| RunError::ParseOutputError(format!("{CALL} returned no {what}"));
    let bound_nft = number(bound_nft).ok_or_else(|| missing("boundNft"))?;
    let staked = number(staked).ok_or_else(|| missing("stakedTokens"))?;
    let delegated = number(delegated).ok_or_else(|| missing("delegatedTokens"))?;
    let updated = number(updated).ok_or_else(|| missing("lastUpdated"))?;
    let encryption = text(encryption).ok_or_else(|| missing("encryptionKey"))?;
    let signature = text(signature).ok_or_else(|| missing("signatureKey"))?;
    let routing = flag(routing).ok_or_else(|| missing("routing"))?;
    let nodes: Vec<String> = match nodes {
        DynSolValue::Array(items) => items.iter().filter_map(text).collect(),
        _ => return Err(missing("addressOrProxyNodes")),
    };

    let unknown = bound_nft.is_zero()
        && staked.is_zero()
        && delegated.is_zero()
        && updated.is_zero()
        && encryption.is_empty()
        && signature.is_empty()
        && !routing
        && nodes.is_empty();
    if unknown {
        return Ok(None);
    }

    Ok(Some(IdentityData {
        // Token quantities keep the `n` the registry's own clients report,
        // marking them as whole numbers too large for a JSON number.
        bound_nft: format!("{bound_nft}n"),
        staked_tokens: format!("{staked}n"),
        encryption_key: encryption,
        signature_key: signature,
        routing,
        address_or_proxy_nodes: nodes,
        delegated_tokens: format!("{delegated}n"),
        last_updated: updated.saturating_to(),
    }))
}

pub async fn get_identity_data(
    rpc_urls: Vec<String>,
    contract_address: String,
    contract_abi: String,
    identity_id: String,
) -> Result<Output, RunError> {
    let function = declared(&contract_abi, CALL)?;
    let registry = contract_address
        .parse::<Address>()
        .map_err(|e| RunError::SerializeConfigurationsError(format!("contract address is not an address: {e}")))?;
    let call = function
        .abi_encode_input(&[DynSolValue::String(identity_id.clone())])
        .map_err(|e| RunError::SerializeParamsError(format!("could not encode {CALL}({identity_id}): {e}")))?;

    for url in &rpc_urls {
        let answer = match Rpc::new(url, TIMEOUT) {
            Ok(rpc) => rpc.call(registry, &call).await,
            Err(error) => Err(error),
        };
        match answer.and_then(|returned| record(&function, &returned)) {
            Ok(Some(identity)) => {
                return Ok(Output {
                    identity_data: Some(identity),
                })
            }
            Ok(None) => log::debug!("{CALL} found no record for {identity_id} at {url}"),
            Err(error) => log::warn!("{CALL} failed for rpc:{url} with error:{error}"),
        }
    }

    Ok(Output { identity_data: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_sol_types::SolValue;

    const ABI: &str = include_str!("../../../hanzo-identity/src/abi/HanzoRegistry.sol/HanzoRegistry.json");
    const REGISTRY: &str = "0x425Fb20ba3874e887336aAa7f3fab32D08135BA9";

    fn function() -> Function {
        declared(ABI, CALL).unwrap()
    }

    /// The registry's own artifact must yield a call this code can encode.
    #[test]
    fn reads_the_registry_abi() {
        let function = function();
        assert_eq!(function.name, CALL);
        assert_eq!(function.inputs.len(), 1);
        assert_eq!(function.inputs[0].ty, "string");

        let call = function
            .abi_encode_input(&[DynSolValue::String("official.sep-hanzo".to_string())])
            .unwrap();
        assert_eq!(call[..4], function.selector()[..]);
    }

    /// The registry ships a bare entry list; a compiler artifact wraps the same
    /// list under `abi`. Both are accepted.
    #[test]
    fn an_artifact_wrapper_is_also_an_abi() {
        let entries: serde_json::Value = serde_json::from_str(ABI).unwrap();
        assert!(entries.is_array(), "the registry abi is a bare entry list");
        let artifact = serde_json::json!({ "abi": entries, "bytecode": "0x" }).to_string();
        assert_eq!(declared(&artifact, CALL).unwrap().selector(), function().selector());
    }

    #[test]
    fn rejects_an_abi_it_cannot_use() {
        assert!(declared("{{not json", CALL).is_err());
        assert!(declared("[]", CALL).is_err());
        assert!(declared(ABI, "noSuchFunction").is_err());
    }

    /// Build the reply the registry would return, using the registry's own ABI
    /// so the encoding under test is the encoding the contract emits.
    fn reply(
        bound_nft: u64,
        staked: &str,
        encryption: &str,
        signature: &str,
        routing: bool,
        nodes: &[&str],
        delegated: u64,
        updated: u64,
    ) -> Vec<u8> {
        let record = DynSolValue::Tuple(vec![
            DynSolValue::Uint(U256::from(bound_nft), 256),
            DynSolValue::Uint(U256::from_str_radix(staked, 10).unwrap(), 256),
            DynSolValue::String(encryption.to_string()),
            DynSolValue::String(signature.to_string()),
            DynSolValue::Bool(routing),
            DynSolValue::Array(nodes.iter().map(|n| DynSolValue::String(n.to_string())).collect()),
            DynSolValue::Uint(U256::from(delegated), 256),
            DynSolValue::Uint(U256::from(updated), 256),
        ]);
        function().abi_encode_output(&[record]).unwrap()
    }

    /// A record of zeroes is how the registry says "no such identity", and must
    /// not become an identity with empty keys.
    #[test]
    fn an_all_zero_record_is_no_record() {
        let empty = reply(0, "0", "", "", false, &[], 0, 0);
        assert!(record(&function(), &empty).unwrap().is_none());
    }

    /// A populated record keeps the `n` suffix on token quantities, which is
    /// what `OnchainIdentity` carries downstream.
    #[test]
    fn a_populated_record_is_read_field_by_field() {
        let filled = reply(
            4,
            "165000000000000000000",
            "9d89af22de24fcc621ed47a08e98f1c52fada3e49b98462cb02c48237940c85b",
            "1ffbfa5d90e7b79b395d034f81ec07ea0c7eabd6c9a510014173c6e5081411d1",
            true,
            &["hanzo.ai:443", "other.sep-hanzo"],
            0,
            1_715_000_001,
        );
        let identity = record(&function(), &filled).unwrap().unwrap();

        assert_eq!(identity.bound_nft, "4n");
        assert_eq!(identity.staked_tokens, "165000000000000000000n");
        assert_eq!(identity.delegated_tokens, "0n");
        assert_eq!(
            identity.encryption_key,
            "9d89af22de24fcc621ed47a08e98f1c52fada3e49b98462cb02c48237940c85b"
        );
        assert_eq!(
            identity.signature_key,
            "1ffbfa5d90e7b79b395d034f81ec07ea0c7eabd6c9a510014173c6e5081411d1"
        );
        assert!(identity.routing);
        assert_eq!(identity.address_or_proxy_nodes, vec!["hanzo.ai:443", "other.sep-hanzo"]);
        assert_eq!(identity.last_updated, 1_715_000_001);
    }

    /// A single routing entry is enough to make a record real, even when every
    /// quantity on it is still zero.
    #[test]
    fn a_record_with_only_a_node_is_still_a_record() {
        let sparse = reply(0, "0", "", "", false, &["proxy.sep-hanzo"], 0, 0);
        let identity = record(&function(), &sparse).unwrap().unwrap();
        assert_eq!(identity.address_or_proxy_nodes, vec!["proxy.sep-hanzo"]);
        assert_eq!(identity.bound_nft, "0n");
    }

    #[test]
    fn refuses_a_reply_that_is_not_the_record() {
        assert!(record(&function(), &[]).is_err());
        assert!(record(&function(), &U256::from(1u64).abi_encode()).is_err());
    }

    /// Endpoints that cannot answer are stepped over, and exhausting them is a
    /// "not found", not an error.
    #[tokio::test]
    async fn unreachable_endpoints_yield_no_identity() {
        let output = get_identity_data(
            // Reserved for documentation, on a port nothing listens to.
            vec!["http://192.0.2.1:9".to_string(), "http://192.0.2.2:9".to_string()],
            REGISTRY.to_string(),
            ABI.to_string(),
            "official.sep-hanzo".to_string(),
        )
        .await
        .unwrap();
        assert!(output.identity_data.is_none());
    }

    #[tokio::test]
    async fn a_bad_contract_address_is_an_error() {
        assert!(get_identity_data(
            vec!["http://192.0.2.1:9".to_string()],
            "not-an-address".to_string(),
            ABI.to_string(),
            "official.sep-hanzo".to_string(),
        )
        .await
        .is_err());
    }

    #[tokio::test]
    #[ignore = "Requires external RPC endpoints which may be unreliable"]
    async fn test_get_identity_data() {
        let output = get_identity_data(
            vec![
                "https://sepolia.base.org".to_string(),
                "https://base-sepolia-rpc.publicnode.com".to_string(),
                "https://base-sepolia.gateway.tenderly.co".to_string(),
            ],
            REGISTRY.to_string(),
            ABI.to_string(),
            "official.sep-hanzo".to_string(),
        )
        .await
        .unwrap();
        println!("output: {:?}", output);

        let identity_data = output.identity_data.unwrap();
        assert_eq!(identity_data.bound_nft, "4n");
        assert_eq!(
            identity_data.encryption_key,
            "9d89af22de24fcc621ed47a08e98f1c52fada3e49b98462cb02c48237940c85b"
        );
        assert_eq!(
            identity_data.signature_key,
            "1ffbfa5d90e7b79b395d034f81ec07ea0c7eabd6c9a510014173c6e5081411d1"
        );
        assert_eq!(identity_data.staked_tokens, "165000000000000000000n");
        assert_eq!(identity_data.delegated_tokens, "0n");
        assert!(identity_data.last_updated > 1715000000);
    }
}
