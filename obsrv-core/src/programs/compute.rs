use std::collections::HashMap;

use crate::types::{DecodedInstruction, InstructionType, ProgramType, Severity};

// COMPUTE BUDGET PROGRAM DECODER
// Unlike System Program, uses u8 discriminator (1 byte)
// No accounts needed for any compute budget instruction
pub fn decode(
    index: usize,
    data: &[u8],
    _accounts: &[u8], // always empty for compute budget
    _account_keys: &[String],
) -> DecodedInstruction {
    if data.len() < 1 {
        return invalid(index);
    }

    // compute budget uses u8 discriminator (just first byte)
    let ix_type = data[0];

    match ix_type {
        2 => set_compute_unit_limit(index, data),
        3 => set_compute_unit_price(index, data),
        _ => unknown_compute(index, ix_type),
    }
}

// TYPE 2: SetComputeUnitLimit
// how many compute units this tx can use
// normal range: 200_000 (default) to 1_400_000 (max)
/*
data layout:
[0]    = discriminator (2)
[1..5] = units as u32 little endian
*/
fn set_compute_unit_limit(index: usize, data: &[u8]) -> DecodedInstruction {
    if data.len() < 5 {
        return invalid(index);
    }

    // units at bytes 1-4 as u32 little endian
    let units = u32::from_le_bytes(data[1..5].try_into().unwrap_or([0; 4]));

    let mut details = HashMap::new();
    details.insert("compute_units".to_string(), units.to_string());

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

// TYPE 3: SetComputeUnitPrice
// sets priority fee in microlamports per compute unit
// higher = faster processing
/*
data layout:
[0]    = discriminator (3)
[1..9] = microlamports as u64 little endian
*/
fn set_compute_unit_price(index: usize, data: &[u8]) -> DecodedInstruction {
    if data.len() < 9 {
        return invalid(index);
    }

    // price at bytes 1-8 as u64 little endian
    let microlamports = u64::from_le_bytes(data[1..9].try_into().unwrap_or([0; 8]));

    // convert to SOL per CU for display
    // microlamports / 1_000_000 = lamports
    // lamports / 1_000_000_000 = SOL
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
