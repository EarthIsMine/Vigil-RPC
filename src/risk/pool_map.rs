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
        let Ok(mut cache) = self.inner.write() else {
            return;
        };
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

    /// Find the highest risk score among a set of candidate pool addresses.
    /// Takes a single read lock for the entire batch to minimize contention.
    pub fn best_score(&self, candidates: &[String]) -> (Option<String>, f32) {
        let Ok(cache) = self.inner.read() else {
            return (None, 0.0);
        };
        let mut best_pool = None;
        let mut best_score = 0.0_f32;
        for cand in candidates {
            if let Some(stats) = cache.peek(cand) {
                if stats.last_updated.elapsed() <= self.ttl {
                    let score = Self::compute_score(stats.recent_sandwich_count);
                    if score > best_score {
                        best_score = score;
                        best_pool = Some(cand.clone());
                    }
                }
            }
        }
        (best_pool, best_score)
    }

    fn compute_score(count: u32) -> f32 {
        ((1.0 + count as f32).log2() / 8.0).min(1.0)
    }
}
