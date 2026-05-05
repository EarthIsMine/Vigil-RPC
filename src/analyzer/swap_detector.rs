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
    discriminator: Option<u8>,
}

const DEX_LAYOUTS: &[DexPoolLayout] = &[
    DexPoolLayout { program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", name: "Raydium V4", pool_account_index: Some(1), discriminator: Some(9) },
    DexPoolLayout { program_id: "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", name: "Raydium CLMM", pool_account_index: Some(2), discriminator: None },
    DexPoolLayout { program_id: "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C", name: "Raydium CPMM", pool_account_index: Some(3), discriminator: None },
    DexPoolLayout { program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc", name: "Orca Whirlpool", pool_account_index: Some(2), discriminator: None },
    DexPoolLayout { program_id: "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", name: "Jupiter V6", pool_account_index: None, discriminator: None },
    DexPoolLayout { program_id: "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo", name: "Meteora DLMM", pool_account_index: Some(0), discriminator: None },
    DexPoolLayout { program_id: "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P", name: "Pump.fun", pool_account_index: Some(2), discriminator: None },
    DexPoolLayout { program_id: "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY", name: "Phoenix", pool_account_index: Some(0), discriminator: None },
];

pub fn detect_swap(tx: &VersionedTransaction) -> Option<SwapDetection> {
    let account_keys = tx.message.static_account_keys();

    for ix in tx.message.instructions() {
        let Some(pid) = account_keys.get(ix.program_id_index as usize) else {
            continue;
        };
        let pid_str = pid.to_string();

        if let Some(layout) = DEX_LAYOUTS.iter().find(|l| l.program_id == pid_str) {
            return Some(SwapDetection {
                program_id: pid_str,
                dex_name: layout.name.to_string(),
            });
        }
    }

    None
}

/// Extract the pool/AMM address from the first matched DEX swap instruction.
/// Returns None for Jupiter (router) or if no DEX instruction found.
pub fn extract_pool_address(tx: &VersionedTransaction) -> Option<String> {
    let account_keys = tx.message.static_account_keys();

    for ix in tx.message.instructions() {
        let Some(pid) = account_keys.get(ix.program_id_index as usize) else {
            continue;
        };
        let pid = pid.to_string();

        let Some(layout) = DEX_LAYOUTS.iter().find(|l| l.program_id == pid) else {
            continue;
        };

        if let Some(disc) = layout.discriminator {
            if ix.data.first() != Some(&disc) {
                continue;
            }
        }

        let pool_idx = match layout.pool_account_index {
            Some(idx) => idx,
            None => continue,
        };
        let Some(&account_idx) = ix.accounts.get(pool_idx) else {
            continue;
        };
        let Some(pool) = account_keys.get(account_idx as usize) else {
            continue;
        };
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
    use super::*;

    #[test]
    fn dex_layouts_cover_major_dexes() {
        let names: Vec<&str> = DEX_LAYOUTS.iter().map(|l| l.name).collect();
        assert!(names.contains(&"Raydium V4"));
        assert!(names.contains(&"Pump.fun"));
        assert!(names.contains(&"Orca Whirlpool"));
        assert!(names.contains(&"Jupiter V6"));
        assert!(names.contains(&"Meteora DLMM"));
        assert_eq!(DEX_LAYOUTS.len(), 8);
    }
}
