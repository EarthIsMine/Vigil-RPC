use std::num::NonZeroUsize;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use lru::LruCache;
use sandwich_detector::types::SandwichAttack;

#[derive(Debug, Clone)]
pub struct AttackerStats {
    pub attack_count: u32,
    pub last_seen_slot: u64,
    pub last_updated: Instant,
}

impl Default for AttackerStats {
    fn default() -> Self {
        Self {
            attack_count: 0,
            last_seen_slot: 0,
            last_updated: Instant::now(),
        }
    }
}

pub struct AttackerSet {
    inner: RwLock<LruCache<String, AttackerStats>>,
    ttl: Duration,
}

impl AttackerSet {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: RwLock::new(LruCache::new(cap)),
            ttl,
        }
    }

    pub fn record(&self, attack: &SandwichAttack) {
        let Ok(mut cache) = self.inner.write() else {
            return;
        };
        let entry = cache.get_or_insert_mut(attack.attacker.clone(), AttackerStats::default);
        entry.attack_count = entry.attack_count.saturating_add(1);
        entry.last_seen_slot = entry.last_seen_slot.max(attack.slot);
        entry.last_updated = Instant::now();
    }

    pub fn contains_active(&self, signer: &str) -> bool {
        let Ok(cache) = self.inner.read() else {
            return false;
        };
        match cache.peek(signer) {
            Some(s) => s.last_updated.elapsed() <= self.ttl,
            None => false,
        }
    }
}
