#[cfg(test)]
mod risk_pipeline {
    use std::time::Duration;

    use sandwich_detector::types::SandwichAttack;

    use crate::risk::attacker_set::AttackerSet;
    use crate::risk::decision::{assess, RiskLevel};
    use crate::risk::pool_map::PoolRiskMap;
    use crate::risk::slippage::SlippageInfo;

    fn attack(pool: &str, attacker: &str, slot: u64) -> SandwichAttack {
        let json = serde_json::json!({
            "slot": slot,
            "attacker": attacker,
            "frontrun": {
                "signature": "front", "signer": attacker,
                "dex": "raydium_v4", "pool": pool,
                "direction": "buy", "token_mint": "mint",
                "amount_in": 1000000, "amount_out": 900000, "tx_index": 0
            },
            "victim": {
                "signature": "victim", "signer": "victim_user",
                "dex": "raydium_v4", "pool": pool,
                "direction": "buy", "token_mint": "mint",
                "amount_in": 500000, "amount_out": 400000, "tx_index": 1
            },
            "backrun": {
                "signature": "back", "signer": attacker,
                "dex": "raydium_v4", "pool": pool,
                "direction": "sell", "token_mint": "mint",
                "amount_in": 900000, "amount_out": 1100000, "tx_index": 2
            },
            "pool": pool,
            "dex": "raydium_v4",
            "estimated_attacker_profit": 100000
        });
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn empty_risk_map_returns_safe() {
        let attacker_set = AttackerSet::new(100, Duration::from_secs(3600));
        let slippage = SlippageInfo::default();
        let result = assess(0.0, Some("user"), &slippage, &attacker_set);
        assert_eq!(result.level, RiskLevel::Safe);
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn pool_attacks_increase_score() {
        let pool_map = PoolRiskMap::new(100, Duration::from_secs(3600));
        let attacker_set = AttackerSet::new(100, Duration::from_secs(3600));

        for i in 0..4 {
            pool_map.record_attack(&attack("risky_pool", "atk", 100 + i));
        }

        let score = pool_map.score("risky_pool");
        assert!(score > 0.0);

        let result = assess(score, Some("user"), &SlippageInfo::default(), &attacker_set);
        assert_eq!(result.level, RiskLevel::Warn);
    }

    #[test]
    fn many_attacks_triggers_block() {
        let pool_map = PoolRiskMap::new(100, Duration::from_secs(3600));
        let attacker_set = AttackerSet::new(100, Duration::from_secs(3600));

        for i in 0..100 {
            pool_map.record_attack(&attack("danger", "atk", 100 + i));
        }

        let score = pool_map.score("danger");
        assert!(score >= 0.60);

        let result = assess(score, Some("user"), &SlippageInfo::default(), &attacker_set);
        assert_eq!(result.level, RiskLevel::Block);
    }

    #[test]
    fn known_attacker_triggers_block() {
        let attacker_set = AttackerSet::new(100, Duration::from_secs(3600));
        attacker_set.record(&attack("pool", "evil", 100));

        let result = assess(0.0, Some("evil"), &SlippageInfo::default(), &attacker_set);
        assert_eq!(result.level, RiskLevel::Block);
        assert!(result.reasons.iter().any(|r| r.contains("attacker_set")));
    }

    #[test]
    fn unbounded_slippage_triggers_warn() {
        let attacker_set = AttackerSet::new(100, Duration::from_secs(3600));
        let slippage = SlippageInfo { unbounded: true, ..Default::default() };

        let result = assess(0.0, Some("user"), &slippage, &attacker_set);
        assert_eq!(result.level, RiskLevel::Warn);
    }

    #[test]
    fn high_bps_triggers_block() {
        let attacker_set = AttackerSet::new(100, Duration::from_secs(3600));
        let slippage = SlippageInfo {
            slippage_bps: Some(2000),
            unbounded: true,
            ..Default::default()
        };

        let result = assess(0.0, Some("user"), &slippage, &attacker_set);
        assert_eq!(result.level, RiskLevel::Block);
        assert!(result.reasons.iter().any(|r| r.contains("slippage_bps")));
    }

    #[test]
    fn ttl_expiry_resets_score() {
        let pool_map = PoolRiskMap::new(100, Duration::from_millis(1));
        pool_map.record_attack(&attack("pool", "atk", 100));
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(pool_map.score("pool"), 0.0);
    }

    #[test]
    fn lru_eviction() {
        let pool_map = PoolRiskMap::new(2, Duration::from_secs(3600));
        pool_map.record_attack(&attack("a", "atk", 100));
        pool_map.record_attack(&attack("b", "atk", 101));
        pool_map.record_attack(&attack("c", "atk", 102));

        assert_eq!(pool_map.score("a"), 0.0, "pool_a evicted");
        assert!(pool_map.score("b") > 0.0);
        assert!(pool_map.score("c") > 0.0);
    }
}

#[cfg(test)]
mod http_integration {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use std::sync::Arc;
    use std::time::Duration;
    use tower::ServiceExt;

    use crate::config::Config;
    use crate::metrics::Metrics;
    use crate::risk::{AttackerSet, PoolRiskMap};
    use crate::server::routes::app_router;
    use crate::state::AppState;
    use crate::transmission::rpc_forward::DirectRpcSender;

    fn test_state() -> AppState {
        use crate::config::BlockMode;
        let config = Config {
            port: 0,
            solana_rpc_url: "http://127.0.0.1:1".to_string(),
            solana_send_tx_url: "http://127.0.0.1:1".to_string(),
            block_mode: BlockMode::Permissive,
            polling_interval_secs: 10,
            slot_watcher_lag: 32,
            pool_lru_capacity: 100,
            attacker_lru_capacity: 100,
            pool_ttl_secs: 3600,
        };
        AppState {
            config: Arc::new(config),
            rpc_sender: Arc::new(DirectRpcSender::new("http://127.0.0.1:1")),
            http_client: reqwest::Client::new(),
            pool_map: Arc::new(PoolRiskMap::new(100, Duration::from_secs(3600))),
            attacker_set: Arc::new(AttackerSet::new(100, Duration::from_secs(3600))),
            metrics: Arc::new(Metrics::new()),
        }
    }

    #[tokio::test]
    async fn health_returns_metrics() {
        let app = app_router(test_state());
        let resp = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["metrics"]["slot_processed_total"].is_number());
    }

    #[tokio::test]
    async fn non_send_tx_returns_rpc_error() {
        let app = app_router(test_state());
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getHealth", "params": []
        });
        let resp = app
            .oneshot(
                Request::post("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].is_object());
    }

    #[tokio::test]
    async fn sol_transfer_no_risk_assessment() {
        use base64::Engine;
        use solana_sdk::{hash::Hash, signature::Keypair, signer::Signer};
        use solana_system_transaction as system_transaction;

        let from = Keypair::new();
        let to = Keypair::new().pubkey();
        let tx = system_transaction::transfer(&from, &to, 1000, Hash::default());
        let bytes = bincode::serialize(&tx).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let state = test_state();
        let app = app_router(state.clone());
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "sendTransaction", "params": [encoded]
        });
        let resp = app
            .oneshot(
                Request::post("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        // SOL transfer → not a swap → no risk metrics
        assert!(json["error"].is_object(), "forward to dummy RPC fails");
        assert_eq!(
            state.metrics.risk_safe_total.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }
}
