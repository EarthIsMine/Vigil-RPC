use axum::{routing::{get, post}, Router};

use crate::state::AppState;

use super::handlers;

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/", post(handlers::rpc::handle_rpc))
        .with_state(state)
}
