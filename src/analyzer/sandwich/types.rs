//! Types ported from solana-sandwich-detector.
//! Source: https://github.com/SangHyeonKwon/solana-sandwich-detector
//!
//! These types are Solana-SDK-agnostic (all String/u64 fields), so they can
//! live here until sandwich-detector upgrades to solana-sdk 2.x, at which
//! point this module can be replaced by a crate dependency.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Supported DEX protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DexType {
    RaydiumV4,
    OrcaWhirlpool,
    JupiterV6,
}

impl fmt::Display for DexType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DexType::RaydiumV4 => write!(f, "Raydium V4"),
            DexType::OrcaWhirlpool => write!(f, "Orca Whirlpool"),
            DexType::JupiterV6 => write!(f, "Jupiter V6"),
        }
    }
}

/// Direction of a token swap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwapDirection {
    Buy,
    Sell,
}

impl SwapDirection {
    pub fn opposite(&self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

/// A single swap event extracted from a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
    pub signature: String,
    pub signer: String,
    pub dex: DexType,
    pub pool: String,
    pub direction: SwapDirection,
    pub token_mint: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub tx_index: usize,
}

/// A detected sandwich attack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandwichAttack {
    pub slot: u64,
    pub attacker: String,
    pub frontrun: SwapEvent,
    pub victim: SwapEvent,
    pub backrun: SwapEvent,
    pub pool: String,
    pub dex: DexType,
    pub estimated_attacker_profit: Option<i64>,
    pub estimated_victim_loss: Option<i64>,
}
