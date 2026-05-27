//! Compute Budget Program Decoder (`ComputeBudget111111111111111111111111111111`)
//!
//! The Compute Budget Program controls transaction resource allocation on Solana.
//! Every transaction has a compute budget that determines:
//!
//! - **How many compute units (CU)** the transaction can consume before halting
//! - **Priority fee** — how much extra the user pays per CU for faster inclusion
//!
//! # Why This Matters for Security
//!
//! Compute budget instructions are usually benign (just fee tuning), but two patterns
//! are suspicious:
//!
//! - **Very low CU limit** (<1,000): The transaction may be designed to fail after
//!   executing side effects (e.g., approve a token, then run out of CU before the
//!   "safe" revert logic runs). Flagged as Warning.
//! - **Very high priority fee** (>10M microlamports/CU)
//!
//! # Binary Format
//!
//! Unlike the System Program (which uses a 4-byte u32 discriminator), the Compute
//! Budget Program uses a **1-byte u8 discriminator**:
//! ```text
//! [0]   = instruction discriminator (u8)
//! [1..] = instruction-specific data
//! ```
//!
//! # Instruction Types (discriminator values)
//!
//! | Discriminator | Instruction          | Data Size | Description                    |
//! |---------------|----------------------|-----------|--------------------------------|
//! | 0             | RequestUnitsDeprecated | 9 bytes | Legacy, not implemented        |
//! | 1             | RequestHeapFrame     | 5 bytes   | Not implemented                |
//! | 2             | SetComputeUnitLimit  | 5 bytes   | Max CU this tx can consume     |
//! | 3             | SetComputeUnitPrice  | 9 bytes   | Priority fee in microlamports  |
//!
//! # Account References
//!
//! Compute Budget instructions never reference any accounts. The `accounts` parameter
//! is always empty — these instructions configure the transaction itself, not any
//! on-chain account state.

use std::collections::HashMap;

use crate::types::{DecodedInstruction, InstructionType, ProgramType, Severity};

/// Entry point for Compute Budget Program instruction decoding.
///
/// Called by `programs::decode_instruction()` when the program ID matches
/// `ComputeBudget111111111111111111111111111111`.
///
/// # Arguments
/// * `index` — position of this instruction in the transaction (0 = first)
/// * `data` — raw instruction data bytes (1-byte discriminator + payload)
/// * `_accounts` — always empty for compute budget instructions (no accounts referenced)
/// * `_account_keys` — unused, included for consistent decoder signature
///
/// # Discriminator Format
///
/// The Compute Budget Program uses a single byte (u8) as its discriminator,
/// unlike the System Program which uses 4 bytes (u32 LE). This is because
/// the Compute Budget Program has fewer instruction variants and was designed
/// later with a simpler encoding.
pub fn decode(
    index: usize,
    data: &[u8],
    _accounts: &[u8],
    _account_keys: &[String],
) -> DecodedInstruction {
    if data.is_empty() {
        return invalid(index);
    }

    // Single byte discriminator (u8), unlike System Program's 4-byte u32
    let ix_type = data[0];

    match ix_type {
        2 => set_compute_unit_limit(index, data),
        3 => set_compute_unit_price(index, data),
        _ => unknown_compute(index, ix_type),
    }
}

/// Discriminator 2: SetComputeUnitLimit
///
/// Sets the maximum number of compute units this transaction is allowed to consume.
/// If the transaction exceeds this limit, the runtime halts execution and the
/// transaction fails (but fees are still charged).
///
/// # Defaults & Limits
/// - **Default**: 200,000 CU per instruction (if this instruction is omitted)
/// - **Maximum**: 1,400,000 CU per transaction
/// - **Typical range**: 50,000 – 400,000 CU for most DeFi transactions
///
/// # Data Layout (5 bytes total)
/// ```text
/// [0]    = discriminator (2)
/// [1..5] = compute unit limit as u32 little-endian
/// ```
fn set_compute_unit_limit(index: usize, data: &[u8]) -> DecodedInstruction {
    // Need 5 bytes: 1 (discriminator) + 4 (u32 units)
    if data.len() < 5 {
        return invalid(index);
    }

    // CU limit at bytes 1-4 as u32 little-endian
    let units = u32::from_le_bytes(data[1..5].try_into().unwrap_or([0; 4]));

    let mut details = HashMap::new();
    details.insert("compute_units".to_string(), units.to_string());

    // Flag suspiciously low CU limits (< 1,000)
    let (risk_flags, severity) = if units < 1_000 {
        (
            vec![
                format!("VERY LOW COMPUTE LIMIT: {} CU", units),
                "abnormally low compute unit limit".to_string(),
                "transaction may be designed to fail after side effects".to_string(),
            ],
            Severity::Warning,
        )
    } else {
        (vec![], Severity::None)
    };

    DecodedInstruction {
        index,
        program: ProgramType::ComputeBudget,
        instruction_type: InstructionType::SetComputeUnitLimit,
        details,
        is_nonce_advance: false,
        risk_flags,
        severity,
    }
}

/// Discriminator 3: SetComputeUnitPrice
///
/// Sets the priority fee in **microlamports per compute unit**. Validators use this
/// to prioritize transactions — higher price = faster inclusion in the next block.
///
/// # How Priority Fees Work
///
/// ```text
/// total_priority_fee = microlamports_per_cu × compute_units_consumed
///                      ÷ 1_000_000  (convert to lamports)
/// ```
///
/// Example: 100,000 microlamports/CU × 200,000 CU = 20,000 lamports = 0.00002 SOL
///
/// # Unit Conversions
/// - 1 SOL = 1,000,000,000 lamports
/// - 1 lamport = 1,000,000 microlamports
/// - So: 1 SOL = 1,000,000,000,000,000 microlamports (1e15)
///
/// # Data Layout (9 bytes total)
/// ```text
/// [0]    = discriminator (3)
/// [1..9] = price in microlamports per CU as u64 little-endian
/// ```
///
fn set_compute_unit_price(index: usize, data: &[u8]) -> DecodedInstruction {
    // Need 9 bytes: 1 (discriminator) + 8 (u64 price)
    if data.len() < 9 {
        return invalid(index);
    }

    // Priority fee at bytes 1-8 as u64 little-endian
    let microlamports = u64::from_le_bytes(data[1..9].try_into().unwrap_or([0; 8]));

    // Convert microlamports to lamports for readability
    // microlamports / 1_000_000 = lamports per CU
    let lamports_per_cu = microlamports as f64 / 1_000_000.0;

    let mut details = HashMap::new();
    details.insert(
        "microlamports_per_cu".to_string(),
        microlamports.to_string(),
    );
    details.insert(
        "lamports_per_cu".to_string(),
        format!("{:.6}", lamports_per_cu),
    );

    // Flag unusually high priority fees (> 10M microlamports = 10 lamports/CU)
    let (risk_flags, severity) = if microlamports > 10_000_000 {
        (
            vec![
                format!("HIGH PRIORITY FEE: {} microlamports/CU", microlamports),
                "unusually high compute unit price".to_string(),
            ],
            Severity::Warning,
        )
    } else {
        (vec![], Severity::None)
    };

    DecodedInstruction {
        index,
        program: ProgramType::ComputeBudget,
        instruction_type: InstructionType::SetComputeUnitPrice,
        details,
        is_nonce_advance: false,
        risk_flags,
        severity,
    }
}

/// Fallback for Compute Budget instruction types we haven't implemented yet.
fn unknown_compute(index: usize, ix_type: u8) -> DecodedInstruction {
    let mut details = HashMap::new();
    details.insert("ix_type".to_string(), ix_type.to_string());

    DecodedInstruction {
        index,
        program: ProgramType::ComputeBudget,
        instruction_type: InstructionType::Unknown(format!("compute_ix_{}", ix_type)),
        details,
        is_nonce_advance: false,
        risk_flags: vec![format!("unknown compute budget instruction: {}", ix_type)],
        severity: Severity::Warning,
    }
}

/// Fallback for instructions with insufficient data bytes.
fn invalid(index: usize) -> DecodedInstruction {
    let mut details = HashMap::new();
    details.insert("error".to_string(), "insufficient data bytes".to_string());

    DecodedInstruction {
        index,
        program: ProgramType::ComputeBudget,
        instruction_type: InstructionType::Unknown("invalid".to_string()),
        details,
        is_nonce_advance: false,
        risk_flags: vec![],
        severity: Severity::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_compute_unit_limit() {
        // [2, 104, 41, 0, 0]
        // discriminator = 2
        // units = [104, 41, 0, 0] = 10_600 CU
        let data = [2u8, 104, 41, 0, 0];
        let result = decode(0, &data, &[], &[]);
        println!("compute unit limit:{:#?}", result);

        assert!(result.risk_flags.is_empty());
        assert_eq!(result.details["compute_units"], "10600");
        assert_eq!(result.severity, Severity::None);
    }

    #[test]
    fn test_set_compute_unit_price() {
        // real data from test transaction
        // [3, 160, 134, 1, 0, 0, 0, 0, 0]
        // discriminator = 3
        // microlamports = [160, 134, 1, 0, 0, 0, 0, 0] = 100_000
        let data = [3u8, 160, 134, 1, 0, 0, 0, 0, 0];
        let result = decode(0, &data, &[], &[]);
        println!("compute unit price:{:#?}", result);

        assert!(result.risk_flags.is_empty());
        assert_eq!(result.details["microlamports_per_cu"], "100000");
        assert_eq!(result.severity, Severity::None);
    }

    #[test]
    fn test_very_low_cu_limit_flagged() {
        // 500 CU = suspiciously low
        // little endian: [244, 1, 0, 0]
        let data = [2u8, 244, 1, 0, 0];
        let result = decode(0, &data, &[], &[]);
        println!("low cu limit:{:#?}", result);

        assert_eq!(result.severity, Severity::Warning);
        assert!(
            result
                .risk_flags
                .iter()
                .any(|f| f.contains("VERY LOW COMPUTE LIMIT"))
        );
    }

    #[test]
    fn test_high_priority_fee_flagged() {
        // 50_000_000 microlamports = very high
        // little endian u64: [128, 150, 152, 2, 0, 0, 0, 0]
        let data = [3u8, 128, 150, 152, 2, 0, 0, 0, 0];
        let result = decode(0, &data, &[], &[]);
        println!("high priority fee:{:#?}", result);

        assert_eq!(result.severity, Severity::Warning);
        assert!(
            result
                .risk_flags
                .iter()
                .any(|f| f.contains("HIGH PRIORITY FEE"))
        );
    }
}
