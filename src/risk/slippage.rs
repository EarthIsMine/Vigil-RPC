use solana_sdk::transaction::VersionedTransaction;

const RAYDIUM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const ORCA_SWAP_DISC: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

#[derive(Debug, Clone, Default)]
pub struct SlippageInfo {
    pub min_amount_out: Option<u64>,
    pub max_amount_in: Option<u64>,
    pub allowed_pct: Option<f32>,
    pub unbounded: bool,
}

/// Best-effort slippage decode from outer instructions.
///
/// Phase A: only the simplest, well-documented instruction layouts —
/// Raydium V4 (SwapBaseIn/SwapBaseOut) and Orca Whirlpool (swap). Jupiter V6
/// is a router whose route bytes encode each hop separately and is left for
/// Phase B. CPI'd swaps from inside Jupiter aren't visible here either.
pub fn decode_slippage(tx: &VersionedTransaction) -> SlippageInfo {
    let mut info = SlippageInfo::default();
    let account_keys = tx.message.static_account_keys();

    for ix in tx.message.instructions() {
        let Some(pid) = account_keys.get(ix.program_id_index as usize) else {
            continue;
        };
        let pid_str = pid.to_string();
        let parsed = match pid_str.as_str() {
            RAYDIUM_V4_PROGRAM_ID => decode_raydium_v4(&ix.data),
            ORCA_WHIRLPOOL_PROGRAM_ID => decode_orca_whirlpool(&ix.data),
            _ => None,
        };
        if let Some(parsed) = parsed {
            merge(&mut info, parsed);
        }
    }

    info
}

fn merge(info: &mut SlippageInfo, parsed: SlippageInfo) {
    if parsed.unbounded {
        info.unbounded = true;
    }
    if let Some(v) = parsed.min_amount_out {
        info.min_amount_out = Some(v);
    }
    if let Some(v) = parsed.max_amount_in {
        info.max_amount_in = Some(v);
    }
}

fn decode_raydium_v4(data: &[u8]) -> Option<SlippageInfo> {
    if data.len() < 17 {
        return None;
    }
    match data[0] {
        // SwapBaseIn: disc(1) + amount_in(8) + min_amount_out(8)
        9 => {
            let min_out = u64::from_le_bytes(data[9..17].try_into().ok()?);
            Some(SlippageInfo {
                min_amount_out: Some(min_out),
                unbounded: min_out == 0,
                ..Default::default()
            })
        }
        // SwapBaseOut: disc(1) + max_amount_in(8) + amount_out(8)
        11 => {
            let max_in = u64::from_le_bytes(data[1..9].try_into().ok()?);
            Some(SlippageInfo {
                max_amount_in: Some(max_in),
                unbounded: max_in == u64::MAX,
                ..Default::default()
            })
        }
        _ => None,
    }
}

fn decode_orca_whirlpool(data: &[u8]) -> Option<SlippageInfo> {
    // Anchor: disc(8) + amount(8) + other_amount_threshold(8)
    //         + sqrt_price_limit(16) + amount_specified_is_input(1) + a_to_b(1)
    if data.len() < 42 || data[..8] != ORCA_SWAP_DISC {
        return None;
    }
    let threshold = u64::from_le_bytes(data[16..24].try_into().ok()?);
    let amount_specified_is_input = data[40] != 0;
    if amount_specified_is_input {
        Some(SlippageInfo {
            min_amount_out: Some(threshold),
            unbounded: threshold == 0,
            ..Default::default()
        })
    } else {
        Some(SlippageInfo {
            max_amount_in: Some(threshold),
            unbounded: threshold == u64::MAX,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raydium_swap_base_in_zero_min() {
        let mut data = vec![9u8];
        data.extend_from_slice(&1_000_000u64.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());
        let info = decode_raydium_v4(&data).unwrap();
        assert_eq!(info.min_amount_out, Some(0));
        assert!(info.unbounded);
    }

    #[test]
    fn raydium_swap_base_in_with_min() {
        let mut data = vec![9u8];
        data.extend_from_slice(&1_000_000u64.to_le_bytes());
        data.extend_from_slice(&950_000u64.to_le_bytes());
        let info = decode_raydium_v4(&data).unwrap();
        assert_eq!(info.min_amount_out, Some(950_000));
        assert!(!info.unbounded);
    }

    #[test]
    fn orca_swap_exact_in_zero_min() {
        let mut data = ORCA_SWAP_DISC.to_vec();
        data.extend_from_slice(&1_000_000u64.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&0u128.to_le_bytes());
        data.push(1); // amount_specified_is_input = true
        data.push(1); // a_to_b
        let info = decode_orca_whirlpool(&data).unwrap();
        assert_eq!(info.min_amount_out, Some(0));
        assert!(info.unbounded);
    }
}
