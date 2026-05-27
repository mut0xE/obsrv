//! Durable Nonce Detector
//!
//! Analyzes decoded instructions to determine if this transaction uses a durable nonce
//! instead of a recent blockhash for transaction validity.
//!
//! # What Are Durable Nonces?
//!
//! Normal Solana transactions expire after ~60-90 seconds when the recent blockhash ages out.
//! Durable nonces allow a transaction to remain valid **forever** until the nonce value is consumed.
//!
//! This is useful for:
//! - Multisig wallets (offline signers need time to coordinate)
//! - Hardware wallets (user not always present to sign immediately)
//! - Scheduled transactions
//!
//! But attackers abuse it to create transactions that can be executed at any time:
//! - Phishing: Get victim to sign a "safe looking" tx, execute it months later after victim forgets
//! - Delayed drains: Set up drain transaction, wait for victim's balance to grow, then execute
//!
//! # Detection Logic
//!
//! A transaction is a durable nonce transaction if and only if:
//! ```text
//! instructions[0].is_nonce_advance == true
//! ```
//!
//! The System Program's NonceAdvance instruction (discriminator 4) is the ONLY instruction
//! type where `is_nonce_advance` is set to `true` (see programs/system.rs).
//!
//! # Pipeline Position
//!
//! ```text
//! decoder.rs → programs/ → nonce.rs → risk.rs → summary.rs
//!                          ^^^^^^^^
//!                          you are here
//! ```

use crate::types::DecodedInstruction;

/// Result of nonce detection analysis.
#[derive(Debug, Clone)]
pub struct NonceInfo {
    /// True if instruction[0] is NonceAdvance
    pub is_durable_nonce: bool,

    /// The nonce account address (if durable nonce detected)
    pub nonce_account: Option<String>,

    /// Who controls the nonce account (if durable nonce detected)
    pub nonce_authority: Option<String>,
}

impl NonceInfo {
    /// Returns a NonceInfo indicating no durable nonce was detected.
    pub fn none_info() -> Self {
        NonceInfo {
            is_durable_nonce: false,
            nonce_account: None,
            nonce_authority: None,
        }
    }
}

/// Main entry point — called by analyzer after instruction decoding.
///
/// # Detection Rules
///
/// 1. Instructions must not be empty
/// 2. First instruction must be a NonceAdvance (checked via `is_nonce_advance` flag)
/// 3. Extract nonce account and authority from the instruction's details map
///
/// # Why Only Check Position 0?
///
/// The Solana runtime enforces that NonceAdvance MUST be the first instruction
/// for the nonce to be used as the transaction's validity anchor. A NonceAdvance
/// at position > 0 is unusual but doesn't make the tx durable — the runtime will
/// still use a recent blockhash for validity.
///
/// # Arguments
/// * `instructions` — all decoded instructions from the transaction
///
/// # Returns
/// * `NonceInfo` — contains durable nonce status + account details if detected
pub fn detect(instructions: &[DecodedInstruction]) -> NonceInfo {
    // empty transaction has no nonce
    if instructions.is_empty() {
        return NonceInfo::none_info();
    }

    let first = &instructions[0];

    // check if first instruction is NonceAdvance
    // (only NonceAdvance sets is_nonce_advance = true, see programs/system.rs)
    if !first.is_nonce_advance {
        return NonceInfo::none_info();
    }

    // extract nonce account and authority from instruction details
    // (system::nonce_advance() populates these fields)
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

    // helper — creates a NonceAdvance instruction at specified index
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

    // helper — creates a simple Transfer instruction
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
        // classic durable nonce pattern: NonceAdvance + Transfer
        let instructions = vec![make_nonce_advance(0), make_transfer(1)];

        let info = detect(&instructions);

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
        // nonce at index 1 — unusual but not a durable nonce transaction
        // (runtime won't use it as validity anchor if it's not first)
        let instructions = vec![make_transfer(0), make_nonce_advance(1)];

        let info = detect(&instructions);

        assert!(!info.is_durable_nonce);
        assert!(info.nonce_account.is_none());
    }

    #[test]
    fn test_durable_nonce_multiple_instructions() {
        // nonce + multiple operations (token drains, etc)
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

        assert!(info.is_durable_nonce);
        assert!(info.nonce_account.is_some());
        assert!(info.nonce_authority.is_some());
    }
}
