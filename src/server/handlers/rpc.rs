use axum::extract::State;
use axum::Json;
use serde_json::Value;

use crate::analyzer::tx_parser::decode_transaction;
use crate::error::ApiError;
use crate::server::rpc_types::{JsonRpcRequest, JsonRpcResponse, SendTransactionConfig};
use crate::state::AppState;

pub async fn handle_rpc(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "sendTransaction" => handle_send_transaction(&state, &req).await,
        _ => forward_to_rpc(&state, &req).await,
    };

    match result {
        Ok(resp) => Json(resp),
        Err(err) => Json(err.into_rpc_response(req.id.clone())),
    }
}

async fn handle_send_transaction(
    state: &AppState,
    req: &JsonRpcRequest,
) -> Result<JsonRpcResponse, ApiError> {
    // Extract encoded TX from params[0]
    let encoded_tx = req
        .params
        .first()
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::InvalidRequest("missing transaction parameter".to_string()))?;

    // Extract optional config from params[1]
    let user_config: SendTransactionConfig = req
        .params
        .get(1)
        .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
        .unwrap_or_default();

    // Decode transaction
    let tx = decode_transaction(encoded_tx, user_config.encoding.as_deref())?;

    tracing::info!(
        signatures = ?tx.signatures,
        "sendTransaction received"
    );

    // Forward to Solana RPC (Tier 1: always direct forward)
    let signature = state.rpc_sender.send_transaction(&tx, &user_config).await?;

    tracing::info!(%signature, "transaction forwarded");

    Ok(JsonRpcResponse::success(
        req.id.clone(),
        Value::String(signature.to_string()),
    ))
}

/// Pass-through: forward any non-sendTransaction JSON-RPC request to upstream Solana RPC.
async fn forward_to_rpc(
    state: &AppState,
    req: &JsonRpcRequest,
) -> Result<JsonRpcResponse, ApiError> {
    let body = serde_json::json!({
        "jsonrpc": req.jsonrpc,
        "id": req.id,
        "method": req.method,
        "params": req.params,
    });

    let resp = state
        .http_client
        .post(state.config.solana_rpc_url.as_str())
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::RpcForwardFailed(e.to_string()))?;

    let rpc_response: Value = resp
        .json()
        .await
        .map_err(|e| ApiError::RpcForwardFailed(e.to_string()))?;

    // Return the upstream response as-is, preserving original id
    if let Some(error) = rpc_response.get("error") {
        Ok(JsonRpcResponse::error(
            req.id.clone(),
            serde_json::from_value(error.clone())
                .map_err(|e| ApiError::Internal(e.into()))?,
        ))
    } else {
        Ok(JsonRpcResponse::success(
            req.id.clone(),
            rpc_response
                .get("result")
                .cloned()
                .unwrap_or(Value::Null),
        ))
    }
}
