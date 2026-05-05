use sandwich_detector::dex::all_parsers;
use solana_sdk::transaction::VersionedTransaction;

#[derive(Debug, Clone)]
pub struct SwapDetection {
    pub program_id: String,
    pub dex_name: String,
}

struct DexPoolLayout {
    program_id: &'static str,
    name: &'static str,
    pool_account_index: Option<usize>,
}

const DEX_LAYOUTS: &[DexPoolLayout] = &[
    DexPoolLayout { program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", name: "Raydium V4", pool_account_index: Some(1) },
    DexPoolLayout { program_id: "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", name: "Raydium CLMM", pool_account_index: Some(2) },
    DexPoolLayout { program_id: "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C", name: "Raydium CPMM", pool_account_index: Some(3) },
    DexPoolLayout { program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc", name: "Orca Whirlpool", pool_account_index: Some(2) },
    DexPoolLayout { program_id: "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", name: "Jupiter V6", pool_account_index: None },
    DexPoolLayout { program_id: "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo", name: "Meteora DLMM", pool_account_index: Some(0) },
    DexPoolLayout { program_id: "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P", name: "Pump.fun", pool_account_index: Some(2) },
    DexPoolLayout { program_id: "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY", name: "Phoenix", pool_account_index: Some(0) },
];

pub fn detect_swap(tx: &VersionedTransaction) -> Option<SwapDetection> {
    let parsers = all_parsers();
    let parser_pids: Vec<&str> = parsers.iter().map(|p| p.program_id()).collect();
    let account_keys = tx.message.static_account_keys();

    for ix in tx.message.instructions() {
        let pid = account_keys[ix.program_id_index as usize].to_string();

        if let Some(layout) = DEX_LAYOUTS.iter().find(|l| l.program_id == pid) {
            return Some(SwapDetection {
                program_id: pid,
                dex_name: layout.name.to_string(),
            });
        }

        if parser_pids.contains(&pid.as_str()) {
            return Some(SwapDetection {
                program_id: pid,
                dex_name: "Unknown DEX".to_string(),
            });
        }
    }

    None
}

/// Extract the pool/AMM address from the first matched DEX instruction.
/// Returns None for Jupiter (router — no single pool) or unrecognized DEX.
pub fn extract_pool_address(tx: &VersionedTransaction) -> Option<String> {
    let account_keys = tx.message.static_account_keys();

    for ix in tx.message.instructions() {
        let pid = account_keys.get(ix.program_id_index as usize)?.to_string();

        let layout = DEX_LAYOUTS.iter().find(|l| l.program_id == pid)?;
        let pool_idx = layout.pool_account_index?;
        let account_idx = *ix.accounts.get(pool_idx)? as usize;
        let pool = account_keys.get(account_idx)?;
        return Some(pool.to_string());
    }

    None
}

pub fn signer_as_string(tx: &VersionedTransaction) -> Option<String> {
    tx.message
        .static_account_keys()
        .first()
        .map(|k| k.to_string())
}

#[cfg(test)]
mod tests {
    use sandwich_detector::dex::all_parsers;

    #[test]
    fn known_dex_programs_loaded() {
        let parsers = all_parsers();
        assert!(parsers.len() >= 7);

        let pids: Vec<&str> = parsers.iter().map(|p| p.program_id()).collect();
        assert!(pids.contains(&"JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"));
        assert!(pids.contains(&"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"));
        assert!(pids.contains(&"whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"));
    }
}
