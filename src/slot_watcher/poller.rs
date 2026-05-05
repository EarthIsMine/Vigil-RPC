use std::time::Duration;

use sandwich_detector::detector::detect_sandwiches;
use sandwich_detector::dex::{all_parsers, extract_swaps};
use sandwich_detector::parser::parse_block;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcBlockConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{TransactionDetails, UiTransactionEncoding};

use crate::state::AppState;

const MAX_CATCH_UP: u64 = 5;

pub async fn run(rpc_url: String, interval: Duration, lag: u64, state: AppState) {
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let parsers = all_parsers();
    let mut last_processed: Option<u64> = None;
    let mut ticker = tokio::time::interval(interval);

    tracing::info!(lag, ?interval, "slot watcher started");

    loop {
        ticker.tick().await;

        let head_slot = match client.get_slot().await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "slot watcher: getSlot failed");
                continue;
            }
        };

        let safe_head = head_slot.saturating_sub(lag);
        if safe_head == 0 {
            continue;
        }
        let start = last_processed.map(|l| l + 1).unwrap_or(safe_head);
        if start > safe_head {
            continue;
        }
        let end = safe_head.min(start + MAX_CATCH_UP - 1);

        for target in start..=end {
            let cfg = RpcBlockConfig {
                encoding: Some(UiTransactionEncoding::Json),
                transaction_details: Some(TransactionDetails::Full),
                rewards: Some(false),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            };

            let block = match client.get_block_with_config(target, cfg).await {
                Ok(b) => b,
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("Slot was skipped")
                        || err_str.contains("was skipped, or missing")
                        || err_str.contains("SlotSkipped")
                    {
                        tracing::debug!(slot = target, "slot skipped (no leader block)");
                    } else {
                        tracing::warn!(slot = target, error = %e, "slot watcher: getBlock failed");
                        state.metrics.record_slot(false, 0);
                        // Network error: stop catch-up, retry next tick
                        break;
                    }
                    last_processed = Some(target);
                    continue;
                }
            };

            let block_data = match parse_block(target, block) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(slot = target, error = %e, "slot watcher: parse_block failed");
                    state.metrics.record_slot(false, 0);
                    last_processed = Some(target);
                    continue;
                }
            };

            let mut all_swaps = Vec::new();
            for tx in &block_data.transactions {
                all_swaps.extend(extract_swaps(tx, &parsers));
            }

            let attacks = detect_sandwiches(target, &all_swaps);
            for attack in &attacks {
                state.pool_map.record_attack(attack);
                state.attacker_set.record(attack);
            }

            let watcher_lag = head_slot.saturating_sub(target);
            if attacks.is_empty() {
                tracing::debug!(
                    slot = target,
                    watcher_lag,
                    tx_count = block_data.transactions.len(),
                    swap_count = all_swaps.len(),
                    "slot processed (no attacks)"
                );
            } else {
                tracing::info!(
                    slot = target,
                    head_slot,
                    watcher_lag,
                    swap_count = all_swaps.len(),
                    attack_count = attacks.len(),
                    "sandwich detected in slot"
                );
            }
            state.metrics.record_slot(true, attacks.len() as u64);
            last_processed = Some(target);
        }
    }
}
