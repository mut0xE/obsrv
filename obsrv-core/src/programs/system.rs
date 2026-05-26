//! System Program Decoder (`11111111111111111111111111111111`)
//!
//! The System Program is Solana's built-in program for fundamental account operations.
//! Every Solana account is initially owned by the System Program, and it handles:
//!
//! - **Account creation** — allocating space on-chain and assigning ownership
//! - **SOL transfers** — moving lamports between accounts
//! - **Nonce operations** — durable transaction nonces for offline/delayed signing
//!
//! # Binary Format
//!
//! All System Program instructions use the same prefix format:
//! ```text
//! [0..4] = instruction discriminator (u32 little-endian)
//! [4..]  = instruction-specific data
//! ```
//!
//! # Instruction Types (discriminator values)
//!
//! | Discriminator | Instruction           | Risk Level |
//! |---------------|-----------------------|------------|
//! | 0             | CreateAccount         | None/Critical (nonce = 80 bytes) |
//! | 1             | Assign                | Not implemented |
//! | 2             | Transfer              | None/Warning (>10 SOL) |
//! | 3             | CreateAccountWithSeed | Not implemented |
//! | 4             | AdvanceNonceAccount   | Critical |
//! | 5             | WithdrawNonceAccount  | Critical |
//! | 6             | InitializeNonceAccount| Critical |
//! | 7             | AuthorizeNonceAccount | Critical |
//! | 8             | Allocate              | Not implemented |
//! | 9             | AllocateWithSeed      | Not implemented |
//! | 10            | AssignWithSeed        | Not implemented |
//! | 11            | TransferWithSeed      | Not implemented |
//! | 12            | UpgradeNonceAccount   | Not implemented |
//!
//! # Why Nonce Instructions Are Critical
//!
//! Normal Solana transactions expire after ~60-90 seconds (when the blockhash ages out).
//! Durable nonces — a nonce transaction stays valid **forever** until the
//! nonce is advanced. This is used legitimately for multisig/offline signing, but
//! attackers exploit it to create transactions that can be executed at any time.
//! That's why all nonce-related instructions are flagged as Critical.

use std::collections::HashMap;

use solana_sdk::native_token::LAMPORTS_PER_SOL;

use crate::types::{DecodedInstruction, InstructionType, ProgramType, Severity};

/// Entry point for System Program instruction decoding.
///
/// Called by `programs::decode_instruction()` when the program ID matches the System Program.
///
/// # How It Works
///
/// Every System Program instruction starts with a 4-byte discriminator (u32 little-endian)
/// that tells us which instruction type it is. We read those 4 bytes, match on the value,
/// and route to the appropriate decoder function.
///
/// # Arguments
/// * `index` — position of this instruction in the transaction (0 = first)
/// * `data` — raw instruction data bytes (discriminator + instruction-specific payload)
/// * `accounts` — array of account indices that this instruction references.
///                Each value is an index into `account_keys`.
///                Example: `[0, 1]` means "use account_keys[0] and account_keys[1]"
/// * `account_keys` — all account addresses in the transaction (base58 strings)
///
/// # The `accounts` Indirection
///
/// This is often confusing. Here's how it works:
/// ```text
/// account_keys = ["ABC...", "DEF...", "GHI...", "111..."]
///                   idx 0     idx 1     idx 2     idx 3
///
/// accounts = [0, 2]  ← this instruction uses account_keys[0] and account_keys[2]
///             │  │
///             │  └─ position 1 = "GHI..." (e.g. new_account for CreateAccount)
///             └─ position 0 = "ABC..." (e.g. from/funder for CreateAccount)
/// ```
///
/// Each instruction type defines which position means what (documented per function).
pub fn decode(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    // Need at least 4 bytes to read the discriminator
    if data.len() < 4 {
        return invalid(index);
    }

    // Read the instruction type from the first 4 bytes (u32 little-endian)
    let ix_type = u32::from_le_bytes(data[..4].try_into().unwrap_or([0; 4]));

    match ix_type {
        0 => create_account(index, data, accounts, account_keys),
        2 => transfer(index, data, accounts, account_keys),
        4 => nonce_advance(index, accounts, account_keys),
        5 => nonce_withdraw(index, data, accounts, account_keys),
        6 => nonce_initialize(index, data, accounts, account_keys),
        7 => nonce_authorize(index, data, accounts, account_keys),
        _ => unknown_system(index, ix_type),
    }
}

/// Discriminator 0: CreateAccount
///
/// Creates a new account on-chain. The funder pays rent + allocation cost,
/// and the new account is assigned to an owner program.
///
/// # Data Layout (52 bytes total)
/// ```text
/// [0..4]   = discriminator (0, 0, 0, 0)
/// [4..12]  = lamports to transfer to new account (u64 LE) — covers rent
/// [12..20] = space in bytes to allocate (u64 LE) — how much storage
/// [20..52] = owner program ID (32 bytes) — who controls this account
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = funder (pays for account creation, must sign)
/// accounts[1] = new account address (must sign)
/// ```
///
/// # Nonce Detection
///
/// If `space == 80`, this is creating a **durable nonce account**. The nonce account
/// struct is exactly 80 bytes (NonceState layout). This is a critical signal because
/// it means someone is setting up infrastructure for transactions that never expire.
///
/// Common use: multisig wallets, hardware wallet offline signing.
fn create_account(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    // Minimum 52 bytes: 4 (disc) + 8 (lamports) + 8 (space) + 32 (owner pubkey)
    if data.len() < 52 {
        return invalid(index);
    }

    // accounts[0] = funder who pays for the new account
    let from = get_account(accounts, 0, account_keys);
    // accounts[1] = the new account being created
    let new_account = get_account(accounts, 1, account_keys);

    // How many lamports to deposit into the new account (covers rent-exemption)
    let lamports = read_u64(data, 4);

    // How many bytes of storage to allocate for the account's data field
    let space = read_u64(data, 12);

    // The program that will own this account (e.g. Token Program, System Program)
    // Raw 32 bytes → base58 string for human readability
    let owner = bs58::encode(&data[20..52]).into_string();

    let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;

    let mut details = HashMap::new();
    details.insert("from".to_string(), from);
    details.insert("new_account".to_string(), new_account);
    details.insert("lamports".to_string(), lamports.to_string());
    details.insert("sol".to_string(), format!("{:.6}", sol));
    details.insert("space_bytes".to_string(), space.to_string());
    details.insert("owner".to_string(), owner);

    // 80 bytes = nonce account (NonceState struct is exactly 80 bytes)
    let (risk_flags, severity) = if space == 80 {
        details.insert(
            "account_type".to_string(),
            "durable_nonce_account".to_string(),
        );

        details.insert("rent_exempt".to_string(), "true".to_string());
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

/// Discriminator 2: Transfer
///
/// Moves SOL (lamports) from one account to another. The simplest and most common
/// System Program instruction on Solana.
///
/// # Data Layout (12 bytes total)
/// ```text
/// [0..4]  = discriminator (2, 0, 0, 0)
/// [4..12] = lamports to transfer (u64 LE)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = sender (must sign, debited)
/// accounts[1] = recipient (credited)
/// ```
///
/// # Risk Flagging
///
/// Transfers over 10 SOL are flagged as Warning severity. This threshold is arbitrary
/// but catches most "significant" transfers. The user should always verify the
/// recipient address regardless of amount, but large amounts deserve extra attention.
///
/// # Lamports vs SOL
/// 1 SOL = 1,000,000,000 lamports (1e9). All on-chain values are in lamports.
fn transfer(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    // Minimum 12 bytes: 4 (disc) + 8 (lamports)
    if data.len() < 12 {
        return invalid(index);
    }

    // accounts[0] = sender (signer, pays the lamports)
    let from = get_account(accounts, 0, account_keys);
    // accounts[1] = recipient (receives the lamports)
    let to = get_account(accounts, 1, account_keys);

    // Amount to transfer in lamports (1 SOL = 1_000_000_000 lamports)
    let lamports = read_u64(data, 4);

    let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;

    let mut details = HashMap::new();
    details.insert("from".to_string(), from);
    details.insert("to".to_string(), to);
    details.insert("lamports".to_string(), lamports.to_string());
    details.insert("sol".to_string(), format!("{:.6}", sol));

    // Flag large transfers (>10 SOL) for user review
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

/// Discriminator 4: AdvanceNonceAccount
///
/// **THE most important instruction for this tool to detect.**
///
/// When a transaction's **first instruction** is NonceAdvance, that transaction uses a
/// durable nonce instead of a recent blockhash. This means:
///
/// - Normal tx: expires in ~60-90 seconds (blockhash ages out)
/// - Nonce tx: **never expires** until the nonce value is advanced
///
/// The `is_nonce_advance: true` flag on this instruction is what the report builder
/// checks to set `TransactionReport.is_durable_nonce = true`.
///
/// # Data Layout (4 bytes — discriminator only, no extra data)
/// ```text
/// [0..4] = discriminator (4, 0, 0, 0)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = nonce account (WRITE) — the nonce value gets advanced
/// accounts[1] = RecentBlockhashes sysvar (read-only, we skip it)
/// accounts[2] = nonce authority (SIGNER) — must authorize the advance
/// ```
fn nonce_advance(index: usize, accounts: &[u8], account_keys: &[String]) -> DecodedInstruction {
    // accounts[0] = the nonce account whose stored value gets replaced
    let nonce_account = get_account(accounts, 0, account_keys);
    // accounts[2] = who has authority over this nonce (must sign)
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
        // This is the ONLY instruction where is_nonce_advance = true
        // The report builder checks instructions[0].is_nonce_advance to determine
        // if the entire transaction is a durable nonce transaction
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

/// Discriminator 6: InitializeNonceAccount
///
/// Sets up a nonce account after it has been created with CreateAccount(space=80).
/// These two instructions always appear together in the same transaction:
///
/// ```text
/// ix[0] = CreateAccount { space: 80, ... }  allocates the account
/// ix[1] = InitializeNonce { authority }     initializes nonce state
/// ```
///
/// After initialization, the nonce account stores:
/// - A nonce value (derived from the latest blockhash at init time)
/// - The authority pubkey (who can advance/withdraw/authorize)
///
/// # Data Layout (36 bytes total)
/// ```text
/// [0..4]  = discriminator (6, 0, 0, 0)
/// [4..36] = authority pubkey (32 bytes) — who controls this nonce account
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = nonce account (WRITE) — being initialized
/// accounts[1] = RecentBlockhashes sysvar (skip)
/// accounts[2] = Rent sysvar (skip)
/// ```
fn nonce_initialize(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    // accounts[0] = the nonce account being initialized
    let nonce_account = get_account(accounts, 0, account_keys);

    // Authority pubkey is embedded in the data at bytes 4-35 (32 bytes)
    let authority = if data.len() >= 36 {
        bs58::encode(&data[4..36]).into_string()
    } else {
        "unknown".to_string()
    };

    let mut details = HashMap::new();
    details.insert("nonce_account".to_string(), nonce_account);
    details.insert("authority".to_string(), authority);

    DecodedInstruction {
        index,
        program: ProgramType::System,
        instruction_type: InstructionType::NonceInitialize,
        details,
        is_nonce_advance: false,
        risk_flags: vec![
            "DURABLE NONCE ACCOUNT CREATED".to_string(),
            "future transactions may use durable nonces".to_string(),
            "offline or delayed transaction signing possible".to_string(),
        ],
        severity: Severity::Critical,
    }
}

/// Discriminator 5: WithdrawNonceAccount
///
/// Withdraws SOL (lamports) from a nonce account. If the entire balance is withdrawn,
/// the nonce account is effectively closed (it can no longer be used).
///
/// # Data Layout (12 bytes total)
/// ```text
/// [0..4]  = discriminator (5, 0, 0, 0)
/// [4..12] = lamports to withdraw (u64 LE)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = nonce account (WRITE) — being drained
/// accounts[1] = destination (WRITE) — receives the lamports
/// accounts[2] = RecentBlockhashes sysvar (skip)
/// accounts[3] = Rent sysvar (skip)
/// accounts[4] = nonce authority (SIGNER)
/// ```
fn nonce_withdraw(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    // accounts[0] = nonce account being drained
    let nonce_account = get_account(accounts, 0, account_keys);
    // accounts[1] = where the withdrawn lamports go
    let destination = get_account(accounts, 1, account_keys);
    // accounts[4] = authority who controls this nonce (must sign)
    // Note: accounts[2] and [3] are sysvars, we skip them
    let nonce_authority = get_account(accounts, 4, account_keys);

    // Amount being withdrawn
    let lamports = read_u64(data, 4);
    let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;

    let mut details = HashMap::new();
    details.insert("nonce_account".to_string(), nonce_account);
    details.insert("destination".to_string(), destination);
    details.insert("nonce_authority".to_string(), nonce_authority);
    details.insert("lamports".to_string(), lamports.to_string());
    details.insert("sol".to_string(), format!("{:.6}", sol));

    DecodedInstruction {
        index,
        program: ProgramType::System,
        instruction_type: InstructionType::NonceWithdraw,
        details,
        is_nonce_advance: false,
        risk_flags: vec![
            "NONCE ACCOUNT WITHDRAWAL".to_string(),
            "nonce account balance reduced".to_string(),
            "account may be closed if fully drained".to_string(),
        ],
        severity: Severity::Critical,
    }
}

/// Discriminator 7: AuthorizeNonceAccount
///
/// Changes the authority (controller) of a nonce account. After this instruction,
/// only the new authority can advance, withdraw from, or re-authorize the nonce.
///
/// # Data Layout (36 bytes total)
/// ```text
/// [0..4]  = discriminator (7, 0, 0, 0)
/// [4..36] = new authority pubkey (32 bytes)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = nonce account (WRITE)
/// accounts[1] = current nonce authority (SIGNER) — must approve the transfer
/// ```
fn nonce_authorize(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    // accounts[0] = the nonce account whose authority is being changed
    let nonce_account = get_account(accounts, 0, account_keys);
    // accounts[1] = current authority (must sign to approve the change)
    let current_authority = get_account(accounts, 1, account_keys);

    // New authority pubkey is embedded in instruction data at bytes 4-35
    let new_authority = if data.len() >= 36 {
        bs58::encode(&data[4..36]).into_string()
    } else {
        "unknown".to_string()
    };

    let mut details = HashMap::new();
    details.insert("nonce_account".to_string(), nonce_account);
    details.insert("current_authority".to_string(), current_authority);
    details.insert("new_authority".to_string(), new_authority);

    DecodedInstruction {
        index,
        program: ProgramType::System,
        instruction_type: InstructionType::NonceAuthorize,
        details,
        is_nonce_advance: false,
        risk_flags: vec![
            "NONCE AUTHORITY UPDATED".to_string(),
            "control of nonce account transferred".to_string(),
            "verify new authority is trusted".to_string(),
        ],
        severity: Severity::Critical,
    }
}

// Helper Functions
/// Reads a u64 value from a byte slice at the given offset (little-endian).
///
/// Solana stores all integer values as little-endian. This helper safely reads
/// 8 bytes starting at `offset` and converts them to a u64.
///
/// Returns 0 if there aren't enough bytes — callers should bounds-check
/// before calling if they need to distinguish "0 lamports" from "truncated data".
///
/// # Example
/// ```text
/// data = [2, 0, 0, 0,  64, 66, 15, 0, 0, 0, 0, 0]
///                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///                       offset=4, reads these 8 bytes
///                       = 1_000_000 (0.001 SOL)
/// ```
fn read_u64(data: &[u8], offset: usize) -> u64 {
    if data.len() >= offset + 8 {
        u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap_or([0; 8]))
    } else {
        0
    }
}

/// Resolves an account address from the accounts indirection array.
///
/// # How Account Indirection Works
///
/// Each instruction doesn't store full account addresses — it stores indices.
/// The `accounts` array contains positions into the transaction's `account_keys`.
///
/// ```text
/// account_keys = ["ABC...", "DEF...", "GHI...", "111..."]
/// accounts     = [0, 2]
///
/// get_account(accounts, 0, account_keys)  → "ABC..." (accounts[0] = 0 → account_keys[0])
/// get_account(accounts, 1, account_keys)  → "GHI..." (accounts[1] = 2 → account_keys[2])
/// ```
///
/// Returns "unknown" if the position is out of bounds
fn get_account(accounts: &[u8], position: usize, account_keys: &[String]) -> String {
    accounts
        .get(position)
        .and_then(|&idx| account_keys.get(idx as usize))
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}

/// Fallback for System Program instruction types we haven't implemented yet.
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

/// Fallback for instructions with insufficient data bytes.
///
/// If the instruction data is too short to even read the required fields
/// (e.g. CreateAccount with < 52 bytes), we return this instead of panicking.
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
        println!("create account normal: {:#?}", result.details);
    }

    #[test]
    fn test_create_nonce_account_flagged() {
        // space = 80 = nonce account
        let data = [
            0, 0, 0, 0, // discriminator = 0 = CreateAccount
            0, 23, 22, 0, 0, 0, 0, 0, // lamports = 1_447_680 (rent for 80 bytes)
            80, 0, 0, 0, 0, 0, 0, 0, // space = 80 ← NONCE SIZE → CRITICAL
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
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
        println!("nonce creation flagged: {:#?}", result.risk_flags);
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
        println!("transfer: {:#?}", result.details);
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

    #[test]
    fn test_nonce_initialize_flagged() {
        // ix[1] of the nonce creation transaction
        let data = [
            6, 0, 0, 0, // discriminator = 6 = InitializeNonce
            130, 93, 119, 115, 218, 75, 23, 202, 74, 214, 252, 55, 9, 0, 157, 61, 209, 22, 7, 169,
            50, 26, 90, 31, 229, 167, 89, 113, 202, 240, 43,
            85, // authority pubkey (32 bytes)
        ];

        // accounts: [1, 3, 4]
        // position 0 - nonce_account  = keys[1]
        // position 1 - sysvar_recent  = keys[3] (skip)
        // position 2 - sysvar_rent    = keys[4] (skip)
        let accounts = [1u8, 3u8, 4u8];

        // authority:     9mtfTWo46MoevHDmPjiikVvn8BbBBMngp9xhA8dibRL8

        let keys = vec![
            "4Qnzm1w24ZFSVZqpKcn3MsyTstRhwcFpt38AEXSPpur8".to_string(), // 0 payer
            "FWGVsL2HfMoahTNwCGtfY9CF98UsnbsjWD7dGPts7erC".to_string(), // 1 nonce account
            "11111111111111111111111111111111".to_string(),             // 2 system program
            "SysvarRecentB1ockHashes11111111111111111111".to_string(),  // 3 sysvar recent
            "SysvarRent111111111111111111111111111111111".to_string(),  // 4 sysvar rent
        ];

        let result = decode(0, &data, &accounts, &keys);
        println!("nonce_initialize:{:#?}", result);

        assert_eq!(result.severity, Severity::Critical);
        assert!(!result.is_nonce_advance);
        assert!(
            result
                .risk_flags
                .iter()
                .any(|f| f.contains("DURABLE NONCE ACCOUNT CREATED"))
        );
        assert_eq!(
            result.details["nonce_account"],
            "FWGVsL2HfMoahTNwCGtfY9CF98UsnbsjWD7dGPts7erC"
        );
    }

    #[test]
    fn test_nonce_withdraw_flagged() {
        let data = [
            5, 0, 0, 0, // discriminator = 5 = WithdrawNonce
            0, 23, 22, 0, 0, 0, 0, 0, // lamports = 1_447_680
        ];
        // accounts: [nonce_account, destination, sysvar_recent, sysvar_rent, authority]
        let accounts = [2, 3, 5, 6, 1];
        let keys = vec![
            "53xx4c7gLqsPo86g9j4TtVGAq6aLddgk5BMjm3tfmcJd".to_string(),
            "2Yayov2oWzdLDaWNwCPd6VoFToZqUAU9eTugW4v91Efh".to_string(),
            "4GTeyPuMiwNTCPiSe9UYauNcceeswKytswzqnd7GxvMD".to_string(),
            "7cAePWAD9Y6RJqanETuzySGUiwRZ9CrYp9C19gZwet3M".to_string(),
            "11111111111111111111111111111111".to_string(),
            "SysvarRecentB1ockHashes11111111111111111111".to_string(),
            "SysvarRent111111111111111111111111111111111".to_string(),
        ];

        let result = decode(0, &data, &accounts, &keys);
        println!("nonce_withdraw:{:#?}", result);

        assert_eq!(result.severity, Severity::Critical);
        assert!(
            result
                .risk_flags
                .iter()
                .any(|f| f.contains("NONCE ACCOUNT WITHDRAWAL"))
        );
        assert_eq!(result.details["lamports"], "1447680");
    }

    #[test]
    fn test_nonce_authorize_flagged() {
        let data = [
            7, 0, 0, 0, // discriminator = 7 = AuthorizeNonce
            202, 51, 148, 191, 86, 135, 59, 101, 234, 38, 209, 60, 62, 54, 111, 131, 59, 70, 150,
            63, 76, 55, 128, 140, 226, 88, 173, 54, 71, 2, 74,
            157, // new authority pubkey (32 bytes)
        ];
        let accounts = [2, 1];
        let keys = vec![
            "BiNpcmmorXgNhEMzXkW8NJkHMpGzWSFvuQx5DWJkvwNM".to_string(),
            "Ef1Vvfna4EomMVwTR2f4Hu7LhEjwKv273XKiAMWCKhhc".to_string(),
            "xns148xpEZ6wE81XGJt3TFK4uFvDsWgidFE5Qc1kh6d".to_string(),
            "11111111111111111111111111111111".to_string(),
        ];

        let result = decode(0, &data, &accounts, &keys);
        println!("nonce_authorize:{:#?}", result);

        assert_eq!(result.severity, Severity::Critical);
        assert!(
            result
                .risk_flags
                .iter()
                .any(|f| f.contains("NONCE AUTHORITY UPDATED"))
        );
        assert!(result.details.contains_key("new_authority"));
        assert!(result.details.contains_key("current_authority"));
    }
}
