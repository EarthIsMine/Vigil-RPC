use std::env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockMode {
    /// BLOCK level downgrades to WARN behavior (forwards anyway).
    Permissive,
    /// BLOCK level returns a JSON-RPC error to the client.
    Strict,
}

impl BlockMode {
    fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "strict" => Self::Strict,
            _ => Self::Permissive,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    /// General RPC endpoint for read queries (getBalance, getLatestBlockhash, etc.)
    pub solana_rpc_url: String,
    /// Staked connection endpoint for sendTransaction (better landing rate)
    pub solana_send_tx_url: String,
    /// How risk BLOCK decisions are enforced.
    pub block_mode: BlockMode,
    /// Slot watcher polling interval in seconds.
    pub polling_interval_secs: u64,
    /// Number of slots to lag behind the head when polling (for finalization).
    pub slot_watcher_lag: u64,
    /// Pool risk map LRU capacity.
    pub pool_lru_capacity: usize,
    /// Attacker set LRU capacity.
    pub attacker_lru_capacity: usize,
    /// TTL (seconds) before pool/attacker entries are considered stale.
    pub pool_ttl_secs: u64,
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
            block_mode: env::var("BLOCK_MODE")
                .map(|v| BlockMode::from_str(&v))
                .unwrap_or(BlockMode::Permissive),
            polling_interval_secs: env::var("SLOT_POLL_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            slot_watcher_lag: env::var("SLOT_WATCHER_LAG")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(32),
            pool_lru_capacity: env::var("POOL_LRU_CAPACITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1024),
            attacker_lru_capacity: env::var("ATTACKER_LRU_CAPACITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1024),
            pool_ttl_secs: env::var("POOL_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60 * 60 * 24),
        }
    }
}
