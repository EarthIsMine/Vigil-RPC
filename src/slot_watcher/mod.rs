pub mod poller;

use std::time::Duration;

use crate::state::AppState;

pub fn spawn_slot_watcher(state: AppState) {
    let interval = Duration::from_secs(state.config.polling_interval_secs);
    let lag = state.config.slot_watcher_lag;
    let rpc_url = state.config.solana_rpc_url.clone();
    tokio::spawn(async move {
        poller::run(rpc_url, interval, lag, state).await;
    });
}
