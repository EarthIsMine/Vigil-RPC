use axum::extract::State;
use axum::Json;
use serde_json::Value;

use crate::analyzer::swap_detector::{detect_swap, extract_pool_address, signer_as_string};
use crate::analyzer::tx_parser::decode_transaction;
use crate::config::BlockMode;
use crate::error::ApiError;
use crate::risk::{assess, decode_slippage, RiskAssessment, RiskLevel};
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

    // Detect DEX swap using sandwich-detector's program ID registry
    let is_swap = detect_swap(&tx);
    if let Some(ref swap) = is_swap {
        tracing::info!(
            dex = %swap.dex_name,
            program_id = %swap.program_id,
            "swap transaction detected — protection candidate"
        );

        let assessment = evaluate_risk(state, &tx);
        state.metrics.record_risk(assessment.level);
        tracing::info!(
            level = ?assessment.level,
            pool_score = assessment.pool_score,
            reasons = ?assessment.reasons,
            "risk assessment"
        );

        if assessment.level == RiskLevel::Block && state.config.block_mode == BlockMode::Strict {
            state.metrics.record_blocked_strict();
            return Err(ApiError::TxBlocked(format!("{:?}", assessment.reasons)));
        }

        if state.config.jito_enabled {
            tracing::info!(level = ?assessment.level, "routing swap via Jito relay");
            match state.jito_sender.send_transaction(&tx).await {
                Ok(signature) => {
                    state.metrics.record_jito_routed();
                    tracing::info!(%signature, route = "jito", "transaction sent via Jito");
                    return Ok(JsonRpcResponse::success(
                        req.id.clone(),
                        Value::String(signature.to_string()),
                    ));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Jito relay failed, falling back to Direct RPC");
                }
            }
        }
    }

    let signature = state.rpc_sender.send_transaction(&tx, &user_config).await?;
    state.metrics.record_forwarded();

    tracing::info!(%signature, route = "direct", "transaction forwarded");

    Ok(JsonRpcResponse::success(
        req.id.clone(),
        Value::String(signature.to_string()),
    ))
}

/// Run the risk assessment pipeline against a swap-bearing transaction.
///
/// Pool identification is approximated by scoring every static account key and
/// taking the maximum — DEX-specific account-index decoding is deferred to a
/// later phase (see `risk::slippage` Phase B).
fn evaluate_risk(state: &AppState, tx: &solana_sdk::transaction::VersionedTransaction) -> RiskAssessment {
    let signer = signer_as_string(tx);
    let pool = extract_pool_address(tx);
    let pool_score = pool
        .as_deref()
        .map(|p| state.pool_map.score(p))
        .unwrap_or(0.0);

    let slippage = decode_slippage(tx);
    assess(
        pool_score,
        signer.as_deref(),
        &slippage,
        &state.attacker_set,
    )
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
