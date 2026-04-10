mod analyzer;
mod config;
mod error;
mod server;

use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env();
    let app = server::routes::app_router();

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Vigil Protection RPC listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
