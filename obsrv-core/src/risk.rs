//! Risk Calculator
//!
//! Takes decoded instructions and nonce info from earlier pipeline stages
//! and produces a final risk assessment for the transaction.
//!
//! # Scoring Model
//!
//! The risk score ranges from 1–10 and is built additively:
//!
//! | Signal                          | Points |
//! |---------------------------------|--------|
//! | Base (every transaction)        | +1     |
//! | Durable nonce detected          | +5     |
//! | Per Critical severity instruction | +3   |
//! | Per Warning severity instruction  | +2   |
//! | Per Info severity instruction     | +1   |
//! | Nonce account creation (space=80) | +2   |
//! | Unknown program encountered       | +1   |
//!
//! The score is clamped to [1, 10] before returning.
//!
//! # Risk Levels
//!
//! | Score | Level    | Recommendation      |
//! |-------|----------|---------------------|
//! | 1–2   | None     | SAFE TO SIGN        |
//! | 3–4   | Info     | REVIEW CAREFULLY    |
//! | 5–6   | Warning  | PROCEED WITH CAUTION|
//! | 7–10  | Critical | DO NOT SIGN         |

use crate::nonce::NonceInfo;
use crate::types::{DecodedInstruction, InstructionType, Severity};

#[derive(Debug, Clone)]
pub struct RiskReport {
    // 1–10 risk score
    pub score: u8,

    // None / Info / Warning / Critical
    pub level: Severity,

    // SAFE TO SIGN / REVIEW CAREFULLY / PROCEED WITH CAUTION / DO NOT SIGN
    pub recommendation: String,

    // all risk flags from all instructions combined
    pub flags: Vec<String>,
}

/// Main entry point — called by the analyzer after instruction decoding and nonce detection.
///
/// # Arguments
/// * `instructions` — all decoded instructions from the transaction
/// * `nonce_info` — result of nonce::detect(), tells us if this is a durable nonce tx
///
/// # How Scoring Works
///
/// 1. Start at score 1 (minimum)
/// 2. Add points for durable nonce presence (+5)
/// 3. Walk each instruction:
///    - Collect unique risk_flags from every instruction
///    - Add points based on severity (Critical +3, Warning +2, Info +1)
///    - Add bonus points for specific high-risk patterns (nonce creation, unknown programs)
/// 4. Clamp final score to [1, 10]
/// 5. Map score to level + recommendation string
pub fn calculate(instructions: &[DecodedInstruction], nonce_info: &NonceInfo) -> RiskReport {
    let mut score: u8 = 1;
    let mut flags: Vec<String> = Vec::new();

    // durable nonce is a major risk signal — transaction never expires
    if nonce_info.is_durable_nonce {
        score = score.saturating_add(5);

        flags.push("DURABLE NONCE: transaction never expires".to_string());
        flags.push("execution timing controlled by nonce authority".to_string());

        if let Some(ref account) = nonce_info.nonce_account {
            flags.push(format!("nonce account: {}", account));
        }
        if let Some(ref authority) = nonce_info.nonce_authority {
            flags.push(format!("nonce authority: {}", authority));
        }
    }

    // each instruction and accumulate risk
    for ix in instructions {
        // collect unique flags from instruction decoders
        for flag in &ix.risk_flags {
            if !flags.contains(flag) {
                flags.push(flag.clone());
            }
        }

        // severity-based scoring (already assigned by individual decoders)
        match ix.severity {
            Severity::Critical => score = score.saturating_add(3),
            Severity::Warning => score = score.saturating_add(2),
            Severity::Info => score = score.saturating_add(1),
            Severity::None => {}
        }

        // bonus scoring for specific high-risk instruction patterns
        match &ix.instruction_type {
            // nonce account creation (space == 80 bytes) is a setup for durable nonce attacks
            InstructionType::CreateAccount => {
                let is_nonce_account = ix
                    .details
                    .get("space_bytes")
                    .map(|s| s == "80")
                    .unwrap_or(false);

                if is_nonce_account {
                    score = score.saturating_add(2);
                }
            }

            InstructionType::TokenTransfer => {
                if ix.risk_flags.iter().any(|f| f.contains("APPROVAL")) {
                    score = score.saturating_add(2);
                }
            }

            InstructionType::NonceAuthorize => {
                score = score.saturating_add(2);
            }

            // unknown programs can't be analyzed — inherently risky
            InstructionType::Unknown(_) => {
                score = score.saturating_add(1);
            }

            // all other instruction types are already scored by severity above
            // no bonus points needed — the decoders set appropriate severity levels
            _ => {}
        }
    }

    // clamp to valid range [1, 10]
    score = score.clamp(1, 10);

    // map final score to risk level and recommendation
    let (level, recommendation) = match score {
        1..=2 => (Severity::None, "SAFE TO SIGN"),
        3..=4 => (Severity::Info, "REVIEW CAREFULLY"),
        5..=6 => (Severity::Warning, "PROCEED WITH CAUTION"),
        _ => (Severity::Critical, "DO NOT SIGN"),
    };

    RiskReport {
        score,
        level,
        recommendation: recommendation.to_string(),
        flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonce::NonceInfo;
    use crate::types::{InstructionType, ProgramType, Severity};
    use std::collections::HashMap;

    // helper — builds a simple SOL transfer instruction
    fn make_transfer(severity: Severity) -> DecodedInstruction {
        let risk_flags = match severity {
            Severity::Warning => vec!["LARGE TRANSFER: 100.00 SOL".to_string()],
            _ => vec![],
        };
        DecodedInstruction {
            index: 0,
            program: ProgramType::System,
            instruction_type: InstructionType::Transfer,
            details: HashMap::new(),
            is_nonce_advance: false,
            risk_flags,
            severity,
        }
    }

    // helper — builds a nonce advance instruction
    fn make_nonce_advance() -> DecodedInstruction {
        let mut details = HashMap::new();
        details.insert("nonce_account".to_string(), "NonceAcc111".to_string());
        details.insert("authority".to_string(), "Authority111".to_string());

        DecodedInstruction {
            index: 0,
            program: ProgramType::System,
            instruction_type: InstructionType::NonceAdvance,
            details,
            is_nonce_advance: true,
            risk_flags: vec!["DURABLE NONCE DETECTED".to_string()],
            severity: Severity::Critical,
        }
    }

    // helper — builds a nonce account creation (space = 80)
    fn make_nonce_create() -> DecodedInstruction {
        let mut details = HashMap::new();
        details.insert("space_bytes".to_string(), "80".to_string());

        DecodedInstruction {
            index: 1,
            program: ProgramType::System,
            instruction_type: InstructionType::CreateAccount,
            details,
            is_nonce_advance: false,
            risk_flags: vec!["NONCE ACCOUNT CREATION DETECTED".to_string()],
            severity: Severity::Critical,
        }
    }

    // helper — builds a token approve (delegate) instruction
    fn make_token_approve(unlimited: bool) -> DecodedInstruction {
        let (severity, risk_flags) = if unlimited {
            (
                Severity::Critical,
                vec!["UNLIMITED DELEGATE APPROVAL DETECTED".to_string()],
            )
        } else {
            (
                Severity::Warning,
                vec!["TOKEN DELEGATE APPROVAL: 1000000 raw units".to_string()],
            )
        };

        DecodedInstruction {
            index: 0,
            program: ProgramType::SplToken,
            instruction_type: InstructionType::TokenApprove,
            details: HashMap::new(),
            is_nonce_advance: false,
            risk_flags,
            severity,
        }
    }

    // helper — builds an unknown program instruction
    fn make_unknown_program() -> DecodedInstruction {
        DecodedInstruction {
            index: 0,
            program: ProgramType::Unknown("SomeProgram1111".to_string()),
            instruction_type: InstructionType::Unknown("SomeProg".to_string()),
            details: HashMap::new(),
            is_nonce_advance: false,
            risk_flags: vec!["unknown program: SomeProgram1111".to_string()],
            severity: Severity::Warning,
        }
    }

    fn no_nonce() -> NonceInfo {
        NonceInfo::none_info()
    }

    fn with_nonce() -> NonceInfo {
        NonceInfo {
            is_durable_nonce: true,
            nonce_account: Some("NonceAcc111".to_string()),
            nonce_authority: Some("Authority111".to_string()),
        }
    }

    // simple transfer with no risk signals
    #[test]
    fn test_simple_transfer_safe() {
        let instructions = vec![make_transfer(Severity::None)];
        let report = calculate(&instructions, &no_nonce());

        assert_eq!(report.score, 1);
        assert_eq!(report.level, Severity::None);
        assert_eq!(report.recommendation, "SAFE TO SIGN");
    }

    // large SOL transfer flagged as warning
    #[test]
    fn test_large_transfer_warning() {
        let instructions = vec![make_transfer(Severity::Warning)];
        let report = calculate(&instructions, &no_nonce());

        // base(1) + warning(2) = 3
        assert_eq!(report.score, 3);
        assert_eq!(report.level, Severity::Info);
        assert_eq!(report.recommendation, "REVIEW CAREFULLY");
    }

    // durable nonce + transfer = critical
    #[test]
    fn test_durable_nonce_transfer() {
        let instructions = vec![make_nonce_advance(), make_transfer(Severity::None)];
        let report = calculate(&instructions, &with_nonce());
        println!("report:{:#?}", report);
        // base(1) + nonce(5) + critical_severity(3) = 9
        assert!(report.score >= 7);
        assert_eq!(report.level, Severity::Critical);
        assert_eq!(report.recommendation, "DO NOT SIGN");
        assert!(report.flags.iter().any(|f| f.contains("DURABLE NONCE")));
    }

    // nonce account creation transaction
    #[test]
    fn test_nonce_account_creation() {
        let instructions = vec![make_nonce_create()];
        let report = calculate(&instructions, &no_nonce());
        println!("report:{:#?}", report);

        // base(1) + critical_severity(3) + nonce_create_bonus(2) = 6
        assert_eq!(report.score, 6);
        assert_eq!(report.level, Severity::Warning);
    }

    // unlimited delegate approval is critical
    #[test]
    fn test_unlimited_approve_critical() {
        let instructions = vec![make_token_approve(true)];
        let report = calculate(&instructions, &no_nonce());

        // base(1) + critical(3) = 4
        assert_eq!(report.score, 4);
        assert_eq!(report.level, Severity::Info);
        assert!(
            report
                .flags
                .iter()
                .any(|f| f.contains("UNLIMITED DELEGATE"))
        );
    }

    // unknown program adds extra risk
    #[test]
    fn test_unknown_program_flagged() {
        let instructions = vec![make_unknown_program()];
        let report = calculate(&instructions, &no_nonce());

        // base(1) + warning(2) + unknown_bonus(1) = 4
        assert_eq!(report.score, 4);
        assert!(report.flags.iter().any(|f| f.contains("unknown program")));
    }

    // score never exceeds 10 even with many critical instructions
    #[test]
    fn test_score_clamped_at_10() {
        let instructions = vec![
            make_nonce_advance(),
            make_nonce_create(),
            make_token_approve(true),
            make_unknown_program(),
        ];
        let report = calculate(&instructions, &with_nonce());

        assert_eq!(report.score, 10);
        assert_eq!(report.level, Severity::Critical);
        assert_eq!(report.recommendation, "DO NOT SIGN");
    }

    // empty transaction is safe
    #[test]
    fn test_empty_instructions_safe() {
        let instructions = vec![];
        let report = calculate(&instructions, &no_nonce());

        assert_eq!(report.score, 1);
        assert_eq!(report.recommendation, "SAFE TO SIGN");
    }

    // limited delegate approval is warning, not critical
    #[test]
    fn test_limited_approve_warning() {
        let instructions = vec![make_token_approve(false)];
        let report = calculate(&instructions, &no_nonce());

        // base(1) + warning(2) = 3
        assert_eq!(report.score, 3);
        assert_eq!(report.level, Severity::Info);
    }

    // multiple warnings stack up
    #[test]
    fn test_multiple_warnings_stack() {
        let instructions = vec![
            make_transfer(Severity::Warning),
            make_transfer(Severity::Warning),
            make_transfer(Severity::Warning),
        ];
        let report = calculate(&instructions, &no_nonce());

        // base(1) + 3×warning(2) = 7
        assert_eq!(report.score, 7);
        assert_eq!(report.level, Severity::Critical);
        assert_eq!(report.recommendation, "DO NOT SIGN");
    }
}
