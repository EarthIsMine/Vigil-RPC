use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::state::AppState;

pub async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "block_mode": format!("{:?}", state.config.block_mode),
        "metrics": state.metrics.snapshot(),
    }))
}
