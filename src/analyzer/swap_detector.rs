use sandwich_detector::dex::all_parsers;
use solana_sdk::transaction::VersionedTransaction;

/// Additional DEX program IDs not yet in sandwich-detector
const EXTRA_DEX_PROGRAMS: &[(&str, &str)] = &[(
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
    "Meteora DLMM",
)];

/// Result of swap detection on a single incoming transaction
#[derive(Debug, Clone)]
pub struct SwapDetection {
    /// Matched DEX program ID
    pub program_id: String,
    /// Human-readable DEX name
    pub dex_name: String,
}

/// All static account keys of a transaction as base58 strings.
/// Used for risk lookup since the precise pool account depends on DEX-specific
/// instruction layout — checking every candidate is a safe over-approximation.
pub fn account_keys_as_strings(tx: &VersionedTransaction) -> Vec<String> {
    tx.message
        .static_account_keys()
        .iter()
        .map(|k| k.to_string())
        .collect()
}

/// Fee payer (first signer) as a base58 string, if present.
pub fn signer_as_string(tx: &VersionedTransaction) -> Option<String> {
    tx.message
        .static_account_keys()
        .first()
        .map(|k| k.to_string())
}

/// Detect if a decoded transaction contains a DEX swap instruction.
///
/// Checks the transaction's instruction program IDs against:
/// - sandwich-detector's parser registry (Raydium V4, Orca Whirlpool, Jupiter V6)
/// - Additional known DEX programs (Meteora DLMM)
pub fn detect_swap(tx: &VersionedTransaction) -> Option<SwapDetection> {
    let parsers = all_parsers();
    let known_dexes: Vec<(&str, &str)> = parsers
        .iter()
        .map(|p| {
            let pid = p.program_id();
            let name = match pid {
                "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => "Jupiter V6",
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium V4",
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "Orca Whirlpool",
                _ => "Unknown DEX",
            };
            (pid, name)
        })
        .chain(EXTRA_DEX_PROGRAMS.iter().copied())
        .collect();

    let account_keys = tx.message.static_account_keys();

    for ix in tx.message.instructions() {
        let pid = account_keys[ix.program_id_index as usize].to_string();
        if let Some(&(_, name)) = known_dexes.iter().find(|&&(dex_pid, _)| dex_pid == pid) {
            return Some(SwapDetection {
                program_id: pid,
                dex_name: name.to_string(),
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use sandwich_detector::dex::all_parsers;

    #[test]
    fn known_dex_programs_loaded() {
        let parsers = all_parsers();
        assert!(parsers.len() >= 3);

        let pids: Vec<&str> = parsers.iter().map(|p| p.program_id()).collect();
        assert!(pids.contains(&"JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"));
        assert!(pids.contains(&"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"));
        assert!(pids.contains(&"whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"));
    }
}
