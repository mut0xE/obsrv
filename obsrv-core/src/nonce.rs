// NONCE DETECTOR
// Looks at decoded instructions and answers:
//   is this a durable nonce transaction?
//   who controls the nonce?
//   what account is being used?
//
// Called by analyzer.rs after all instructions are decoded
// Result feeds into risk.rs and summary.rs
use crate::types::DecodedInstruction;

#[derive(Debug, Clone)]
pub struct NonceInfo {
    // true if instruction[0] is NonceAdvance
    pub is_durable_nonce: bool,
    // the nonce account address
    pub nonce_account: Option<String>,
    // who controls the nonce account
    pub nonce_authority: Option<String>,
}

impl NonceInfo {
    pub fn None() -> Self {
        NonceInfo {
            is_durable_nonce: false,
            nonce_account: None,
            nonce_authority: None,
        }
    }
}

/// Detect durable nonce pattern from decoded instructions.
///
/// Rules:
///   1. instructions must not be empty
///   2. instruction[0].is_nonce_advance must be true
///   3. extract nonce_account and authority from details
///
/// A nonce advance at position > 0 is unusual but not
/// the canonical durable nonce attack pattern.

pub fn detect(instructions: &[DecodedInstruction]) -> NonceInfo {
    if instructions.is_empty() {
        return NonceInfo::None();
    }

    let first = &instructions[0];
    // instruction[0] must be NonceAdvance
    if !first.is_nonce_advance {
        return NonceInfo::None();
    }

    let nonce_account = first.details.get("nonce_account").cloned();

    let nonce_authority = first.details.get("authority").cloned();

    NonceInfo {
        is_durable_nonce: true,
        nonce_account,
        nonce_authority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{InstructionType, ProgramType, Severity};
    use std::collections::HashMap;

    fn make_nonce_advance(index: usize) -> DecodedInstruction {
        let mut details = HashMap::new();
        details.insert(
            "nonce_account".to_string(),
            "NonceAccXXX111111111111111111111111111111111".to_string(),
        );
        details.insert(
            "authority".to_string(),
            "AuthorityXXX1111111111111111111111111111111".to_string(),
        );

        DecodedInstruction {
            index,
            program: ProgramType::System,
            instruction_type: InstructionType::NonceAdvance,
            details,
            is_nonce_advance: true,
            risk_flags: vec!["DURABLE NONCE DETECTED".to_string()],
            severity: Severity::Critical,
        }
    }
    fn make_transfer(index: usize) -> DecodedInstruction {
        let mut details = HashMap::new();
        details.insert(
            "from".to_string(),
            "SenderXXX111111111111111111111111111111111".to_string(),
        );
        details.insert(
            "to".to_string(),
            "ReceiverXXX11111111111111111111111111111111".to_string(),
        );
        details.insert("lamports".to_string(), "10000000".to_string());
        details.insert("sol".to_string(), "0.010000".to_string());

        DecodedInstruction {
            index,
            program: ProgramType::System,
            instruction_type: InstructionType::Transfer,
            details,
            is_nonce_advance: false,
            risk_flags: vec![],
            severity: Severity::None,
        }
    }

    #[test]
    fn test_durable_nonce_detected() {
        // ix[0] = NonceAdvance + ix[1] = Transfer
        // classic durable nonce attack pattern
        let instructions = vec![make_nonce_advance(0), make_transfer(1)];

        let info = detect(&instructions);
        println!("nonce_info:{:#?}", info);

        assert!(info.is_durable_nonce);
        assert_eq!(
            info.nonce_account.unwrap(),
            "NonceAccXXX111111111111111111111111111111111"
        );
        assert_eq!(
            info.nonce_authority.unwrap(),
            "AuthorityXXX1111111111111111111111111111111"
        );
    }

    #[test]
    fn test_no_nonce_simple_transfer() {
        // just a transfer, no nonce
        let instructions = vec![make_transfer(0)];

        let info = detect(&instructions);
        println!("no_nonce:{:#?}", info);

        assert!(!info.is_durable_nonce);
        assert!(info.nonce_account.is_none());
        assert!(info.nonce_authority.is_none());
    }

    #[test]
    fn test_empty_instructions() {
        let instructions = vec![];
        let info = detect(&instructions);

        assert!(!info.is_durable_nonce);
        assert!(info.nonce_account.is_none());
    }

    #[test]
    fn test_nonce_not_first_not_detected() {
        // nonce at index 1
        // transfer first, then nonce = unusual but not flagged as durable nonce
        let instructions = vec![make_transfer(0), make_nonce_advance(1)];

        let info = detect(&instructions);
        println!("nonce_not_first:{:#?}", info);

        assert!(!info.is_durable_nonce);
        assert!(info.nonce_account.is_none());
    }

    #[test]
    fn test_durable_nonce_multiple_instructions() {
        // nonce + multiple token drains
        let mut token_drain_1 = make_transfer(1);
        token_drain_1
            .details
            .insert("amount".to_string(), "285000000000".to_string());

        let mut token_drain_2 = make_transfer(2);
        token_drain_2
            .details
            .insert("amount".to_string(), "100000000000".to_string());

        let instructions = vec![make_nonce_advance(0), token_drain_1, token_drain_2];

        let info = detect(&instructions);
        println!("multi_drain:{:#?}", info);

        assert!(info.is_durable_nonce);
        assert!(info.nonce_account.is_some());
        assert!(info.nonce_authority.is_some());
    }
}
