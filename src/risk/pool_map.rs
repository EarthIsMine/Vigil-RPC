use std::num::NonZeroUsize;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use lru::LruCache;
use sandwich_detector::types::SandwichAttack;

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub recent_sandwich_count: u32,
    pub total_extracted_lamports: u64,
    pub last_seen_slot: u64,
    pub last_updated: Instant,
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            recent_sandwich_count: 0,
            total_extracted_lamports: 0,
            last_seen_slot: 0,
            last_updated: Instant::now(),
        }
    }
}

pub struct PoolRiskMap {
    inner: RwLock<LruCache<String, PoolStats>>,
    ttl: Duration,
}

impl PoolRiskMap {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: RwLock::new(LruCache::new(cap)),
            ttl,
        }
    }

    pub fn record_attack(&self, attack: &SandwichAttack) {
        let mut cache = self.inner.write().unwrap();
        let entry = cache.get_or_insert_mut(attack.pool.clone(), PoolStats::default);
        entry.recent_sandwich_count = entry.recent_sandwich_count.saturating_add(1);
        if let Some(profit) = attack.estimated_attacker_profit {
            if profit > 0 {
                entry.total_extracted_lamports =
                    entry.total_extracted_lamports.saturating_add(profit as u64);
            }
        }
        entry.last_seen_slot = entry.last_seen_slot.max(attack.slot);
        entry.last_updated = Instant::now();
    }

    /// Pool risk score in [0.0, 1.0]. Higher = more dangerous.
    /// Currently log-scaled by recent_sandwich_count.
    pub fn score(&self, pool: &str) -> f32 {
        let cache = self.inner.read().unwrap();
        let stats = match cache.peek(pool) {
            Some(s) => s,
            None => return 0.0,
        };
        if stats.last_updated.elapsed() > self.ttl {
            return 0.0;
        }
        let count = stats.recent_sandwich_count as f32;
        (1.0 + count).log2() / 8.0_f32
    }

    #[allow(dead_code)]
    pub fn snapshot_top(&self, n: usize) -> Vec<(String, PoolStats)> {
        let cache = self.inner.read().unwrap();
        cache
            .iter()
            .take(n)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}
