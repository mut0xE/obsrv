//! Token2022 (Token Extensions) Program Decoder
//!
//! Program ID: TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
//!
//! Token2022 is Solana's extended token program that includes all SPL Token
//! functionality plus additional extensions for advanced use cases.
//!
//! # Key Extensions
//! - Transfer fees (protocol can take a cut of transfers)
//! - Permanent delegates (bypass owner approval)
//! - Interest-bearing tokens (dynamic supply based on time)
//! - Non-transferable tokens (soulbound)
//! - Mint close authority (recover mint account rent)
//! - Transfer hooks (custom program logic on transfer)
//! - Metadata pointer (on-chain metadata)
//! - Group and member pointers (token collections)

use super::core;
use crate::{
    helpers::{get_account, read_u64},
    types::{DecodedInstruction, InstructionType, ProgramType, Severity},
};
use std::collections::HashMap;

/// Entry point for Token2022 instruction decoding.
///
/// # Shared Instructions (0-23)
/// Token2022 shares instruction discriminators with SPL Token for compatibility.
///
/// # Token2022 Extensions (24+)
/// - 25: PermanentDelegateTransfer (implemented below)
/// - 26: InitializeTransferFee
/// - 27: TransferWithFee
/// - Additional extensions as needed
pub fn decode(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    if data.is_empty() {
        return core::invalid(index, ProgramType::SplToken);
    }

    let ix_type = data[0];

    match ix_type {
        // shared with SPL Token
        3 => core::transfer(index, data, accounts, account_keys, ProgramType::Token2022),
        4 => core::approve(index, data, accounts, account_keys, ProgramType::Token2022),
        6 => core::set_authority(index, data, accounts, account_keys, ProgramType::Token2022),
        9 => core::close_account(index, accounts, account_keys, ProgramType::Token2022),
        12 => core::transfer_checked(index, data, accounts, account_keys, ProgramType::Token2022),

        // Token2022 only extensions
        25 => permanent_delegate_transfer(index, data, accounts, account_keys),

        _ => core::unknown_token(index, ix_type, ProgramType::Token2022, "token2022"),
    }
}

/// TYPE 25: PermanentDelegateTransfer
///
/// Allows a mint-level permanent delegate to transfer tokens WITHOUT the owner's
/// signature or approval. This completely bypasses normal token authorization.
///
/// # Data Layout (9 bytes total)
/// ```text
/// [0]    = 25 (instruction discriminator)
/// [1..9] = amount (u64, raw units)
/// ```
///
/// # Account References
/// ```text
/// accounts[0] = source account (writable)
/// accounts[1] = mint (must have permanent delegate extension enabled)
/// accounts[2] = destination account (writable)
/// accounts[3] = permanent delegate authority (signer, set at mint level)
/// ```
///
/// # Security Notes
/// - Permanent delegate has unlimited power over ALL token accounts
/// - No user consent required for transfers
/// - Cannot be disabled once set at mint creation
/// - Centralization risk — single point of failure
fn permanent_delegate_transfer(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    let source = get_account(accounts, 0, account_keys);
    let dest = get_account(accounts, 1, account_keys);
    let delegate = get_account(accounts, 2, account_keys);
    let amount = read_u64(data, 1);

    let mut details = HashMap::new();
    details.insert("source".to_string(), source);
    details.insert("destination".to_string(), dest);
    details.insert("permanent_delegate".to_string(), delegate.clone());
    details.insert("amount".to_string(), amount.to_string());

    DecodedInstruction {
        index,
        program: ProgramType::Token2022,
        instruction_type: InstructionType::TokenTransfer,
        details,
        is_nonce_advance: false,
        risk_flags: vec![
            "PERMANENT DELEGATE TRANSFER".to_string(),
            format!("delegate {} transferred without owner approval", delegate),
            "Token2022 permanent delegate bypasses normal authorization".to_string(),
            "extremely dangerous token extension".to_string(),
        ],
        severity: Severity::Critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token2022_transfer_checked_real_tx() {
        // standard transfer_checked works the same as SPL Token
        let data = [12u8, 64, 66, 15, 0, 0, 0, 0, 0, 6];
        let accounts = [2u8, 5u8, 3u8, 1u8];
        let keys = vec![
            "G5ys7CgwiZP7QHUL5d51DnccHw3QywzXzVPL6U3obX5x".to_string(),
            "ZQokWwjjLgcysiVtrPdizX7CgHMGcaLBTXXSa2yET4M".to_string(),
            "DtNs6LdfqsyyzQ7RP4TYuJoC8B371aEAoy3TzCeHSHC7".to_string(),
            "GPkcyQcti9Z94EA9x28ppTGyXmChcjLFkstUZNSmDTdV".to_string(),
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".to_string(),
            "7wjB1wAoHSCfS4AeT6rdEo8QtuB1U3RoxdrCv8AhZQPN".to_string(),
        ];

        let result = decode(0, &data, &accounts, &keys);

        assert_eq!(result.details["amount_raw"], "1000000");
        assert_eq!(result.details["decimals"], "6");
        assert_eq!(result.details["amount_human"], "1.000000");
        assert_eq!(result.details["mint"], keys[5]);
        assert_eq!(result.details["source"], keys[2]);
    }
}
