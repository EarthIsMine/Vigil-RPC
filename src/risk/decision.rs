use crate::risk::attacker_set::AttackerSet;
use crate::risk::slippage::SlippageInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Safe,
    Warn,
    Block,
}

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub level: RiskLevel,
    pub reasons: Vec<String>,
    pub pool_score: f32,
}

const POOL_WARN_THRESHOLD: f32 = 0.25;
const POOL_BLOCK_THRESHOLD: f32 = 0.60;

pub fn assess(
    pool_score: f32,
    signer: Option<&str>,
    slippage: &SlippageInfo,
    attacker_set: &AttackerSet,
) -> RiskAssessment {
    let mut reasons: Vec<String> = Vec::new();
    let mut level = RiskLevel::Safe;

    if pool_score >= POOL_BLOCK_THRESHOLD {
        level = RiskLevel::Block;
        reasons.push(format!("pool_score={:.2}>=block_threshold", pool_score));
    } else if pool_score >= POOL_WARN_THRESHOLD {
        level = RiskLevel::Warn;
        reasons.push(format!("pool_score={:.2}>=warn_threshold", pool_score));
    }

    if let Some(s) = signer {
        if attacker_set.contains_active(s) {
            level = RiskLevel::Block;
            reasons.push("signer_in_attacker_set".to_string());
        }
    }

    if slippage.unbounded {
        if level == RiskLevel::Safe {
            level = RiskLevel::Warn;
        }
        reasons.push("slippage_unbounded".to_string());
    }

    RiskAssessment {
        level,
        reasons,
        pool_score,
    }
}
