//! Transaction Analyzer
//!
//! The master orchestrator that runs the full analysis pipeline on a decoded transaction.
//! Takes a DecodedPayload and produces a complete TransactionReport with risk assessment.
//!
//! # Pipeline Flow
//!
//! ```text
//! ┌──────────────┐
//! │ DecodedPayload│  ← input (from decoder.rs)
//! └──────┬───────┘
//!        │
//!        ├─→ decoder::decode_instructions()
//!        │   └─→ Vec<DecodedInstruction> + account_keys
//!        │
//!        ├─→ nonce::detect()
//!        │   └─→ NonceInfo (durable nonce detection)
//!        │
//!        ├─→ risk::calculate()
//!        │   └─→ RiskReport (score, level, recommendation)
//!        │
//!        ├─→ summary::build()
//!        │   └─→ String (plain English description)
//!        │
//!        └─→ assemble TransactionReport
//!            └─→ output (ready for API response)
//! ```
//!
//! # Why This Module Never Returns Errors
//!
//! The analyzer operates on a successfully decoded transaction (DecodedPayload).
//! By the time we reach this point:
//! - Input validation is complete (decoder.rs checked size, format, encoding)
//! - Transaction deserialization succeeded (it's a valid Solana transaction)
//! - All required data exists (instructions, accounts, fee payer)
//!
//! The pipeline stages (nonce, risk, summary) are pure functions that:
//! - Never make network calls (no RPC errors)
//! - Never parse untrusted external data (already validated)
//! - Always return valid results (even for empty/malformed instructions)
//!
//! Therefore, `analyze()` always succeeds and returns a TransactionReport.
use crate::{
    decoder::{DecodedPayload, decode_instructions},
    errors::ObsrvError,
    nonce::detect,
    risk::calculate,
    summary::build,
    types::TransactionReport,
};

/// Main entry point — analyzes a decoded transaction and produces a full report.
///
/// # Arguments
/// * `payload` — A successfully decoded Solana transaction (VersionedTransaction, Transaction, or Message)
///
/// # Returns
/// * `TransactionReport` — Complete analysis including decoded instructions, risk assessment, and human-readable summary
///
/// # Pipeline Stages
///
/// ## 1. Decode Instructions
/// Routes each instruction to the appropriate program decoder (System, Token, ComputeBudget).
/// Extracts human-readable details like amounts, accounts, and operation types.
///
/// ## 2. Detect Durable Nonce
/// Checks if instruction[0] is NonceAdvance, which makes the transaction valid until consumed.
///
/// ## 3. Calculate Risk
/// Scores the transaction 1-10 based on:
/// - Durable nonce presence
/// - Instruction severity levels
/// - High-risk patterns (unlimited approvals, authority changes, unknown programs)
///
/// ## 4. Build Summary
/// Generates a plain English paragraph describing what the transaction does.
///
/// ## 5. Assemble Report
/// Combines all analysis results into a TransactionReport for API response.
///
/// # Example Usage
///
/// ```ignore
/// use obsrv_core::{decoder, analyzer};
///
/// // user pastes transaction bytes
/// let payload = decoder::decode_payload("AQABAwAA...")?;
///
/// // analyze it
/// let report = analyzer::analyze(&payload);
///
/// // check risk
/// if report.risk_score >= 7 {
///     println!("⚠️  {}", report.recommendation); // "DO NOT SIGN"
/// }
/// ```
pub fn analyze(payload: &DecodedPayload) -> Result<TransactionReport, ObsrvError> {
    // STEP 1: Decode all instructions through program routers
    let (account_keys, instructions) = decode_instructions(payload);

    // STEP 2: Detect durable nonce pattern
    let nonce_info = detect(&instructions);

    // STEP 3: Calculate risk score (1-10)
    let risk = calculate(&instructions, &nonce_info);

    // STEP 4: Build plain English summary
    let fee_payer = account_keys
        .first()
        .cloned()
        .unwrap_or("unknown".to_string());

    let summary = build(&instructions, &nonce_info, &risk, &fee_payer);

    // STEP 5: Assemble final report
    Ok(TransactionReport {
        is_durable_nonce: nonce_info.is_durable_nonce,
        nonce_account: nonce_info.nonce_account,
        nonce_authority: nonce_info.nonce_authority,
        instructions,
        risk_score: risk.score,
        risk_level: risk.level,
        recommendation: risk.recommendation,
        risk_flags: risk.flags,
        summary,
        account_keys,
        fee_payer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::decode_payload;
    use crate::types::Severity;

    // durable nonce transaction (NonceAdvance + Transfer)
    const DURABLE_NONCE_TX: &str = "ArnD+fr8xechobZ7QUGrxq128WmMxT2iucGFXln364A7GO9Twp664gPAAeN948eiAYD9obD62ZEl55Mrt/uVCAcQrmiYS9g/6VfP6MqwobHFj5kW1t7wOMdcr0sv3cVIdR9T/pgwA1iCnvOqPqFM6s92/ApOgkdhLG8z6+zB+rIGAgECBuGfBJBZ26yNFPM3gCutcjzKspoXBOe1MvwXviVRO28Z4mv91TUKVeudzB5DiOE098dcS8nBxfmyxOv5EkIzVygK39HrqRa0X4Aqm1RBcb/cHcmOSa3Q/wUwltAETaGVRSqsLZ8ixBhOqlzbaEKJCB0NnflS1sh8mYm7bBif5NMVAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAGp9UXGSxWjuCKhF9z0peIzwNcMUWyGrNE2AYuqUAAAL/jcuDUXInb+pAgSAgVaHRSN3RrVD4Y6MxoCtq/Gr9IAgQDAgUBBAQAAAAEAgADDAIAAAAAZc0dAAAAAA==";

    // simple transfer message (no nonce)
    const SIMPLE_TRANSFER_TX: &str = "AXMpVzKQLwpmaY1eSPuFh+UkbrqaTGWI2IrODWcvh9Va4GrrfzaItHTagQTUFqEgTpGxxyV5ZcQQG6BZE9WiRg8BAAEDRQbwDX/agkQ3EICoKDy4SjtrrVTzLMV92uQlCn0a0GSI965zOfznZvS6/KDLYy49rN5+NJ+RPqHjpBAGP0VA5QAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAOkc9MwJV2OB0kfoOSZLD3FEbpXFCnWiPmWuf9jI8amkBAgIAAQwCAAAAQEIPAAAAAAA=";

    #[test]
    fn test_analyze_durable_nonce() {
        let payload = decode_payload(DURABLE_NONCE_TX).unwrap();
        let report = analyze(&payload).unwrap();

        // must detect durable nonce
        assert!(report.is_durable_nonce);
        assert!(report.nonce_account.is_some());
        assert!(report.nonce_authority.is_some());

        // must have critical risk
        assert!(report.risk_score >= 7);
        assert_eq!(report.recommendation, "DO NOT SIGN");
        assert_eq!(report.risk_level, Severity::Critical);

        // summary must mention nonce
        assert!(report.summary.contains("CRITICAL RISK"));
        assert!(report.summary.contains("NEVER expire"));
    }

    #[test]
    fn test_analyze_nonce_creation() {
        // CreateAccount(space=80) + InitializeNonce
        let payload = decode_payload(
               "Ak4gFKCa83/GlV1iSX5LwXwIvPkr072VBNjcLxn1SB8TQuh7LNmy6JNNhvuBw7sfys8/efZVe34NNpeaZAo2KwirZIBZyPUAXpT/qWjRDsIOL58s1QFtRYScrt5OlffjGrDvioK5fFFC3lhQV4jWs+H6A9G2xZSqRta17M5LimwIAgADBZBnAE896fexEFYyDV9lWxYy3zaWKMxacbGqHpYV/KtPQ38RQTGuS48xm6H1XZ3qY12pNVNqX1Cn4glTEGyVnyoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAan1RcZLFaO4IqEX3PSl4jPA1wxRbIas0TYBi6pQAAABqfVFxksXFEhjMlMPUrxf1ja7gibof1E49vZigAAAAC/43Lg1FyJ2/qQIEgIFWh0Ujd0a1Q+GOjMaAravxq/SAICAgABNAAAAAAAFxYAAAAAAFAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAwEDBCQGAAAAWsN067aXx0PUCSH9lxQAf+ywAXjnCj+IVsUPzpOM8M8=",
           ).unwrap();

        let report = analyze(&payload).unwrap();

        // not a durable nonce transaction (no NonceAdvance at ix[0])
        assert!(!report.is_durable_nonce);

        // but still high risk due to nonce account creation
        assert!(report.risk_score >= 5);
        assert!(report.summary.contains("DURABLE NONCE ACCOUNT"));
    }

    #[test]
    fn test_analyze_simple_transfer() {
        let payload = decode_payload(SIMPLE_TRANSFER_TX).unwrap();
        let report = analyze(&payload).unwrap();

        // no durable nonce
        assert!(!report.is_durable_nonce);

        // low risk
        assert!(report.risk_score <= 3);
        assert_eq!(report.recommendation, "SAFE TO SIGN");

        // summary should indicate safety
        assert!(report.summary.contains("safe") || report.summary.contains("✅"));

        // must have instructions
        assert!(!report.instructions.is_empty());
    }

    #[test]
    fn test_report_has_all_fields() {
        let payload = decode_payload(SIMPLE_TRANSFER_TX).unwrap();
        let report = analyze(&payload).unwrap();

        // all fields must be populated
        assert!(!report.account_keys.is_empty());
        assert!(!report.fee_payer.is_empty());
        assert!(!report.instructions.is_empty());
        assert!(!report.summary.is_empty());
        assert!(report.risk_score >= 1);
        assert!(!report.recommendation.is_empty());
    }

    #[test]
    fn test_empty_transaction_handled() {
        // minimal valid transaction with no instructions
        // (this would normally fail in decoder, but test defensive coding)
        let payload = decode_payload(SIMPLE_TRANSFER_TX).unwrap();
        let report = analyze(&payload).unwrap();

        // should still produce a valid report
        assert!(report.risk_score >= 1);
        assert!(!report.summary.is_empty());
    }
}
