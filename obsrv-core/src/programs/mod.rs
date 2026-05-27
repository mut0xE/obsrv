use crate::types::DecodedInstruction;

pub mod compute;
pub mod system;
pub mod token;

pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

pub const SPL_TOKEN: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

pub const TOKEN_2022: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

pub const COMPUTE_BUDGET: &str = "ComputeBudget111111111111111111111111111111";

pub fn decode_instruction(
    index: usize,
    program_id: &str,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    match program_id {
        SYSTEM_PROGRAM => system::decode(index, data, accounts, account_keys),
        SPL_TOKEN => token::spl::decode(index, data, accounts, account_keys),
        TOKEN_2022 => token::token2022::decode(index, data, accounts, account_keys),
        COMPUTE_BUDGET => compute::decode(index, data, accounts, account_keys),
        _ => unknown_program(index, program_id, data),
    }
}

fn unknown_program(index: usize, program_id: &str, data: &[u8]) -> DecodedInstruction {
    use crate::types::{InstructionType, ProgramType, Severity};
    use std::collections::HashMap;

    let mut details = HashMap::new();
    details.insert("program_id".to_string(), program_id.to_string());
    details.insert("data_length".to_string(), data.len().to_string());

    DecodedInstruction {
        index,
        program: ProgramType::Unknown(program_id.to_string()),
        instruction_type: InstructionType::Unknown(
            program_id[..8.min(program_id.len())].to_string(),
        ),
        details,
        is_nonce_advance: false,
        risk_flags: vec![format!("unknown program: {}", program_id)],
        severity: Severity::Warning,
    }
}
