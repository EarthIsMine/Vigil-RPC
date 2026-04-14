//! Sandwich attack detection module — ported from solana-sandwich-detector.
//!
//! Source: https://github.com/SangHyeonKwon/solana-sandwich-detector
//!
//! This module contains the version-agnostic core of sandwich-detector:
//! types, DEX program IDs, and the detection algorithm.
//! When sandwich-detector upgrades to solana-sdk 2.x, this can be replaced
//! by a direct crate dependency.

pub mod detector;
pub mod types;

// Re-export commonly used items
pub use detector::detect_sandwiches;
pub use types::{DexType, SandwichAttack, SwapDirection, SwapEvent};

/// Known DEX program IDs from sandwich-detector's parser registry.
pub const RAYDIUM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const JUPITER_V6_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

/// Additional DEX program ID from Tier 2 spec (not yet in sandwich-detector).
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

/// All known DEX program IDs with their display names.
pub const KNOWN_DEX_PROGRAMS: &[(&str, &str)] = &[
    (JUPITER_V6_PROGRAM_ID, "Jupiter V6"),
    (RAYDIUM_V4_PROGRAM_ID, "Raydium V4"),
    (ORCA_WHIRLPOOL_PROGRAM_ID, "Orca Whirlpool"),
    (METEORA_DLMM_PROGRAM_ID, "Meteora DLMM"),
];
