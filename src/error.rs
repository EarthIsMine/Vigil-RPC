use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;

use crate::server::rpc_types::{JsonRpcError, JsonRpcResponse};

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("transaction decode failed: {0}")]
    TxDecodeFailed(String),

    #[error("RPC forward failed: {0}")]
    RpcForwardFailed(String),

    #[error("transaction blocked by protection policy: {0}")]
    TxBlocked(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    fn error_code(&self) -> i64 {
        match self {
            Self::InvalidRequest(_) => -32600,
            Self::TxDecodeFailed(_) => -32602,
            Self::RpcForwardFailed(_) => -32603,
            Self::TxBlocked(_) => -32004,
            Self::Internal(_) => -32603,
        }
    }

    pub fn into_rpc_response(self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::error(
            id,
            JsonRpcError {
                code: self.error_code(),
                message: self.to_string(),
                data: None,
            },
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::TxDecodeFailed(_) => StatusCode::BAD_REQUEST,
            Self::RpcForwardFailed(_) => StatusCode::BAD_GATEWAY,
            Self::TxBlocked(_) => StatusCode::FORBIDDEN,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = self.into_rpc_response(Value::Null);
        (status, Json(body)).into_response()
    }
}
