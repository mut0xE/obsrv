use crate::{
    decoder::{DecodedPayload, decode_instructions},
    nonce::detect,
    risk::calculate,
    summary::build,
    types::TransactionReport,
};

// ANALYZER
// The master orchestrator.
// Takes a DecodedPayload and runs the full analysis pipeline.
//
// Flow:
//   1. decode_instructions()  → Vec<DecodedInstruction> + account_keys
//   2. nonce::detect()        → NonceInfo
//   3. risk::calculate()      → RiskReport
//   4. summary::build()       → String
//   5. assemble TransactionReport
pub fn analyze(payload: &DecodedPayload) -> TransactionReport {
    // ── STEP 1: decode all instructions ──────────────────
    let (account_keys, instructions) = decode_instructions(payload);
    // ── STEP 2: detect durable nonce pattern ─────────────
    let nonce_info = detect(&instructions);
    // ── STEP 3: calculate risk score ─────────────────────
    let risk = calculate(&instructions, &nonce_info);
    // ── STEP 4: build plain English summary ──────────────
    let fee_payer = account_keys
        .first()
        .cloned()
        .unwrap_or("unknown".to_string());
    let summary = build(&instructions, &nonce_info, &risk, &fee_payer);
    // ── STEP 5: assemble TransactionReport ───────────────
    TransactionReport {
        is_durable_nonce: nonce_info.is_durable_nonce,
        nonce_account: nonce_info.nonce_account,
        nonce_authority: nonce_info.nonce_authority,
        instructions,
        risk_score: risk.score,
        risk_level: risk.level,
        recommendation: risk.recommendation,
        summary,
        account_keys,
        fee_payer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::decode_payload;
    use crate::types::Severity;

    // durable nonce base64
    const DURABLE_NONCE_TX: &str = "ArnD+fr8xechobZ7QUGrxq128WmMxT2iucGFXln364A7GO9Twp664gPAAeN948eiAYD9obD62ZEl55Mrt/uVCAcQrmiYS9g/6VfP6MqwobHFj5kW1t7wOMdcr0sv3cVIdR9T/pgwA1iCnvOqPqFM6s92/ApOgkdhLG8z6+zB+rIGAgECBuGfBJBZ26yNFPM3gCutcjzKspoXBOe1MvwXviVRO28Z4mv91TUKVeudzB5DiOE098dcS8nBxfmyxOv5EkIzVygK39HrqRa0X4Aqm1RBcb/cHcmOSa3Q/wUwltAETaGVRSqsLZ8ixBhOqlzbaEKJCB0NnflS1sh8mYm7bBif5NMVAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAGp9UXGSxWjuCKhF9z0peIzwNcMUWyGrNE2AYuqUAAAL/jcuDUXInb+pAgSAgVaHRSN3RrVD4Y6MxoCtq/Gr9IAgQDAgUBBAQAAAAEAgADDAIAAAAAZc0dAAAAAA==";

    // simple transfer message
    const SIMPLE_TRANSFER_TX: &str = "AXMpVzKQLwpmaY1eSPuFh+UkbrqaTGWI2IrODWcvh9Va4GrrfzaItHTagQTUFqEgTpGxxyV5ZcQQG6BZE9WiRg8BAAEDRQbwDX/agkQ3EICoKDy4SjtrrVTzLMV92uQlCn0a0GSI965zOfznZvS6/KDLYy49rN5+NJ+RPqHjpBAGP0VA5QAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAOkc9MwJV2OB0kfoOSZLD3FEbpXFCnWiPmWuf9jI8amkBAgIAAQwCAAAAQEIPAAAAAAA=";

    #[test]
    fn test_analyze_durable_nonce() {
        let payload = decode_payload(DURABLE_NONCE_TX).unwrap();
        let report = analyze(&payload);
        println!("durable nonce transfer:{:#?}", report);

        println!("\nDURABLE NONCE REPORT");
        println!("is_durable_nonce: {}", report.is_durable_nonce);
        println!("nonce_account:    {:?}", report.nonce_account);
        println!("nonce_authority:  {:?}", report.nonce_authority);
        println!("risk_score:       {}", report.risk_score);
        println!("risk_level:       {:?}", report.risk_level);
        println!("recommendation:   {}", report.recommendation);
        println!("summary:\n{}", report.summary);
        println!("\ninstructions:");
        for ix in &report.instructions {
            println!(
                "  [{}] {:?} — {:?}",
                ix.index, ix.instruction_type, ix.severity
            );
        }

        assert!(report.is_durable_nonce);
        assert!(report.nonce_account.is_some());
        assert!(report.nonce_authority.is_some());
        assert!(report.risk_score >= 9);
        assert_eq!(report.recommendation, "DO NOT SIGN");
        assert_eq!(report.risk_level, Severity::Critical);
        assert!(report.summary.contains("CRITICAL RISK"));
        assert!(report.summary.contains("NEVER expire"));
    }

    #[test]
    fn test_analyze_nonce_creation() {
        // real nonce account creation tx
        // ix[0] = CreateAccount(space=80) + ix[1] = InitializeNonce
        let payload = decode_payload(
               "Ak4gFKCa83/GlV1iSX5LwXwIvPkr072VBNjcLxn1SB8TQuh7LNmy6JNNhvuBw7sfys8/efZVe34NNpeaZAo2KwirZIBZyPUAXpT/qWjRDsIOL58s1QFtRYScrt5OlffjGrDvioK5fFFC3lhQV4jWs+H6A9G2xZSqRta17M5LimwIAgADBZBnAE896fexEFYyDV9lWxYy3zaWKMxacbGqHpYV/KtPQ38RQTGuS48xm6H1XZ3qY12pNVNqX1Cn4glTEGyVnyoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAan1RcZLFaO4IqEX3PSl4jPA1wxRbIas0TYBi6pQAAABqfVFxksXFEhjMlMPUrxf1ja7gibof1E49vZigAAAAC/43Lg1FyJ2/qQIEgIFWh0Ujd0a1Q+GOjMaAravxq/SAICAgABNAAAAAAAFxYAAAAAAFAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAwEDBCQGAAAAWsN067aXx0PUCSH9lxQAf+ywAXjnCj+IVsUPzpOM8M8=",
           ).unwrap();

        let report = analyze(&payload);
        println!("nonce create:{:#?}", report);

        println!("\n=== NONCE CREATION REPORT ===");
        println!("risk_score:     {}", report.risk_score);
        println!("recommendation: {}", report.recommendation);
        println!("summary:\n{}", report.summary);
        println!("\ninstructions:");
        for ix in &report.instructions {
            println!(
                "  [{}] {:?} — {:?}",
                ix.index, ix.instruction_type, ix.severity
            );
            println!("      flags: {:?}", ix.risk_flags);
        }

        // nonce creation is high risk but not a durable nonce transfer
        assert!(!report.is_durable_nonce);
        assert!(report.risk_score >= 6);
        assert!(report.summary.contains("DURABLE NONCE ACCOUNT"));
    }

    #[test]
    fn test_analyze_simple_transfer() {
        let payload = decode_payload(SIMPLE_TRANSFER_TX).unwrap();
        let report = analyze(&payload);

        println!("\n=== SIMPLE TRANSFER REPORT ===");
        println!("is_durable_nonce: {}", report.is_durable_nonce);
        println!("risk_score:       {}", report.risk_score);
        println!("recommendation:   {}", report.recommendation);
        println!("summary:\n{}", report.summary);
        println!("\ninstructions:");
        for ix in &report.instructions {
            println!(
                "  [{}] {:?} — {:?}",
                ix.index, ix.instruction_type, ix.severity
            );
        }

        assert!(!report.is_durable_nonce);
        assert!(report.risk_score <= 3);
        assert_eq!(report.recommendation, "SAFE TO SIGN");
        assert!(report.summary.contains("safe"));
        assert!(!report.instructions.is_empty());
    }

    #[test]
    fn test_report_has_all_fields() {
        let payload = decode_payload(SIMPLE_TRANSFER_TX).unwrap();
        let report = analyze(&payload);

        // all fields must be populated
        assert!(!report.account_keys.is_empty());
        assert!(!report.fee_payer.is_empty());
        assert!(!report.instructions.is_empty());
        assert!(!report.summary.is_empty());
        assert!(report.risk_score >= 1);
    }
}
