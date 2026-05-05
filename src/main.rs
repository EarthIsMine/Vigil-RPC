mod analyzer;
mod config;
mod error;
mod metrics;
mod risk;
mod server;
mod slot_watcher;
mod state;
#[cfg(test)]
mod tests;
mod transmission;

use std::sync::Arc;
use std::time::Duration;

use config::Config;
use metrics::Metrics;
use risk::{AttackerSet, PoolRiskMap};
use state::AppState;
use transmission::jito_sender::JitoSender;
use transmission::rpc_forward::DirectRpcSender;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env();

    let rpc_sender = DirectRpcSender::new(&config.solana_send_tx_url);

    let pool_map = Arc::new(PoolRiskMap::new(
        config.pool_lru_capacity,
        Duration::from_secs(config.pool_ttl_secs),
    ));
    let attacker_set = Arc::new(AttackerSet::new(
        config.attacker_lru_capacity,
        Duration::from_secs(config.pool_ttl_secs),
    ));

    let http_client = reqwest::Client::new();
    let jito_sender = JitoSender::new(
        http_client.clone(),
        Some(config.jito_block_engine_url.clone()),
    );

    let state = AppState {
        config: Arc::new(config.clone()),
        rpc_sender: Arc::new(rpc_sender),
        jito_sender: Arc::new(jito_sender),
        http_client,
        pool_map,
        attacker_set,
        metrics: Arc::new(Metrics::new()),
    };

    slot_watcher::spawn_slot_watcher(state.clone());

    let app = server::routes::app_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Vigil Protection RPC listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
