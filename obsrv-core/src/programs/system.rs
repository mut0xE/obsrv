use std::collections::HashMap;

use solana_sdk::native_token::LAMPORTS_PER_SOL;

use crate::types::{DecodedInstruction, InstructionType, ProgramType, Severity};

// SYSTEM PROGRAM DECODER
// Data format:
// bytes 0-3: instruction type as u32 little endian

pub fn decode(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    if data.len() < 4 {
        return invalid(index);
    }

    // read instruction type from first 4 bytes
    // System Program uses u32 little endian
    let ix_type = u32::from_le_bytes(data[..4].try_into().unwrap_or([0; 4]));
    match ix_type {
        0 => create_account(index, data, accounts, account_keys),
        2 => transfer(index, data, accounts, account_keys),
        4 => nonce_advance(index, accounts, account_keys),
        _ => unknown_system(index, ix_type),
    }
}

// TYPE 0: CreateAccount
// creates a new account on chain
// 80 bytes space = nonce account creation
/*
data layout for CreateAccount:
[0..4]   = discriminator instruction type
[4..12]  = lamports as u64 little endian
[12..20] = space (bytes to allocate) as u64 little endian
[20..52] = owner program id (32 bytes)
*/
fn create_account(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    // CreateAccount needs at least 52 bytes: 4 disc + 8 lamports + 8 space + 32 owner
    if data.len() < 52 {
        return invalid(index);
    }

    let from = get_account(accounts, 0, account_keys);
    let new_account = get_account(accounts, 1, account_keys);

    // lamports at bytes 4-11
    let lamports = read_u64(data, 4);

    // space at bytes 12-19
    let space = read_u64(data, 12);

    // owner program id at bytes 20-51
    let owner = bs58::encode(&data[20..52]).into_string();

    let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;

    let mut details = HashMap::new();
    details.insert("from".to_string(), from);
    details.insert("new_account".to_string(), new_account);
    details.insert("lamports".to_string(), lamports.to_string());
    details.insert("sol".to_string(), format!("{:.6}", sol));
    details.insert("space_bytes".to_string(), space.to_string());
    details.insert("owner".to_string(), owner);

    // 80 bytes = nonce account being created
    let (risk_flags, severity) = if space == 80 {
        (
            vec![
                "NONCE ACCOUNT CREATION DETECTED".to_string(),
                "80 bytes = durable nonce account".to_string(),
            ],
            Severity::Critical,
        )
    } else {
        (vec![], Severity::None)
    };

    DecodedInstruction {
        index,
        program: ProgramType::System,
        instruction_type: InstructionType::CreateAccount,
        details,
        is_nonce_advance: false,
        risk_flags,
        severity,
    }
}

// TYPE 2: Transfer
// moves SOL from one account to another
/*
data layout:
[0..4]  = discriminator (2, 0, 0, 0)
[4..12] = lamports as u64 little endian
*/
fn transfer(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    // Transfer needs at least 12 bytes: 4 disc + 8 lamports
    if data.len() < 12 {
        return invalid(index);
    }

    let from = get_account(accounts, 0, account_keys);
    let to = get_account(accounts, 1, account_keys);

    // lamports at bytes 4-11
    let lamports = read_u64(data, 4);

    let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;

    let mut details = HashMap::new();
    details.insert("from".to_string(), from);
    details.insert("to".to_string(), to);
    details.insert("lamports".to_string(), lamports.to_string());
    details.insert("sol".to_string(), format!("{:.6}", sol));

    let (risk_flags, severity) = if lamports > 10 * LAMPORTS_PER_SOL {
        (
            vec![
                format!("LARGE TRANSFER: {:.2} SOL", sol),
                "verify recipient address carefully".to_string(),
            ],
            Severity::Warning,
        )
    } else {
        (vec![], Severity::None)
    };
    DecodedInstruction {
        index,
        program: ProgramType::System,
        instruction_type: InstructionType::Transfer,
        details,
        is_nonce_advance: false,
        risk_flags,
        severity,
    }
}

// TYPE 4: NonceAdvance
// THE KEY DURABLE NONCE SIGNAL
// instruction[0] being nonceAdvance = tx never expires
/// # Account references
///   0. `[WRITE]` Nonce account
///   1. `[]` RecentBlockhashes sysvar
///   2. `[SIGNER]` Nonce authority
fn nonce_advance(index: usize, accounts: &[u8], account_keys: &[String]) -> DecodedInstruction {
    let nonce_account = get_account(accounts, 0, account_keys);
    let authority = get_account(accounts, 2, account_keys);

    let mut details = HashMap::new();
    details.insert("nonce_account".to_string(), nonce_account);

    details.insert("authority".to_string(), authority);

    details.insert(
        "warning".to_string(),
        "this transaction uses a durable nonce and may remain valid until the nonce is consumed"
            .to_string(),
    );

    DecodedInstruction {
        index,
        program: ProgramType::System,
        instruction_type: InstructionType::NonceAdvance,
        details,

        is_nonce_advance: true,

        risk_flags: vec![
            "DURABLE NONCE DETECTED".to_string(),
            "transaction validity is not time-limited like normal blockhash transactions"
                .to_string(),
            "transaction can remain executable until nonce is advanced".to_string(),
            "review delayed execution risk".to_string(),
        ],

        severity: Severity::Critical,
    }
}

/// read u64 little endian from data at offset
fn read_u64(data: &[u8], offset: usize) -> u64 {
    if data.len() >= offset + 8 {
        u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap_or([0; 8]))
    } else {
        0
    }
}

/// get account address by position in accounts array
fn get_account(accounts: &[u8], position: usize, account_keys: &[String]) -> String {
    accounts
        .get(position)
        .and_then(|&idx| account_keys.get(idx as usize))
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}

/// unhandled system instruction type
fn unknown_system(index: usize, ix_type: u32) -> DecodedInstruction {
    let mut details = HashMap::new();
    details.insert("ix_type".to_string(), ix_type.to_string());

    DecodedInstruction {
        index,
        program: ProgramType::System,
        instruction_type: InstructionType::Unknown(format!("system_ix_{}", ix_type)),
        details,
        is_nonce_advance: false,
        risk_flags: vec![format!("unhandled system instruction type: {}", ix_type)],
        severity: Severity::Warning,
    }
}

/// invalid instruction data
fn invalid(index: usize) -> DecodedInstruction {
    let mut details = HashMap::new();
    details.insert("error".to_string(), "insufficient data bytes".to_string());

    DecodedInstruction {
        index,
        program: ProgramType::System,
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

    fn test_accounts() -> Vec<String> {
        vec![
            "1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM".to_string(),
            "3Rz461rwuxY4mJzXtQWad1J1ZwKybeQPRa8BHTTLgSjG".to_string(),
            "6v6YQXkuLTfZjLU5KK7CV8j42MHwqwGQvrjDM5GTJxZV".to_string(),
            "11111111111111111111111111111111".to_string(),
        ]
    }

    #[test]
    fn test_create_account_normal() {
        let data = [
            0, 0, 0, 0, // type = CreateAccounts
            240, 29, 31, 0, 0, 0, 0, 0, // Rent
            165, 0, 0, 0, 0, 0, 0, 0, // Space
            6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180,
            133, 237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169, // Token Program
        ];
        let accounts = [0u8, 1u8];
        let keys = test_accounts();

        let result = decode(0, &data, &accounts, &keys);
        println!("result:{:#?}", result);

        assert!(result.risk_flags.is_empty());
        assert_eq!(result.details["space_bytes"], "165");
        println!("create account normal: {:?}", result.details);
    }

    #[test]
    fn test_create_nonce_account_flagged() {
        // space = 80 = nonce account
        let data = [
            0, 0, 0, 0, // discriminator = 0 = CreateAccount
            0, 23, 22, 0, 0, 0, 0, 0, // lamports = 1_447_680 (rent for 80 bytes)
            80, 0, 0, 0, 0, 0, 0, 0, // space = 80 ← NONCE SIZE → CRITICAL
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, // owner = system program
        ];
        let accounts = [0u8, 1u8];
        let keys = test_accounts();

        let result = decode(0, &data, &accounts, &keys);
        println!("result:{:#?}", result);

        assert_eq!(result.severity, Severity::Critical);
        assert!(
            result
                .risk_flags
                .iter()
                .any(|f| f.contains("NONCE ACCOUNT CREATION"))
        );
        println!("nonce creation flagged: {:?}", result.risk_flags);
    }

    #[test]
    fn test_transfer_small_amount() {
        // type 2, 1_000_000 lamports = 0.001 SOL
        let data = [2, 0, 0, 0, 64, 66, 15, 0, 0, 0, 0, 0];
        let accounts = [0u8, 1u8];
        let keys = test_accounts();

        let result = decode(0, &data, &accounts, &keys);
        println!("transfer result:{:#?}", result);

        assert!(result.risk_flags.is_empty());
        assert_eq!(result.details["sol"], "0.001000");
        assert_eq!(result.details["lamports"], "1000000");
        assert_eq!(result.details["from"], keys[0]); // sender
        assert_eq!(result.details["to"], keys[1]); // receiver
        println!("transfer: {:?}", result.details);
    }

    #[test]
    fn test_nonce_advance_detected() {
        // type 4 = nonceAdvance
        let data = [4, 0, 0, 0];
        let accounts = [3u8, 5u8, 1u8];
        let keys = vec![
            "11111114d3RrygbPdAtMuFnDmzsN8T5fYKVQ7FVr7".to_string(),
            "11111115q4EpJaTXAZWpCg3J2zppWGSZ46KXozzo9".to_string(),
            "111111152P2r5yt6odmBLPsFCLBrFisJ3aS7LqLAT".to_string(),
            "11111115RidqCHAoz6dzmXxGcfWLNzevYqNpaRAUo".to_string(),
            "11111111111111111111111111111111".to_string(),
            "SysvarRecentB1ockHashes11111111111111111111".to_string(),
        ];

        let result = decode(0, &data, &accounts, &keys);

        assert!(result.is_nonce_advance);
        assert_eq!(result.severity, Severity::Critical);
        assert!(
            result
                .risk_flags
                .iter()
                .any(|f| f.contains("DURABLE NONCE DETECTED"))
        );
        println!("nonce advance: {:#?}", result);
    }
}
