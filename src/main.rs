mod analyzer;
mod config;
mod error;
mod server;
mod state;
mod transmission;

use std::sync::Arc;

use config::Config;
use state::AppState;
use transmission::rpc_forward::DirectRpcSender;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env();

    let rpc_sender = DirectRpcSender::new(&config.solana_rpc_url);

    let state = AppState {
        config: Arc::new(config.clone()),
        rpc_sender: Arc::new(rpc_sender),
        http_client: reqwest::Client::new(),
    };

    let app = server::routes::app_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Vigil Protection RPC listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
