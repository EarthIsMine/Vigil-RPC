use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PROTECTION_RPC_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8899),
        }
    }
}
