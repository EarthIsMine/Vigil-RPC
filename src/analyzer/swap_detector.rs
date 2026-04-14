use solana_sdk::transaction::VersionedTransaction;

use super::sandwich::{DexType, KNOWN_DEX_PROGRAMS};

// Re-export sandwich-detector types for use across vigil-rpc
pub use super::sandwich::{detect_sandwiches, SandwichAttack, SwapDirection, SwapEvent};

/// Result of swap detection on a single incoming transaction
#[derive(Debug, Clone)]
pub struct SwapDetection {
    /// Matched DEX program ID
    pub program_id: String,
    /// Human-readable DEX name
    pub dex_name: String,
    /// Resolved DexType if the program is in sandwich-detector's registry
    pub dex_type: Option<DexType>,
}

/// Detect if a decoded transaction contains a DEX swap instruction.
///
/// Checks the transaction's instruction program IDs against known DEX programs
/// from the sandwich-detector registry (Raydium V4, Orca Whirlpool, Jupiter V6)
/// plus Meteora DLMM from the Tier 2 spec.
pub fn detect_swap(tx: &VersionedTransaction) -> Option<SwapDetection> {
    let account_keys = tx.message.static_account_keys();

    for ix in tx.message.instructions() {
        let pid = account_keys[ix.program_id_index as usize].to_string();

        if let Some(&(_, name)) = KNOWN_DEX_PROGRAMS.iter().find(|&&(id, _)| id == pid) {
            let dex_type = match pid.as_str() {
                "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => Some(DexType::JupiterV6),
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => Some(DexType::RaydiumV4),
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => Some(DexType::OrcaWhirlpool),
                _ => None,
            };
            return Some(SwapDetection {
                program_id: pid,
                dex_name: name.to_string(),
                dex_type,
            });
        }
    }

    None
}
