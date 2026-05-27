//! Token Program Public API
//!
//! This module provides the public interface for decoding token program instructions.
//! It routes to the appropriate decoder based on the program type.

use crate::{
    programs::token::{spl, token2022},
    types::DecodedInstruction,
};

/// Decodes an SPL Token program instruction.
///
/// # Arguments
/// * `index` - Position of this instruction in the transaction (0-indexed)
/// * `data` - Raw instruction data bytes (first byte is discriminator)
/// * `accounts` - Account index array (maps to account_keys)
/// * `account_keys` - Full list of account addresses in the transaction
pub fn decode_spl(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    spl::decode(index, data, accounts, account_keys)
}

/// Decodes a Token2022 program instruction.
///
/// # Arguments
/// * `index` - Position of this instruction in the transaction (0-indexed)
/// * `data` - Raw instruction data bytes (first byte is discriminator)
/// * `accounts` - Account index array (maps to account_keys)
/// * `account_keys` - Full list of account addresses in the transaction
pub fn decode_token2022(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    token2022::decode(index, data, accounts, account_keys)
}
