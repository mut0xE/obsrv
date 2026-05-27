//! Transaction Summary Builder
//!
//! Generates human-readable plain English descriptions of Solana transactions.
//! Called as the final step in the analysis pipeline before assembling the TransactionReport.
//!
//! # Purpose
//!
//! Decoded instructions are technical (program IDs, account indices, raw bytes).
//! This module translates them into sentences users can understand:
//!
//! ```text
//! Technical:  { program: "11111...", type: 2, data: [2,0,0,0, 64,66,15,0,0,0,0,0], ... }
//! Human:      "Transfer 0.001000 SOL (1000000 lamports) from 3SoM... to 6v6Y..."
//! ```
//!
//! # Output Format
//!
//! The summary is a single paragraph with:
//! - Opening risk assessment line (✅ safe / ⚠️ warning / ⚠️ critical)
//! - Durable nonce warning (if applicable)
//! - Description of each instruction
//! - Fee payer address
//! - Closing risk score + recommendation
//!
//! # Pipeline Position
//!

use crate::nonce::NonceInfo;
use crate::risk::RiskReport;
use crate::types::{DecodedInstruction, InstructionType, Severity};

/// Main entry point — called by analyzer to generate transaction summary.
///
/// # Arguments
/// * `instructions` — all decoded instructions from the transaction
/// * `nonce_info` — result of nonce::detect(), tells us if this is a durable nonce tx
/// * `risk` — result of risk::calculate(), contains score/level/recommendation
/// * `fee_payer` — first account in transaction (pays fees)
///
/// # Returns
/// * `String` — plain English paragraph describing the transaction
pub fn build(
    instructions: &[DecodedInstruction],
    nonce_info: &NonceInfo,
    risk: &RiskReport,
    fee_payer: &str,
) -> String {
    if instructions.is_empty() {
        return "Empty transaction with no instructions.".to_string();
    }

    let mut parts: Vec<String> = Vec::new();

    // opening line based on risk level
    let opening = match risk.level {
        Severity::Critical => "⚠️  CRITICAL RISK TRANSACTION DETECTED.".to_string(),
        Severity::Warning => "⚠️  This transaction requires careful review.".to_string(),
        Severity::Info => "ℹ️  This transaction contains notable activity.".to_string(),
        Severity::None => "✅ This transaction appears safe.".to_string(),
    };
    parts.push(opening);

    // durable nonce warning (if applicable)
    if nonce_info.is_durable_nonce {
        let nonce_summary = build_nonce_summary(nonce_info);
        parts.push(nonce_summary);
    }

    // describe each instruction in plain English
    for ix in instructions {
        if let Some(desc) = describe_instruction(ix) {
            parts.push(desc);
        }
    }

    // fee payer (shortened to first 4 chars for readability)
    parts.push(format!("Fee payer: {}", shorten(fee_payer)));

    // closing recommendation
    parts.push(format!(
        "Risk score: {}/10 — {}",
        risk.score, risk.recommendation
    ));

    parts.join(" ")
}

/// Generates a durable nonce warning paragraph.
fn build_nonce_summary(nonce_info: &NonceInfo) -> String {
    let account = nonce_info
        .nonce_account
        .as_deref()
        .map(shorten)
        .unwrap_or("unknown".to_string());

    let authority = nonce_info
        .nonce_authority
        .as_deref()
        .map(shorten)
        .unwrap_or("unknown".to_string());

    format!(
        "This transaction uses a durable nonce (account: {}, authority: {}) \
         and will NEVER expire — it can be submitted at any time in the future.",
        account, authority
    )
}

/// Converts a single DecodedInstruction into a human-readable sentence.
///
/// Returns None if the instruction type doesn't warrant a description
/// (e.g. ComputeBudget instructions are usually not interesting to users).
fn describe_instruction(ix: &DecodedInstruction) -> Option<String> {
    match &ix.instruction_type {
        InstructionType::Transfer => {
            let from = ix
                .details
                .get("from")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let to = ix
                .details
                .get("to")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let sol = ix.details.get("sol").cloned().unwrap_or("?".to_string());
            let lamports = ix
                .details
                .get("lamports")
                .cloned()
                .unwrap_or("?".to_string());

            Some(format!(
                "Instruction {}: Transfer {} SOL ({} lamports) from {} to {}.",
                ix.index, sol, lamports, from, to
            ))
        }

        InstructionType::NonceAdvance => {
            let account = ix
                .details
                .get("nonce_account")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let authority = ix
                .details
                .get("authority")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());

            Some(format!(
                "Instruction {}: Advance nonce account {} (authority: {}).",
                ix.index, account, authority
            ))
        }

        InstructionType::NonceInitialize => {
            let account = ix
                .details
                .get("nonce_account")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let authority = ix
                .details
                .get("authority")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());

            Some(format!(
                "Instruction {}: Initialize nonce account {} with authority {}.",
                ix.index, account, authority
            ))
        }

        InstructionType::NonceWithdraw => {
            let account = ix
                .details
                .get("nonce_account")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let dest = ix
                .details
                .get("destination")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let sol = ix.details.get("sol").cloned().unwrap_or("?".to_string());

            Some(format!(
                "Instruction {}: Withdraw {} SOL from nonce account {} to {}.",
                ix.index, sol, account, dest
            ))
        }

        InstructionType::NonceAuthorize => {
            let account = ix
                .details
                .get("nonce_account")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let new_auth = ix
                .details
                .get("new_authority")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());

            Some(format!(
                "Instruction {}: Transfer control of nonce account {} to new authority {}.",
                ix.index, account, new_auth
            ))
        }

        InstructionType::CreateAccount => {
            let from = ix
                .details
                .get("from")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let new_acc = ix
                .details
                .get("new_account")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let space = ix
                .details
                .get("space_bytes")
                .cloned()
                .unwrap_or("?".to_string());
            let sol = ix.details.get("sol").cloned().unwrap_or("?".to_string());

            let account_type = if space == "80" {
                " (DURABLE NONCE ACCOUNT)".to_string()
            } else if space == "165" {
                " (token account)".to_string()
            } else {
                format!(" ({} bytes)", space)
            };

            Some(format!(
                "Instruction {}: Create account {}{} funded with {} SOL by {}.",
                ix.index, new_acc, account_type, sol, from
            ))
        }

        InstructionType::TokenTransfer => {
            let source = ix
                .details
                .get("source")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let dest = ix
                .details
                .get("destination")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let amount = ix.details.get("amount").cloned().unwrap_or("?".to_string());

            Some(format!(
                "Instruction {}: Transfer {} raw token units from {} to {}.",
                ix.index, amount, source, dest
            ))
        }

        InstructionType::TokenTransferChecked => {
            let source = ix
                .details
                .get("source")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let dest = ix
                .details
                .get("destination")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let amount = ix
                .details
                .get("amount_human")
                .cloned()
                .unwrap_or("?".to_string());
            let mint = ix
                .details
                .get("mint")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());

            Some(format!(
                "Instruction {}: Transfer {} tokens (mint: {}) from {} to {}.",
                ix.index, amount, mint, source, dest
            ))
        }

        InstructionType::TokenCloseAccount => {
            let account = ix
                .details
                .get("account")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let dest = ix
                .details
                .get("destination")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());

            Some(format!(
                "Instruction {}: Close token account {}, send rent to {}.",
                ix.index, account, dest
            ))
        }

        InstructionType::TokenSetAuthority => {
            let account = ix
                .details
                .get("account")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let new_auth = ix
                .details
                .get("new_authority")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let auth_type = ix
                .details
                .get("authority_type")
                .cloned()
                .unwrap_or("Unknown".to_string());

            Some(format!(
                "Instruction {}: Change {} authority for {} to {}.",
                ix.index, auth_type, account, new_auth
            ))
        }

        InstructionType::TokenApprove => {
            let source = ix
                .details
                .get("source")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let delegate = ix
                .details
                .get("delegate")
                .map(|s| shorten(s))
                .unwrap_or("unknown".to_string());
            let amount = ix.details.get("amount").cloned().unwrap_or("?".to_string());

            Some(format!(
                "Instruction {}: Approve delegate {} to spend {} raw token units from {}.",
                ix.index, delegate, amount, source
            ))
        }

        InstructionType::SetComputeUnitLimit => {
            let units = ix
                .details
                .get("compute_units")
                .cloned()
                .unwrap_or("?".to_string());

            Some(format!(
                "Instruction {}: Set compute unit limit to {} CU.",
                ix.index, units
            ))
        }

        InstructionType::SetComputeUnitPrice => {
            let price = ix
                .details
                .get("microlamports_per_cu")
                .cloned()
                .unwrap_or("?".to_string());

            Some(format!(
                "Instruction {}: Set compute unit price to {} microlamports/CU.",
                ix.index, price
            ))
        }

        InstructionType::Unknown(name) => Some(format!(
            "Instruction {}: Unknown instruction from program {}.",
            ix.index, name
        )),
    }
}

/// Shortens a Solana public key to first 4 characters for readability.
///
/// Full pubkeys are 32 bytes (44 chars in base58), which is too long for summaries.
/// We keep the whole string for short values (≤12 chars) or return the full string.
fn shorten(s: &str) -> String {
    if s.len() <= 12 {
        return s.to_string();
    }
    // return full string (we can change this to show first 4 + ... + last 4 if needed)
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::RiskReport;
    use crate::types::{ProgramType, Severity};
    use std::collections::HashMap;

    // helper — creates a RiskReport with given score
    fn make_risk(score: u8) -> RiskReport {
        let (level, recommendation) = match score {
            1..=2 => (Severity::None, "SAFE TO SIGN".to_string()),
            3..=4 => (Severity::Info, "REVIEW CAREFULLY".to_string()),
            5..=6 => (Severity::Warning, "PROCEED WITH CAUTION".to_string()),
            _ => (Severity::Critical, "DO NOT SIGN".to_string()),
        };
        RiskReport {
            score,
            level,
            recommendation,
            flags: vec![],
        }
    }

    // helper — creates a Transfer instruction
    fn make_transfer_ix() -> DecodedInstruction {
        let mut details = HashMap::new();
        details.insert(
            "from".to_string(),
            "3SoMn5fXZB6131jjThiVNCcG512DrCkRxiVhtYvu4c5Q".to_string(),
        );
        details.insert(
            "to".to_string(),
            "6v6YQXkuLTfZjLU5KK7CV8j42MHwqwGQvrjDM5GTJxZV".to_string(),
        );
        details.insert("sol".to_string(), "0.010000".to_string());
        details.insert("lamports".to_string(), "10000000".to_string());

        DecodedInstruction {
            index: 0,
            program: ProgramType::System,
            instruction_type: InstructionType::Transfer,
            details,
            is_nonce_advance: false,
            risk_flags: vec![],
            severity: Severity::None,
        }
    }

    // helper — creates a NonceAdvance instruction
    fn make_nonce_advance_ix() -> DecodedInstruction {
        let mut details = HashMap::new();
        details.insert(
            "nonce_account".to_string(),
            "3Rz461rwuxY4mJzXtQWad1J1ZwKybeQPRa8BHTTLgSjG".to_string(),
        );
        details.insert(
            "authority".to_string(),
            "3SoMn5fXZB6131jjThiVNCcG512DrCkRxiVhtYvu4c5Q".to_string(),
        );

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

    #[test]
    fn test_simple_transfer_summary() {
        let instructions = vec![make_transfer_ix()];
        let nonce = NonceInfo::none_info();
        let risk = make_risk(1);

        let summary = build(
            &instructions,
            &nonce,
            &risk,
            "3SoMn5fXZB6131jjThiVNCcG512DrCkRxiVhtYvu4c5Q",
        );

        assert!(summary.contains("Transfer"));
        assert!(summary.contains("0.010000 SOL"));
        assert!(summary.contains("SAFE TO SIGN"));
    }

    #[test]
    fn test_durable_nonce_transfer_summary() {
        let mut transfer = make_transfer_ix();
        transfer.index = 1;

        let instructions = vec![make_nonce_advance_ix(), transfer];

        let nonce = NonceInfo {
            is_durable_nonce: true,
            nonce_account: Some("3Rz461rwuxY4mJzXtQWad1J1ZwKybeQPRa8BHTTLgSjG".to_string()),
            nonce_authority: Some("3SoMn5fXZB6131jjThiVNCcG512DrCkRxiVhtYvu4c5Q".to_string()),
        };
        let risk = make_risk(9);

        let summary = build(
            &instructions,
            &nonce,
            &risk,
            "3SoMn5fXZB6131jjThiVNCcG512DrCkRxiVhtYvu4c5Q",
        );

        assert!(summary.contains("CRITICAL RISK"));
        assert!(summary.contains("durable nonce"));
        assert!(summary.contains("NEVER expire"));
        assert!(summary.contains("DO NOT SIGN"));
        assert!(summary.contains("Transfer"));
    }

    #[test]
    fn test_empty_instructions_summary() {
        let summary = build(&[], &NonceInfo::none_info(), &make_risk(1), "fee_payer");

        assert!(summary.contains("Empty transaction"));
    }

    #[test]
    fn test_nonce_account_creation_summary() {
        let mut details = HashMap::new();
        details.insert(
            "from".to_string(),
            "3SoMn5fXZB6131jjThiVNCcG512DrCkRxiVhtYvu4c5Q".to_string(),
        );
        details.insert(
            "new_account".to_string(),
            "3Rz461rwuxY4mJzXtQWad1J1ZwKybeQPRa8BHTTLgSjG".to_string(),
        );
        details.insert("space_bytes".to_string(), "80".to_string());
        details.insert("sol".to_string(), "0.001448".to_string());

        let ix = DecodedInstruction {
            index: 0,
            program: ProgramType::System,
            instruction_type: InstructionType::CreateAccount,
            details,
            is_nonce_advance: false,
            risk_flags: vec!["NONCE ACCOUNT CREATION DETECTED".to_string()],
            severity: Severity::Critical,
        };

        let summary = build(
            &[ix],
            &NonceInfo::none_info(),
            &make_risk(6),
            "3SoMn5fXZB6131jjThiVNCcG512DrCkRxiVhtYvu4c5Q",
        );

        assert!(summary.contains("DURABLE NONCE ACCOUNT"));
        assert!(summary.contains("0.001448 SOL"));
    }
}
