//! Sandwich attack detection algorithm ported from solana-sandwich-detector.
//! Source: https://github.com/SangHyeonKwon/solana-sandwich-detector

use std::collections::{HashMap, HashSet};

use super::types::{SandwichAttack, SwapEvent};

/// Detect same-block sandwich attacks from a list of swap events.
///
/// Groups swaps by pool, then finds triplets (frontrun, victim, backrun) where:
/// - frontrun and backrun have the same signer (attacker)
/// - frontrun and backrun are opposite directions (buy then sell, or vice versa)
/// - victim is between them in transaction order
/// - victim has a different signer
/// - victim trades in the same direction as frontrun
pub fn detect_sandwiches(slot: u64, swaps: &[SwapEvent]) -> Vec<SandwichAttack> {
    let mut results = Vec::new();

    let mut by_pool: HashMap<&str, Vec<&SwapEvent>> = HashMap::new();
    for swap in swaps {
        by_pool.entry(&swap.pool).or_default().push(swap);
    }

    for (_pool, mut pool_swaps) in by_pool {
        pool_swaps.sort_by_key(|s| s.tx_index);

        if pool_swaps.len() < 3 {
            continue;
        }

        let mut consumed: HashSet<usize> = HashSet::new();

        for i in 0..pool_swaps.len() {
            let frontrun = pool_swaps[i];

            if consumed.contains(&frontrun.tx_index) {
                continue;
            }

            for k in (i + 2)..pool_swaps.len() {
                let backrun = pool_swaps[k];

                if consumed.contains(&backrun.tx_index) {
                    continue;
                }

                if frontrun.signer != backrun.signer {
                    continue;
                }
                if frontrun.direction == backrun.direction {
                    continue;
                }

                let mut found_victim = false;
                for &victim in pool_swaps.iter().take(k).skip(i + 1) {
                    if victim.signer == frontrun.signer {
                        continue;
                    }
                    if victim.direction != frontrun.direction {
                        continue;
                    }

                    results.push(SandwichAttack {
                        slot,
                        attacker: frontrun.signer.clone(),
                        frontrun: frontrun.clone(),
                        victim: victim.clone(),
                        backrun: backrun.clone(),
                        pool: frontrun.pool.clone(),
                        dex: frontrun.dex,
                        estimated_attacker_profit: estimate_profit(frontrun, backrun),
                        estimated_victim_loss: None,
                    });
                    found_victim = true;
                }

                if found_victim {
                    consumed.insert(frontrun.tx_index);
                    consumed.insert(backrun.tx_index);
                    break;
                }
            }
        }
    }

    results
}

fn estimate_profit(frontrun: &SwapEvent, backrun: &SwapEvent) -> Option<i64> {
    Some(backrun.amount_out as i64 - frontrun.amount_in as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::sandwich::types::{DexType, SwapDirection};

    fn swap(sig: &str, signer: &str, pool: &str, dir: SwapDirection, idx: usize) -> SwapEvent {
        SwapEvent {
            signature: sig.to_string(),
            signer: signer.to_string(),
            dex: DexType::RaydiumV4,
            pool: pool.to_string(),
            direction: dir,
            token_mint: "TokenMint111".to_string(),
            amount_in: 1_000_000,
            amount_out: 900_000,
            tx_index: idx,
        }
    }

    #[test]
    fn basic_sandwich() {
        let swaps = vec![
            swap("s1", "attacker", "pool1", SwapDirection::Buy, 0),
            swap("s2", "victim", "pool1", SwapDirection::Buy, 1),
            swap("s3", "attacker", "pool1", SwapDirection::Sell, 2),
        ];
        let res = detect_sandwiches(100, &swaps);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].attacker, "attacker");
        assert_eq!(res[0].victim.signature, "s2");
    }

    #[test]
    fn no_sandwich_same_signer() {
        let swaps = vec![
            swap("s1", "trader", "pool1", SwapDirection::Buy, 0),
            swap("s2", "trader", "pool1", SwapDirection::Buy, 1),
            swap("s3", "trader", "pool1", SwapDirection::Sell, 2),
        ];
        assert!(detect_sandwiches(100, &swaps).is_empty());
    }

    #[test]
    fn no_sandwich_same_direction() {
        let swaps = vec![
            swap("s1", "atk", "pool1", SwapDirection::Buy, 0),
            swap("s2", "vic", "pool1", SwapDirection::Buy, 1),
            swap("s3", "atk", "pool1", SwapDirection::Buy, 2),
        ];
        assert!(detect_sandwiches(100, &swaps).is_empty());
    }

    #[test]
    fn multiple_victims() {
        let swaps = vec![
            swap("s1", "atk", "pool1", SwapDirection::Buy, 0),
            swap("s2", "vic1", "pool1", SwapDirection::Buy, 1),
            swap("s3", "vic2", "pool1", SwapDirection::Buy, 2),
            swap("s4", "atk", "pool1", SwapDirection::Sell, 3),
        ];
        assert_eq!(detect_sandwiches(100, &swaps).len(), 2);
    }

    #[test]
    fn different_pools_no_sandwich() {
        let swaps = vec![
            swap("s1", "atk", "pool1", SwapDirection::Buy, 0),
            swap("s2", "vic", "pool2", SwapDirection::Buy, 1),
            swap("s3", "atk", "pool1", SwapDirection::Sell, 2),
        ];
        assert!(detect_sandwiches(100, &swaps).is_empty());
    }

    #[test]
    fn profit_calculation() {
        let mut front = swap("s1", "atk", "pool1", SwapDirection::Buy, 0);
        front.amount_in = 5_000_000_000;
        let victim = swap("s2", "vic", "pool1", SwapDirection::Buy, 1);
        let mut back = swap("s3", "atk", "pool1", SwapDirection::Sell, 2);
        back.amount_out = 5_200_000_000;

        let res = detect_sandwiches(100, &[front, victim, back]);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].estimated_attacker_profit, Some(200_000_000));
    }
}
