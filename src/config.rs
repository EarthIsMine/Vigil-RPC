use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub solana_rpc_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PROTECTION_RPC_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8899),
            solana_rpc_url: env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
        }
    }
}
