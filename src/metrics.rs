use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::risk::RiskLevel;

#[derive(Default)]
pub struct Metrics {
    pub risk_safe_total: AtomicU64,
    pub risk_warn_total: AtomicU64,
    pub risk_block_total: AtomicU64,
    pub blocked_strict_total: AtomicU64,
    pub forwarded_total: AtomicU64,
    pub slot_processed_total: AtomicU64,
    pub slot_failed_total: AtomicU64,
    pub sandwich_detected_total: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_risk(&self, level: RiskLevel) {
        let counter = match level {
            RiskLevel::Safe => &self.risk_safe_total,
            RiskLevel::Warn => &self.risk_warn_total,
            RiskLevel::Block => &self.risk_block_total,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_blocked_strict(&self) {
        self.blocked_strict_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_forwarded(&self) {
        self.forwarded_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_slot(&self, ok: bool, sandwiches: u64) {
        if ok {
            self.slot_processed_total.fetch_add(1, Ordering::Relaxed);
        } else {
            self.slot_failed_total.fetch_add(1, Ordering::Relaxed);
        }
        if sandwiches > 0 {
            self.sandwich_detected_total
                .fetch_add(sandwiches, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> Value {
        json!({
            "risk_safe_total": self.risk_safe_total.load(Ordering::Relaxed),
            "risk_warn_total": self.risk_warn_total.load(Ordering::Relaxed),
            "risk_block_total": self.risk_block_total.load(Ordering::Relaxed),
            "blocked_strict_total": self.blocked_strict_total.load(Ordering::Relaxed),
            "forwarded_total": self.forwarded_total.load(Ordering::Relaxed),
            "slot_processed_total": self.slot_processed_total.load(Ordering::Relaxed),
            "slot_failed_total": self.slot_failed_total.load(Ordering::Relaxed),
            "sandwich_detected_total": self.sandwich_detected_total.load(Ordering::Relaxed),
        })
    }
}
