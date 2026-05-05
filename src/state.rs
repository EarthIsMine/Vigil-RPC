use std::sync::Arc;

use crate::config::Config;
use crate::metrics::Metrics;
use crate::risk::{AttackerSet, PoolRiskMap};
use crate::transmission::jito_sender::JitoSender;
use crate::transmission::rpc_forward::DirectRpcSender;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub rpc_sender: Arc<DirectRpcSender>,
    pub jito_sender: Arc<JitoSender>,
    pub http_client: reqwest::Client,
    pub pool_map: Arc<PoolRiskMap>,
    pub attacker_set: Arc<AttackerSet>,
    pub metrics: Arc<Metrics>,
}
