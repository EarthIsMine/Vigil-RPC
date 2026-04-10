use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    /// General RPC endpoint for read queries (getBalance, getLatestBlockhash, etc.)
    pub solana_rpc_url: String,
    /// Staked connection endpoint for sendTransaction (better landing rate)
    pub solana_send_tx_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let rpc_url = env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

        // Falls back to general RPC URL if staked endpoint not set
        let send_tx_url =
            env::var("SOLANA_SEND_TX_URL").unwrap_or_else(|_| rpc_url.clone());

        Self {
            port: env::var("PROTECTION_RPC_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8899),
            solana_rpc_url: rpc_url,
            solana_send_tx_url: send_tx_url,
        }
    }
}
