use std::sync::Arc;

use crate::config::Config;
use crate::transmission::rpc_forward::DirectRpcSender;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub rpc_sender: Arc<DirectRpcSender>,
    pub http_client: reqwest::Client,
}
