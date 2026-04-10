use axum::{routing::get, Router};

use super::handlers;

pub fn app_router() -> Router {
    Router::new().route("/health", get(handlers::health::health))
}
