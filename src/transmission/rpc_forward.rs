use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::UiTransactionEncoding;

use crate::error::ApiError;
use crate::server::rpc_types::SendTransactionConfig;

pub struct DirectRpcSender {
    client: RpcClient,
}

impl DirectRpcSender {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: RpcClient::new(rpc_url.to_string()),
        }
    }

    pub async fn send_transaction(
        &self,
        tx: &VersionedTransaction,
        user_config: &SendTransactionConfig,
    ) -> Result<Signature, ApiError> {
        let config = RpcSendTransactionConfig {
            skip_preflight: user_config.skip_preflight.unwrap_or(false),
            preflight_commitment: user_config
                .preflight_commitment
                .as_deref()
                .map(commitment_from_str)
                .transpose()?,
            encoding: Some(UiTransactionEncoding::Base64),
            max_retries: user_config.max_retries.map(|r| r as usize),
            min_context_slot: None,
        };

        self.client
            .send_transaction_with_config(tx, config)
            .await
            .map_err(|e| ApiError::RpcForwardFailed(e.to_string()))
    }
}

fn commitment_from_str(s: &str) -> Result<CommitmentLevel, ApiError> {
    match s {
        "processed" => Ok(CommitmentLevel::Processed),
        "confirmed" => Ok(CommitmentLevel::Confirmed),
        "finalized" => Ok(CommitmentLevel::Finalized),
        other => Err(ApiError::InvalidRequest(format!(
            "unknown commitment: {other}"
        ))),
    }
}
