use bigdecimal::num_bigint::BigInt;
use bigdecimal::BigDecimal;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use zoo_message_primitives::schemas::coinbase_mpc_config::CoinbaseMPCWalletConfig;
use zoo_message_primitives::schemas::zoo_name::ZooName;
use zoo_non_rust_code::functions::x402;
use zoo_sqlite::SqliteManager;
use zoo_tools_primitives::tools::zoo_tool::ZooTool;
use zoo_tools_primitives::tools::tool_config::ToolConfig;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Weak};

use super::wallet_manager::WalletEnum;
use super::wallet_traits::{CommonActions, IsWallet, PaymentWallet, ReceivingWallet, SendActions, TransactionHash};
use crate::utils::environment::fetch_node_environment;
use crate::wallet::wallet_error::WalletError;
use zoo_message_primitives::schemas::wallet_mixed::{
    Address, AddressBalanceList, Asset, AssetType, Balance, PublicAddress
};
use zoo_message_primitives::schemas::x402_types::{Network, PaymentRequirements};

#[derive(Debug, Clone)]
pub struct CoinbaseMPCWallet {
    pub id: String,
    pub network: Network,
    pub address: Address,
    pub config: CoinbaseMPCWalletConfig,
    pub sqlite_manager: Option<Weak<SqliteManager>>,
}

// Note: do we need access to ToolRouter? (maybe not, since we can call the Coinbase SDK directly)
// Should we create a new manager that calls the Coinbase MPC SDK directly? (Probably)
// So we still need access to lancedb so we can get the code for each tool
// If we use lancedb each time (it's slightly slower) but we can have everything in sync

// We could have an UI in Settings, where we can select the Coinbase Wallet or the Ethers Local Wallet

// Note: maybe we should create a new struct that holds the information about Config + Params + Results (for each tool)
// based on what we have in the typescript tools

impl Serialize for CoinbaseMPCWallet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CoinbaseMPCWallet", 5)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("network", &self.network)?;
        state.serialize_field("address", &self.address)?;
        state.serialize_field("config", &self.config)?;
        // Serialize lance_db as a placeholder since Weak references cannot be serialized directly
        state.serialize_field("lance_db", &"Option<Weak<RwLock<LanceZooDb>>>")?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for CoinbaseMPCWallet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CoinbaseMPCWalletData {
            id: String,
            network: Network,
            address: Address,
            config: CoinbaseMPCWalletConfig,
        }

        let data = CoinbaseMPCWalletData::deserialize(deserializer)?;
        // Deserialize lance_db as a placeholder since Weak references cannot be deserialized directly

        Ok(CoinbaseMPCWallet {
            id: data.id,
            network: data.network,
            address: data.address,
            config: data.config,
            sqlite_manager: None,
        })
    }
}

impl CoinbaseMPCWallet {
    pub fn update_sqlite_manager(&mut self, sqlite_manager: Arc<SqliteManager>) {
        self.sqlite_manager = Some(Arc::downgrade(&sqlite_manager));
    }

    pub async fn create_wallet(
        network: Network,
        sqlite_manager: Weak<SqliteManager>, // Changed to Weak
        config: Option<CoinbaseMPCWalletConfig>,
        node_name: ZooName,
    ) -> Result<Self, WalletError> {
        let sqlite_manager_strong = sqlite_manager.upgrade().ok_or(WalletError::ConfigNotFound)?;
        let mut config = match config {
            Some(cfg) => cfg,
            None => {
                let tool_id = ZooToolCoinbase::CreateWallet.definition_id();
                let zoo_tool = sqlite_manager_strong
                    .get_tool_by_key(tool_id)
                    .map_err(|e| WalletError::SqliteManagerError(e.to_string()))?;

                // Extract the required configuration from the JSTool
                let mut name = String::new();
                let mut private_key = String::new();
                let mut use_server_signer = String::new();
                if let ZooTool::Deno(js_tool, _) = zoo_tool {
                    for cfg in js_tool.config {
                        match cfg {
                            ToolConfig::BasicConfig(basic_config) => match basic_config.key_name.as_str() {
                                "name" => name = basic_config.key_value.clone().unwrap_or_default().to_string(),
                                "privateKey" => {
                                    private_key = basic_config.key_value.clone().unwrap_or_default().to_string()
                                }
                                "useServerSigner" => {
                                    use_server_signer = basic_config.key_value.clone().unwrap_or_default().to_string()
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                } else {
                    return Err(WalletError::ConfigNotFound);
                }

                CoinbaseMPCWalletConfig {
                    name,
                    private_key,
                    wallet_id: None,
                    use_server_signer: Some(use_server_signer),
                }
            }
        };

        // Call the function to create the wallet
        let params = serde_json::json!({
            "name": config.name,
            "privateKey": config.private_key,
            "useServerSigner": config.use_server_signer,
        })
        .as_object()
        .unwrap()
        .to_owned();

        let response = Self::call_function(
            config.clone(),
            sqlite_manager.clone(),
            ZooToolCoinbase::CreateWallet,
            params,
            node_name,
        )
        .await?;

        // Extract the necessary fields from the response
        let wallet_id = response
            .get("walletId")
            .and_then(|v| v.as_str())
            .ok_or(WalletError::ConfigNotFound)?
            .to_string();
        let address_id = response
            .get("address")
            .and_then(|v| v.as_str())
            .ok_or(WalletError::ConfigNotFound)?
            .to_string();

        // Update the config with the wallet_id
        config.wallet_id = Some(wallet_id.clone());

        // Use the extracted fields to create the wallet
        let wallet = CoinbaseMPCWallet {
            id: wallet_id.clone(),
            config,
            network: network.clone(),
            address: Address {
                wallet_id: wallet_id,
                network_id: network.clone(),
                public_key: None,
                address_id,
            },
            sqlite_manager: Some(sqlite_manager), // Store the Weak reference
        };

        Ok(wallet)
    }

    pub async fn restore_wallet(
        network: Network,
        sqlite_manager: Weak<SqliteManager>,
        config: Option<CoinbaseMPCWalletConfig>,
        wallet_id: String,
        node_name: ZooName,
    ) -> Result<Self, WalletError> {
        let sqlite_manager_strong = sqlite_manager
            .upgrade()
            .ok_or(WalletError::SqliteManagerError("SqliteManager not found".to_string()))?;
        let config = match config {
            Some(cfg) => cfg,
            None => {
                let tool_id = ZooToolCoinbase::CreateWallet.definition_id();
                let zoo_tool = sqlite_manager_strong
                    .get_tool_by_key(tool_id)
                    .map_err(|e| WalletError::SqliteManagerError(e.to_string()))?;

                // Extract the required configuration from the JSTool
                let mut name = String::new();
                let mut private_key = String::new();
                let mut use_server_signer = String::new();
                if let ZooTool::Deno(js_tool, _) = zoo_tool {
                    for cfg in js_tool.config {
                        match cfg {
                            ToolConfig::BasicConfig(basic_config) => match basic_config.key_name.as_str() {
                                "name" => name = basic_config.key_value.clone().unwrap_or_default().to_string(),
                                "privateKey" => {
                                    private_key = basic_config.key_value.clone().unwrap_or_default().to_string()
                                }
                                "useServerSigner" => {
                                    use_server_signer = basic_config.key_value.clone().unwrap_or_default().to_string()
                                }
                                "walletId" => {
                                    if basic_config.key_value.is_none() {
                                        return Err(WalletError::ConfigNotFound);
                                    }
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                } else {
                    return Err(WalletError::ConfigNotFound);
                }

                CoinbaseMPCWalletConfig {
                    name,
                    private_key,
                    wallet_id: Some(wallet_id.clone()),
                    use_server_signer: Some(use_server_signer),
                }
            }
        };

        let params = serde_json::json!({}).as_object().unwrap().to_owned();

        let response = match Self::call_function(
            config.clone(),
            sqlite_manager.clone(),
            ZooToolCoinbase::GetMyAddress,
            params,
            node_name,
        )
        .await
        {
            Ok(res) => res,
            Err(e) => {
                println!("Error calling function: {:?}", e);
                return Err(e);
            }
        };

        // Extract the necessary fields from the response
        let address_id = response
            .get("data")
            .and_then(|data| data.get("address"))
            .and_then(|v| v.as_str())
            .ok_or(WalletError::ParsingError("Address not found".to_string()))?
            .to_string();

        // Use the extracted fields to create the wallet
        let wallet = CoinbaseMPCWallet {
            id: wallet_id.clone(),
            network: network.clone(),
            config,
            address: Address {
                wallet_id: wallet_id,
                network_id: network.clone(),
                public_key: None,
                address_id,
            },
            sqlite_manager: Some(sqlite_manager),
        };

        Ok(wallet)
    }

    pub async fn call_function(
        config: CoinbaseMPCWalletConfig,
        sqlite_manager: Weak<SqliteManager>,
        function_name: ZooToolCoinbase,
        params: serde_json::Map<String, Value>,
        node_name: ZooName,
    ) -> Result<Value, WalletError> {
        let sqlite_manager_strong = sqlite_manager
            .upgrade()
            .ok_or(WalletError::SqliteManagerError("SqliteManager not found".to_string()))?;
        let tool_id = function_name.definition_id();
        let zoo_tool = sqlite_manager_strong
            .get_tool_by_key(tool_id)
            .map_err(|e| WalletError::SqliteManagerError(e.to_string()))?;

        let mut function_config_value = serde_json::json!({});

        // Overwrite function_config_value with values from config
        function_config_value["name"] = Value::String(config.name);
        function_config_value["privateKey"] = Value::String(config.private_key);
        if let Some(use_server_signer) = config.use_server_signer {
            function_config_value["useServerSigner"] = Value::String(use_server_signer);
        }
        if let Some(wallet_id) = config.wallet_id {
            function_config_value["walletId"] = Value::String(wallet_id);
        }

        let tool_configs = ToolConfig::basic_config_from_value(&function_config_value);

        if let ZooTool::Deno(js_tool, _) = zoo_tool {
            let node_env = fetch_node_environment();
            let node_storage_path = node_env
                .node_storage_path
                .clone()
                .ok_or_else(|| WalletError::FunctionExecutionError("Node storage path is not set".to_string()))?;
            let app_id = format!("coinbase_{}", uuid::Uuid::new_v4());
            let tool_id = js_tool.name.clone();
            let support_files = HashMap::new();
            let result = js_tool
                .run(
                    HashMap::new(), // Note: we don't need envs for this function - as it doesn't call other tools
                    node_env.api_listen_address.ip().to_string(),
                    node_env.api_listen_address.port(),
                    support_files,
                    params,
                    tool_configs,
                    node_storage_path,
                    app_id,
                    tool_id,
                    node_name,
                    false,
                    None,
                    None,
                )
                .await
                .map_err(|e| WalletError::FunctionExecutionError(e.to_string()))?;
            let result_str =
                serde_json::to_string(&result).map_err(|e| WalletError::FunctionExecutionError(e.to_string()))?;
            return Ok(
                serde_json::from_str(&result_str).map_err(|e| WalletError::FunctionExecutionError(e.to_string()))?
            );
        }

        Err(WalletError::FunctionNotFound(tool_id.to_string()))
    }
}

impl IsWallet for CoinbaseMPCWallet {}

impl PaymentWallet for CoinbaseMPCWallet {
    fn to_wallet_enum(&self) -> WalletEnum {
        WalletEnum::CoinbaseMPCWallet(self.clone())
    }
}

impl ReceivingWallet for CoinbaseMPCWallet {
    fn to_wallet_enum(&self) -> WalletEnum {
        WalletEnum::CoinbaseMPCWallet(self.clone())
    }
}

impl CommonActions for CoinbaseMPCWallet {
    fn get_payment_address(&self) -> PublicAddress {
        self.address.clone().into()
    }

    fn get_address(&self) -> Address {
        self.address.clone()
    }

    fn get_balance(
        &self,
        node_name: ZooName,
    ) -> Pin<Box<dyn Future<Output = Result<f64, WalletError>> + Send + 'static>> {
        let config = self.config.clone();
        let sqlite_manager = match self.sqlite_manager.clone() {
            Some(manager) => manager,
            None => {
                return Box::pin(
                    async move { Err(WalletError::SqliteManagerError("SqliteManager not found".to_string())) },
                )
            }
        };

        Box::pin(async move {
            let params = serde_json::json!({
                "walletId": config.wallet_id,
            })
            .as_object()
            .unwrap()
            .to_owned();

            let response = CoinbaseMPCWallet::call_function(
                config,
                sqlite_manager,
                ZooToolCoinbase::GetBalance,
                params,
                node_name,
            )
            .await?;

            let balance_str = response
                .get("balance")
                .and_then(|v| v.as_str())
                .ok_or(WalletError::ConfigNotFound)?;

            let balance: f64 = balance_str
                .parse()
                .map_err(|e: std::num::ParseFloatError| WalletError::ConversionError(e.to_string()))?;
            Ok(balance)
        })
    }

    fn check_balances(
        &self,
        node_name: ZooName,
    ) -> Pin<Box<dyn Future<Output = Result<AddressBalanceList, WalletError>> + Send + 'static>> {
        let config = self.config.clone();

        let sqlite_manager = match self.sqlite_manager.clone() {
            Some(manager) => manager,
            None => {
                return Box::pin(
                    async move { Err(WalletError::SqliteManagerError("SqliteManager not found".to_string())) },
                )
            }
        };

        Box::pin(async move {
            let params = serde_json::json!({
                "walletId": config.wallet_id.clone(),
            })
            .as_object()
            .unwrap()
            .to_owned();

            let response = CoinbaseMPCWallet::call_function(
                config.clone(),
                sqlite_manager,
                ZooToolCoinbase::GetBalance,
                params,
                node_name,
            )
            .await?;

            eprintln!("response: {:?}", response);

            let balances = response
                .get("data")
                .and_then(|data| data.get("balances"))
                .and_then(|v| v.as_object())
                .ok_or(WalletError::ConfigNotFound)?;

            let data: Vec<Balance> = balances
                .iter()
                .filter_map(|(asset, amount)| {
                    let amount = amount.as_f64().unwrap_or(0.0);
                    match asset.as_str() {
                        "ETH" => Some(Balance {
                            amount: amount.to_string(),
                            decimals: Some(18),
                            asset: Asset {
                                asset_id: asset.clone(),
                                decimals: Some(18),
                                network_id: Network::BaseSepolia,
                                contract_address: None,
                            },
                        }),
                        "USDC" => Some(Balance {
                            amount: amount.to_string(),
                            decimals: Some(6),
                            asset: Asset::new(AssetType::USDC, &Network::BaseSepolia)?,
                        }),
                        _ => None,
                    }
                })
                .collect();

            let has_more = response
                .clone()
                .get("has_more")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let next_page = response
                .get("next_page")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default(); // Set to empty string if None

            let total_count = response
                .clone()
                .get("total_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let address_balance_list = AddressBalanceList {
                data,
                has_more,
                next_page,
                total_count: total_count as u32,
            };

            Ok(address_balance_list.clone())
        })
    }

    fn check_asset_balance(
        &self,
        public_address: PublicAddress,
        asset: Asset,
        node_name: ZooName,
    ) -> Pin<Box<dyn Future<Output = Result<Balance, WalletError>> + Send + 'static>> {
        let config = self.config.clone();
        let sqlite_manager = match self.sqlite_manager.clone() {
            Some(manager) => manager,
            None => {
                return Box::pin(
                    async move { Err(WalletError::SqliteManagerError("SqliteManager not found".to_string())) },
                )
            }
        };

        Box::pin(async move {
            let params = serde_json::json!({
                "walletId": config.wallet_id,
                "publicAddress": public_address.address_id,
                "asset": asset.asset_id,
            })
            .as_object()
            .unwrap()
            .to_owned();

            let response = CoinbaseMPCWallet::call_function(
                config,
                sqlite_manager.clone(),
                ZooToolCoinbase::GetBalance,
                params,
                node_name,
            )
            .await?;

            let data = response
                .get("data")
                .and_then(|v| v.as_object())
                .ok_or(WalletError::ParsingError("Data object not found".to_string()))?;

            let balances = data
                .get("balances")
                .and_then(|v| v.as_object())
                .ok_or(WalletError::ParsingError("Balances object not found".to_string()))?;

            // Convert asset ID to lowercase to match the response keys
            let asset_id_lower = asset.asset_id.to_lowercase();

            let amount = balances
                .get(&asset_id_lower)
                .and_then(|v| v.as_f64())
                .ok_or(WalletError::ParsingError(format!(
                    "Amount for asset {} not found",
                    asset.asset_id
                )))?;

            // Use decimals from the asset, default to 18 if None
            let decimals = asset.decimals.unwrap_or(18);

            // Normalize the amount based on the decimals using BigDecimal
            let amount_str = amount.to_string();
            let amount_bigdecimal =
                BigDecimal::from_str(&amount_str).map_err(|e| WalletError::ParsingError(e.to_string()))?;
            let factor = BigInt::from(10u64).pow(decimals as u32);
            let normalized_amount = amount_bigdecimal * BigDecimal::new(factor, 0);

            let balance = Balance {
                amount: normalized_amount.to_string(),
                decimals: Some(decimals as u32),
                asset,
            };

            Ok(balance)
        })
    }
}

impl SendActions for CoinbaseMPCWallet {
    fn send_transaction(
        &self,
        to_wallet: PublicAddress,
        token: Option<Asset>,
        send_amount: String,
        _invoice_id: String,
        node_name: ZooName,
    ) -> Pin<Box<dyn Future<Output = Result<TransactionHash, WalletError>> + Send + 'static>> {
        let send_amount_bd = BigDecimal::from_str(&send_amount);
        let send_amount_bd = match send_amount_bd {
            Ok(amount) => amount,
            Err(e) => return Box::pin(async move { Err(WalletError::ConversionError(e.to_string())) }),
        };

        let config = self.config.clone();
        let sqlite_manager = match self.sqlite_manager.clone() {
            Some(manager) => manager,
            None => {
                return Box::pin(
                    async move { Err(WalletError::SqliteManagerError("SqliteManager not found".to_string())) },
                )
            }
        };

        // Normalize send_amount to the asset decimals e.g. Instead of 1000, it should be 0.001
        let normalized_amount = if let Some(asset) = &token {
            let decimals = asset.decimals.unwrap_or(18);
            let factor = BigDecimal::from(10u64.pow(decimals as u32));
            send_amount_bd / factor
        } else {
            send_amount_bd
        };

        let fut = async move {
            let params = serde_json::json!({
                "recipient_address": to_wallet.address_id,
                "assetId": token.map_or("".to_string(), |t| t.asset_id),
                "amount": normalized_amount.to_string(),
            })
            .as_object()
            .unwrap()
            .to_owned();

            let response = CoinbaseMPCWallet::call_function(
                config,
                sqlite_manager,
                ZooToolCoinbase::SendTx,
                params,
                node_name,
            )
            .await?;

            let tx_hash = response
                .get("data")
                .and_then(|data| data.get("transactionHash"))
                .and_then(|v| v.as_str())
                .ok_or(WalletError::ConfigNotFound)?
                .to_string();

            Ok(tx_hash)
        };

        Box::pin(fut)
    }

    fn sign_transaction(
        &self,
        _tx: zoo_message_primitives::schemas::wallet_mixed::Transaction,
    ) -> Pin<Box<dyn Future<Output = Result<String, WalletError>> + Send + 'static>> {
        let fut = async move {
            // Mock implementation for signing a transaction
            Ok("mock_signature".to_string())
        };

        Box::pin(fut)
    }

    fn create_payment_request(
        &self,
        _payment_requirements: PaymentRequirements,
    ) -> Pin<Box<dyn Future<Output = Result<x402::create_payment::Output, WalletError>> + Send>> {
        unimplemented!()
    }
}

pub enum ZooToolCoinbase {
    CreateWallet,
    GetMyAddress,
    GetBalance,
    GetTransactions,
    SendTx,
    CallFaucet,
}

impl ZooToolCoinbase {
    pub fn definition_id(&self) -> &'static str {
        match self {
            ZooToolCoinbase::CreateWallet => {
                "local:::zoo-tool-coinbase-create-wallet:::zoo__coinbase_wallet_creator"
            }
            ZooToolCoinbase::GetMyAddress => {
                "local:::zoo-tool-coinbase-get-my-address:::zoo__coinbase_my_address_getter"
            }
            ZooToolCoinbase::GetBalance => {
                "local:::zoo-tool-coinbase-get-balance:::zoo__coinbase_balance_getter"
            }
            ZooToolCoinbase::GetTransactions => {
                "local:::zoo-tool-coinbase-get-transactions:::zoo__coinbase_transactions_getter"
            }
            ZooToolCoinbase::SendTx => {
                "local:::zoo-tool-coinbase-send-tx:::zoo__coinbase_transaction_sender"
            }
            ZooToolCoinbase::CallFaucet => {
                "local:::zoo-tool-coinbase-call-faucet:::zoo__coinbase_faucet_caller"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use zoo_message_primitives::schemas::x402_types::Network;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_normalize_send_amount() {
        let token = Some(Asset {
            asset_id: "USDC".to_string(),
            decimals: Some(6),
            network_id: Network::BaseSepolia,
            contract_address: None,
        });

        let send_amount = "1000".to_string();
        let send_amount_bd = BigDecimal::from_str(&send_amount).unwrap();

        // Normalize send_amount to the asset decimals e.g. Instead of 1000, it should be 0.001
        let normalized_amount = if let Some(asset) = &token {
            let decimals = asset.decimals.unwrap_or(18);
            let factor = BigDecimal::from(10u64.pow(decimals as u32));
            send_amount_bd / factor
        } else {
            send_amount_bd
        };

        // Convert normalized_amount to a string for comparison
        let normalized_amount_str = normalized_amount.to_string();

        // The expected result should be "0.001"
        assert_eq!(normalized_amount_str, "0.001");
    }
}
