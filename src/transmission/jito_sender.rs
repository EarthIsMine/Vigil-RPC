use base64::Engine;
use serde_json::Value;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;

use crate::error::ApiError;

const JITO_MAINNET_URL: &str = "https://mainnet.block-engine.jito.wtf";

pub struct JitoSender {
    client: reqwest::Client,
    base_url: String,
}

impl JitoSender {
    pub fn new(client: reqwest::Client, base_url: Option<String>) -> Self {
        Self {
            client,
            base_url: base_url.unwrap_or_else(|| JITO_MAINNET_URL.to_string()),
        }
    }

    /// Send a single transaction via Jito's relay.
    /// Uses the /api/v1/transactions endpoint which accepts standard
    /// sendTransaction JSON-RPC, routing through Jito's private relay
    /// instead of the public mempool.
    pub async fn send_transaction(
        &self,
        tx: &VersionedTransaction,
    ) -> Result<Signature, ApiError> {
        let tx_bytes =
            bincode::serialize(tx).map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                encoded,
                {
                    "encoding": "base64",
                    "skipPreflight": true,
                    "maxRetries": 0
                }
            ]
        });

        let url = format!("{}/api/v1/transactions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::RpcForwardFailed(format!("jito relay: {e}")))?;

        let json: Value = resp
            .json()
            .await
            .map_err(|e| ApiError::RpcForwardFailed(format!("jito relay response: {e}")))?;

        if let Some(error) = json.get("error") {
            return Err(ApiError::RpcForwardFailed(format!(
                "jito relay error: {}",
                error
            )));
        }

        let sig_str = json["result"]
            .as_str()
            .ok_or_else(|| ApiError::RpcForwardFailed("jito: missing result".to_string()))?;

        sig_str
            .parse::<Signature>()
            .map_err(|e| ApiError::RpcForwardFailed(format!("jito: invalid signature: {e}")))
    }

    /// Send a bundle of transactions atomically via Jito Block Engine.
    /// All transactions execute together or none do.
    #[allow(dead_code)]
    pub async fn send_bundle(
        &self,
        transactions: &[VersionedTransaction],
    ) -> Result<String, ApiError> {
        let encoded_txs: Vec<String> = transactions
            .iter()
            .map(|tx| {
                let bytes = bincode::serialize(tx)
                    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
                Ok(bs58::encode(&bytes).into_string())
            })
            .collect::<Result<Vec<_>, ApiError>>()?;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [encoded_txs]
        });

        let url = format!("{}/api/v1/bundles", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::RpcForwardFailed(format!("jito bundle: {e}")))?;

        let json: Value = resp
            .json()
            .await
            .map_err(|e| ApiError::RpcForwardFailed(format!("jito bundle response: {e}")))?;

        if let Some(error) = json.get("error") {
            return Err(ApiError::RpcForwardFailed(format!(
                "jito bundle error: {}",
                error
            )));
        }

        json["result"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ApiError::RpcForwardFailed("jito: missing bundle id".to_string()))
    }
}
