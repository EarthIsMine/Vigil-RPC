use solana_sdk::transaction::VersionedTransaction;

const RAYDIUM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const JUPITER_V6_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

const ORCA_SWAP_DISC: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const PUMPFUN_BUY_DISC: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
const PUMPFUN_SELL_DISC: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

/// 5% in basis points — above this, slippage is considered dangerously high.
const HIGH_SLIPPAGE_BPS: u16 = 500;

#[derive(Debug, Clone, Default)]
pub struct SlippageInfo {
    pub min_amount_out: Option<u64>,
    pub max_amount_in: Option<u64>,
    pub slippage_bps: Option<u16>,
    pub unbounded: bool,
}

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
            JUPITER_V6_PROGRAM_ID => decode_jupiter_v6(&ix.data),
            PUMPFUN_PROGRAM_ID => decode_pumpfun(&ix.data),
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
        let prev = info.min_amount_out.unwrap_or(u64::MAX);
        info.min_amount_out = Some(prev.min(v));
    }
    if let Some(v) = parsed.max_amount_in {
        let prev = info.max_amount_in.unwrap_or(0);
        info.max_amount_in = Some(prev.max(v));
    }
    if let Some(v) = parsed.slippage_bps {
        let prev = info.slippage_bps.unwrap_or(0);
        info.slippage_bps = Some(prev.max(v));
    }
}

fn decode_raydium_v4(data: &[u8]) -> Option<SlippageInfo> {
    if data.len() < 17 {
        return None;
    }
    match data[0] {
        9 => {
            let min_out = u64::from_le_bytes(data[9..17].try_into().ok()?);
            Some(SlippageInfo {
                min_amount_out: Some(min_out),
                unbounded: min_out == 0,
                ..Default::default()
            })
        }
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

/// Jupiter V6 Anchor discriminators for swap instructions.
const JUP_ROUTE_DISC: [u8; 8] = [229, 23, 203, 151, 122, 227, 173, 42];
const JUP_SHARED_ACCOUNTS_ROUTE_DISC: [u8; 8] = [193, 32, 155, 51, 65, 214, 156, 129];
const JUP_SHARED_EXACT_OUT_DISC: [u8; 8] = [176, 209, 105, 168, 154, 125, 69, 62];

/// Jupiter V6 route/shared_accounts_route: last 3 bytes are always
/// `slippage_bps: u16` + `platform_fee_bps: u8` (borsh, LE).
/// Only decoded for known swap discriminators to avoid false positives.
fn decode_jupiter_v6(data: &[u8]) -> Option<SlippageInfo> {
    if data.len() < 24 {
        return None;
    }
    let disc: [u8; 8] = data[..8].try_into().ok()?;
    if disc != JUP_ROUTE_DISC
        && disc != JUP_SHARED_ACCOUNTS_ROUTE_DISC
        && disc != JUP_SHARED_EXACT_OUT_DISC
    {
        return None;
    }
    let bps = u16::from_le_bytes(data[data.len() - 3..data.len() - 1].try_into().ok()?);
    Some(SlippageInfo {
        slippage_bps: Some(bps),
        unbounded: bps > HIGH_SLIPPAGE_BPS,
        ..Default::default()
    })
}

/// Pump.fun buy: disc(8) + amount(8) + max_sol_cost(8)
/// Pump.fun sell: disc(8) + amount(8) + min_sol_output(8)
fn decode_pumpfun(data: &[u8]) -> Option<SlippageInfo> {
    if data.len() < 24 {
        return None;
    }
    if data[..8] == PUMPFUN_BUY_DISC {
        let max_sol = u64::from_le_bytes(data[16..24].try_into().ok()?);
        Some(SlippageInfo {
            max_amount_in: Some(max_sol),
            unbounded: max_sol == u64::MAX,
            ..Default::default()
        })
    } else if data[..8] == PUMPFUN_SELL_DISC {
        let min_sol = u64::from_le_bytes(data[16..24].try_into().ok()?);
        Some(SlippageInfo {
            min_amount_out: Some(min_sol),
            unbounded: min_sol == 0,
            ..Default::default()
        })
    } else {
        None
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
        data.push(1);
        data.push(1);
        let info = decode_orca_whirlpool(&data).unwrap();
        assert_eq!(info.min_amount_out, Some(0));
        assert!(info.unbounded);
    }

    #[test]
    fn jupiter_low_slippage() {
        let mut data = JUP_SHARED_ACCOUNTS_ROUTE_DISC.to_vec();
        data.extend_from_slice(&[0u8; 19]); // padding to 30 bytes total
        let len = data.len();
        data[len - 3] = 50; // 50 bps = 0.5%
        data[len - 2] = 0;
        data[len - 1] = 0; // platform_fee_bps
        let info = decode_jupiter_v6(&data).unwrap();
        assert_eq!(info.slippage_bps, Some(50));
        assert!(!info.unbounded);
    }

    #[test]
    fn jupiter_high_slippage() {
        let mut data = JUP_ROUTE_DISC.to_vec();
        data.extend_from_slice(&[0u8; 19]);
        let bps: u16 = 1000; // 10%
        let bps_bytes = bps.to_le_bytes();
        let len = data.len();
        data[len - 3] = bps_bytes[0];
        data[len - 2] = bps_bytes[1];
        data[len - 1] = 0;
        let info = decode_jupiter_v6(&data).unwrap();
        assert_eq!(info.slippage_bps, Some(1000));
        assert!(info.unbounded);
    }

    #[test]
    fn pumpfun_buy_unbounded() {
        let mut data = PUMPFUN_BUY_DISC.to_vec();
        data.extend_from_slice(&100_000u64.to_le_bytes());
        data.extend_from_slice(&u64::MAX.to_le_bytes());
        let info = decode_pumpfun(&data).unwrap();
        assert!(info.unbounded);
        assert_eq!(info.max_amount_in, Some(u64::MAX));
    }

    #[test]
    fn pumpfun_sell_zero_min() {
        let mut data = PUMPFUN_SELL_DISC.to_vec();
        data.extend_from_slice(&100_000u64.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());
        let info = decode_pumpfun(&data).unwrap();
        assert!(info.unbounded);
        assert_eq!(info.min_amount_out, Some(0));
    }

    #[test]
    fn pumpfun_sell_with_min() {
        let mut data = PUMPFUN_SELL_DISC.to_vec();
        data.extend_from_slice(&100_000u64.to_le_bytes());
        data.extend_from_slice(&90_000u64.to_le_bytes());
        let info = decode_pumpfun(&data).unwrap();
        assert!(!info.unbounded);
        assert_eq!(info.min_amount_out, Some(90_000));
    }
}
