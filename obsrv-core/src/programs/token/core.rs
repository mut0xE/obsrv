//! Core Token Instruction Decoders
//!
//! This module contains instruction decoders shared by both SPL Token and Token2022.
//! These instructions have identical layouts and behavior across both programs.

use crate::{
    helpers::{get_account, read_u64},
    types::{DecodedInstruction, InstructionType, ProgramType, Severity},
};
use std::collections::HashMap;

/// TYPE 3: Transfer
///
/// Transfers tokens from one account to another.
///
/// # Data Layout (9 bytes total)
/// ```text
/// [0]    = 3 (instruction discriminator)
/// [1..9] = amount (u64, little-endian)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = source account (writable)
/// accounts[1] = destination account (writable)
/// accounts[2] = authority (signer, owner or delegate of source)
/// ```
///
/// # Security Notes
/// - Does NOT verify mint address — recipient could receive wrong token type
/// - Amount is raw units — must divide by token decimals for human-readable value
/// - TransferChecked (type 12) is safer as it verifies mint and includes decimals
pub fn transfer(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
    program: ProgramType,
) -> DecodedInstruction {
    if data.len() < 9 {
        return invalid(index, program);
    }

    let source = get_account(accounts, 0, account_keys);
    let dest = get_account(accounts, 1, account_keys);
    let authority = get_account(accounts, 2, account_keys);
    let amount = read_u64(data, 1);

    let mut details = HashMap::new();
    details.insert("source".to_string(), source);
    details.insert("destination".to_string(), dest);
    details.insert("authority".to_string(), authority);
    details.insert("amount".to_string(), amount.to_string());
    details.insert(
        "note".to_string(),
        "raw amount — divide by token decimals for human readable value".to_string(),
    );

    let (risk_flags, severity) = if amount > 1_000_000_000 {
        (
            vec![
                format!("LARGE TOKEN TRANSFER: {} raw units", amount),
                "verify token decimals for actual amount".to_string(),
                "Transfer (type 3) does not verify mint address".to_string(),
            ],
            Severity::Warning,
        )
    } else {
        (
            vec!["Transfer (type 3) does not verify mint address".to_string()],
            Severity::Info,
        )
    };

    DecodedInstruction {
        index,
        program,
        instruction_type: InstructionType::TokenTransfer,
        details,
        is_nonce_advance: false,
        risk_flags,
        severity,
    }
}

/// TYPE 12: TransferChecked
///
/// Safer version of Transfer that verifies mint address and includes decimals.
///
/// # Data Layout (10 bytes total)
/// ```text
/// [0]    = 12 (instruction discriminator)
/// [1..9] = amount (u64, little-endian, raw units)
/// [9]    = decimals (u8, for verification)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = source account (writable)
/// accounts[1] = mint account (verified to match source and destination)
/// accounts[2] = destination account (writable)
/// accounts[3] = authority (signer, owner or delegate)
/// ```
pub fn transfer_checked(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
    program: ProgramType,
) -> DecodedInstruction {
    if data.len() < 10 {
        return invalid(index, program);
    }

    let source = get_account(accounts, 0, account_keys);
    let mint = get_account(accounts, 1, account_keys);
    let destination = get_account(accounts, 2, account_keys);
    let authority = get_account(accounts, 3, account_keys);

    let amount = read_u64(data, 1);
    let decimals = data[9];

    let divisor = 10u64.pow(decimals as u32) as f64;
    let human_amount = amount as f64 / divisor;

    let mut details = HashMap::new();
    details.insert("source".to_string(), source);
    details.insert("mint".to_string(), mint.clone());
    details.insert("destination".to_string(), destination);
    details.insert("authority".to_string(), authority);
    details.insert("amount_raw".to_string(), amount.to_string());
    details.insert("decimals".to_string(), decimals.to_string());
    details.insert(
        "amount_human".to_string(),
        format!("{:.decimals$}", human_amount, decimals = decimals as usize),
    );

    let (risk_flags, severity) = if amount > 1_000_000_000 {
        (
            vec![
                format!(
                    "LARGE TOKEN TRANSFER: {} raw ({:.2} tokens)",
                    amount, human_amount
                ),
                format!("mint: {}", mint),
                "verify this transfer is intended".to_string(),
            ],
            Severity::Warning,
        )
    } else {
        (vec![], Severity::None)
    };

    DecodedInstruction {
        index,
        program,
        instruction_type: InstructionType::TokenTransferChecked,
        details,
        is_nonce_advance: false,
        risk_flags,
        severity,
    }
}

/// TYPE 4: Approve
///
/// Grants a delegate permission to transfer tokens on behalf of the owner.
///
/// # Data Layout (9 bytes total)
/// ```text
/// [0]    = 4 (instruction discriminator)
/// [1..9] = amount (u64, maximum amount delegate can transfer)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = source account (writable, token account being delegated)
/// accounts[1] = delegate (account being authorized to spend)
/// accounts[2] = owner (signer, owner of the source account)
/// ```
///
/// # Important Notes
/// - Delegate approval persists until consumed, revoked, or account closed
/// - No expiration time — approval is permanent until action taken
/// - Amount >= u64::MAX / 2 is considered "unlimited" approval
/// - Delegate can transfer approved amount at ANY time without further consent
/// - Multiple approvals can't coexist — new approval overwrites previous
pub fn approve(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
    program: ProgramType,
) -> DecodedInstruction {
    if data.len() < 9 {
        return invalid(index, program);
    }

    let source = get_account(accounts, 0, account_keys);
    let delegate = get_account(accounts, 1, account_keys);
    let owner = get_account(accounts, 2, account_keys);
    let amount = read_u64(data, 1);

    let mut details = HashMap::new();
    details.insert("source".to_string(), source);
    details.insert("delegate".to_string(), delegate.clone());
    details.insert("owner".to_string(), owner);
    details.insert("amount".to_string(), amount.to_string());

    let (risk_flags, severity) = if amount >= u64::MAX / 2 {
        (
            vec![
                "UNLIMITED DELEGATE APPROVAL DETECTED".to_string(),
                format!("delegate: {}", delegate),
                "approved amount: unlimited".to_string(),
                "delegate can drain entire token balance at any time".to_string(),
                "no expiry — approval remains until explicitly revoked".to_string(),
                "verify delegate program is audited and trusted".to_string(),
            ],
            Severity::Critical,
        )
    } else {
        (
            vec![
                format!("TOKEN DELEGATE APPROVAL: {} raw units", amount),
                format!("delegate authorized: {}", delegate),
                "delegate can execute transfers up to approved amount without further signatures"
                    .to_string(),
                "approval persists until consumed or revoked".to_string(),
            ],
            Severity::Warning,
        )
    };

    DecodedInstruction {
        index,
        program,
        instruction_type: InstructionType::TokenApprove,
        details,
        is_nonce_advance: false,
        risk_flags,
        severity,
    }
}

/// TYPE 9: CloseAccount
///
/// Closes a token account and sends remaining SOL rent to a destination.
///
/// # Data Layout (1 byte)
/// ```text
/// [0] = 9 (instruction discriminator, no additional data)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = token account to close (writable)
/// accounts[1] = destination (writable, receives the rent SOL)
/// accounts[2] = owner (signer, owner of the token account)
/// ```
///
/// # Prerequisites
/// - Token account balance must be ZERO
/// - No active delegates can exist
/// - Owner must sign the transaction
pub fn close_account(
    index: usize,
    accounts: &[u8],
    account_keys: &[String],
    program: ProgramType,
) -> DecodedInstruction {
    let account = get_account(accounts, 0, account_keys);
    let destination = get_account(accounts, 1, account_keys);
    let owner = get_account(accounts, 2, account_keys);

    let mut details = HashMap::new();
    details.insert("account".to_string(), account);
    details.insert("destination".to_string(), destination);
    details.insert("owner".to_string(), owner);

    DecodedInstruction {
        index,
        program,
        instruction_type: InstructionType::TokenCloseAccount,
        details,
        is_nonce_advance: false,
        risk_flags: vec![
            "TOKEN ACCOUNT BEING CLOSED".to_string(),
            "remaining SOL rent sent to destination".to_string(),
            "all tokens must be empty before closing".to_string(),
        ],
        severity: Severity::Warning,
    }
}

/// TYPE 6: SetAuthority
///
/// Changes the authority for a token account or mint.
///
/// # Data Layout (3-35 bytes)
/// ```text
/// [0]     = 6 (instruction discriminator)
/// [1]     = authority_type (u8)
///           0 = MintTokens (can mint new supply)
///           1 = FreezeAccount (can freeze/thaw accounts)
///           2 = AccountOwner (can transfer tokens)
///           3 = CloseAccount (can close the account)
/// [2]     = option byte (1 = new authority exists, 0 = removing authority)
/// [3..35] = new authority pubkey (32 bytes, only if option byte = 1)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = account or mint whose authority is changing (writable)
/// accounts[1] = current authority (signer)
/// ```
///
/// # Important Notes
/// - IRREVERSIBLE if removing authority (setting to None)
/// - New authority has FULL control over the specified capability
/// - No multisig or timelock protection
pub fn set_authority(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
    program: ProgramType,
) -> DecodedInstruction {
    if data.len() < 3 {
        return invalid(index, program);
    }

    let account = get_account(accounts, 0, account_keys);
    let current_authority = get_account(accounts, 1, account_keys);

    let authority_type = data[1];
    let has_new_authority = data[2] == 1;

    let new_authority = if has_new_authority && data.len() >= 35 {
        bs58::encode(&data[3..35]).into_string()
    } else {
        "None (authority being removed)".to_string()
    };

    let authority_type_name = match authority_type {
        0 => "MintTokens",
        1 => "FreezeAccount",
        2 => "AccountOwner",
        3 => "CloseAccount",
        _ => "Unknown",
    };

    let mut details = HashMap::new();
    details.insert("account".to_string(), account);
    details.insert("current_authority".to_string(), current_authority);
    details.insert("new_authority".to_string(), new_authority.clone());
    details.insert(
        "authority_type".to_string(),
        authority_type_name.to_string(),
    );

    let (risk_flags, severity) = if has_new_authority {
        (
            vec![
                format!("TOKEN AUTHORITY CHANGING: {}", authority_type_name),
                format!("new authority: {}", new_authority),
                "verify new authority is trusted".to_string(),
            ],
            Severity::Critical,
        )
    } else {
        (
            vec![format!("TOKEN AUTHORITY REMOVED: {}", authority_type_name)],
            Severity::Info,
        )
    };

    DecodedInstruction {
        index,
        program,
        instruction_type: InstructionType::TokenSetAuthority,
        details,
        is_nonce_advance: false,
        risk_flags,
        severity,
    }
}

/// Fallback for instruction types we haven't implemented yet.
pub fn unknown_token(
    index: usize,
    ix_type: u8,
    program: ProgramType,
    program_name: &str,
) -> DecodedInstruction {
    let mut details = HashMap::new();
    details.insert("ix_type".to_string(), ix_type.to_string());

    DecodedInstruction {
        index,
        program,
        instruction_type: InstructionType::Unknown(format!("{}_ix_{}", program_name, ix_type)),
        details,
        is_nonce_advance: false,
        risk_flags: vec![format!(
            "unhandled {} instruction: {}",
            program_name, ix_type
        )],
        severity: Severity::Warning,
    }
}

/// Fallback for instructions with insufficient data bytes.
pub fn invalid(index: usize, program: ProgramType) -> DecodedInstruction {
    let mut details = HashMap::new();
    details.insert("error".to_string(), "insufficient data bytes".to_string());

    DecodedInstruction {
        index,
        program,
        instruction_type: InstructionType::Unknown("invalid".to_string()),
        details,
        is_nonce_advance: false,
        risk_flags: vec![],
        severity: Severity::None,
    }
}
