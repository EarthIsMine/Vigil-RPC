use axum::{routing::get, Router};

use crate::state::AppState;

use super::handlers;

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health::health))
        .with_state(state)
}
