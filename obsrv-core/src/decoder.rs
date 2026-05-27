//! Transaction Decoder Module
//!
//! This module handles the first stage of transaction analysis: taking a raw string
//! input (which could be base64 or base58 encoded) and converting it into a structured
//! Solana transaction type that our program decoders can work with.
//!
//! # Design Philosophy
//!
//! Users paste transaction data from wallets, dApps, or the Solana CLI in various formats.
//! Rather than making them tell us what format they're using, we detect it automatically:
//!
//! - **Encoding detection**: Try base64 first (superset), fall back to base58
//! - **Type detection**: Try VersionedTransaction → Transaction → Message
//! - **Validation**: Reject signatures, oversized data, malformed structures early
//!
//! This defensive approach gives users helpful errors instead of cryptic deserialization failures.

use base64::{Engine, engine::general_purpose::STANDARD};
use solana_sdk::{
    message::Message,
    transaction::{Transaction, VersionedTransaction},
};

use crate::errors::ObsrvError;
use crate::programs::decode_instruction;
use crate::types::DecodedInstruction;

/// Max characters accepted in raw input string.
const MAX_INPUT_CHARS: usize = 10_000;

/// Solana's maximum transaction size (MTU of UDP packet). Anything larger is invalid.
const MAX_TX_BYTES: usize = 1232;

/// Enum for classifying user input (currently unused, reserved for future input routing).
pub enum InputType {
    RawTransaction,
    TxSignature,
    WalletAddress,
    Unknown,
}

/// The result of successfully decoding a raw transaction string.
///
/// This enum represents the three possible Solana transaction types we might receive.
/// The variant tells us which format the user's input was in, and we handle each
/// differently when extracting instructions and accounts.
#[derive(Debug, Clone)]
pub enum DecodedPayload {
    /// Modern transaction format (v0) with Address Lookup Table support
    VersionedTransaction(VersionedTransaction),
    /// Legacy transaction format (pre-v0)
    Transaction(Transaction),
    /// Unsigned message (no signatures), typically from dApps before user signs
    Message(Message),
}

/// Main entry point: takes a raw string and returns a decoded Solana transaction.
///
/// # Arguments
/// * `input` - Raw transaction string (base64 or base58 encoded)
///
/// # Returns
/// * `Ok(DecodedPayload)` - Successfully decoded transaction in one of three formats
/// * `Err(ObsrvError)` - Input validation failed or deserialization failed
///
/// # Example
/// ```ignore
/// let tx_string = "AQABAwAAAAAAAAAGAA...";  // from wallet
/// let payload = decode_payload(tx_string)?;
/// match payload {
///     DecodedPayload::VersionedTransaction(tx) => { /* handle v0 */ }
///     DecodedPayload::Transaction(tx) => { /* handle legacy */ }
///     DecodedPayload::Message(msg) => { /* handle unsigned */ }
/// }
/// ```
pub fn decode_payload(input: &str) -> Result<DecodedPayload, ObsrvError> {
    // Step 1: Clean whitespace, check length limits
    let input = normalize_input(input)?;

    // Step 2: Reject transaction signatures
    if looks_like_signature(input) {
        return Err(ObsrvError::DeserializationFailed(
            "signature detected, not a transaction payload".to_string(),
        ));
    }

    // Step 3: Decode base64 or base58 to raw bytes
    let bytes = decode_to_bytes(input)?;

    // Step 4: Validate byte size against Solana's 1232 byte limit
    validate_byte_size(&bytes)?;

    // Step 5: Deserialize bytes into VersionedTransaction, Transaction, or Message
    let payload = deserialize_payload(&bytes);
    println!("deserialize payload:{:#?}", &payload);
    payload
}

/// Takes a DecodedPayload and decodes each instruction through the program router.
/// Returns (account_keys, decoded_instructions) for building a TransactionReport.
///
/// # Purpose
/// This is the bridge between raw Solana structs (VersionedTransaction, Transaction, Message)
/// and business logic (system program decoder, token decoder, etc). It handles the
/// differences in how each type stores instructions and accounts, giving you a uniform output.
///
/// # Usage Example
/// ```ignore
/// let payload = decode_payload(user_input)?;
/// let (account_keys, instructions) = decode_instructions(&payload);
///
/// // Now build the TransactionReport
/// let report = TransactionReport {
///     account_keys,
///     instructions,
///     fee_payer: account_keys.get(0).unwrap_or(&"unknown".to_string()).clone(),
///     // ... fill in risk analysis, nonce detection, etc
/// };
/// ```
pub fn decode_instructions(payload: &DecodedPayload) -> (Vec<String>, Vec<DecodedInstruction>) {
    match payload {
        // Handle v0 transaction (with Address Lookup Tables)
        DecodedPayload::VersionedTransaction(tx) => {
            // Extract static account keys
            let account_keys: Vec<String> = tx
                .message
                .static_account_keys()
                .iter()
                .map(|k| k.to_string())
                .collect();

            // Decode each instruction by routing to the appropriate program decoder
            let instructions: Vec<DecodedInstruction> = tx
                .message
                .instructions()
                .iter()
                .enumerate()
                .map(|(i, ix)| {
                    // Look up the program ID from account keys using the index
                    let program_id = account_keys
                        .get(ix.program_id_index as usize)
                        .cloned()
                        .unwrap_or_default();
                    // Route to system::decode, token::decode, etc based on program_id
                    decode_instruction(i, &program_id, &ix.data, &ix.accounts, &account_keys)
                })
                .collect();

            (account_keys, instructions)
        }

        // Handle legacy transaction (pre-v0)
        DecodedPayload::Transaction(tx) => {
            // Extract account keys directly from message.account_keys
            let account_keys: Vec<String> = tx
                .message
                .account_keys
                .iter()
                .map(|k| k.to_string())
                .collect();

            // Same instruction decoding logic as v0, just different struct layout
            let instructions: Vec<DecodedInstruction> = tx
                .message
                .instructions
                .iter()
                .enumerate()
                .map(|(i, ix)| {
                    let program_id = account_keys
                        .get(ix.program_id_index as usize)
                        .cloned()
                        .unwrap_or_default();
                    decode_instruction(i, &program_id, &ix.data, &ix.accounts, &account_keys)
                })
                .collect();

            (account_keys, instructions)
        }

        // Handle unsigned message (no signatures)
        DecodedPayload::Message(msg) => {
            // Extract account keys from message
            let account_keys: Vec<String> =
                msg.account_keys.iter().map(|k| k.to_string()).collect();

            // Same instruction decoding logic as signed transactions
            let instructions: Vec<DecodedInstruction> = msg
                .instructions
                .iter()
                .enumerate()
                .map(|(i, ix)| {
                    let program_id = account_keys
                        .get(ix.program_id_index as usize)
                        .cloned()
                        .unwrap_or_default();
                    decode_instruction(i, &program_id, &ix.data, &ix.accounts, &account_keys)
                })
                .collect();

            (account_keys, instructions)
        }
    }
}

/// Validates and cleans raw input string.
///
/// Trims whitespace and enforces length limits. This prevents DoS attacks
/// via huge input strings and gives users early feedback on malformed input.
fn normalize_input(input: &str) -> Result<&str, ObsrvError> {
    let input = input.trim();

    if input.is_empty() {
        return Err(ObsrvError::EmptyInput);
    }

    if input.len() > MAX_INPUT_CHARS {
        return Err(ObsrvError::InputTooLarge);
    }
    Ok(input)
}

/// detect if the user pasted a transaction signature instead of the payload.
///
/// Users often confuse transaction signatures (the hash you see on Solscan after a tx confirms)
/// with transaction payloads (the raw bytes before signing). This catches that common mistake.
///
/// Solana signatures are 64 bytes, which encode to **87-88 characters** in base58.
/// Base58 never uses `+`, `/`, or `=` (those are base64-only characters).
///
/// # Why This Matters
/// Without this check, a signature would fail deserialization with a cryptic error like
/// "bincode failed: invalid enum variant". With this check, we give a helpful error:
/// "signature detected, not a transaction payload".
fn looks_like_signature(input: &str) -> bool {
    let len = input.len();
    let has_base64_chars = input.contains('+') || input.contains('/') || input.contains('=');
    (87..=88).contains(&len) && !has_base64_chars
}

/// Decodes a string from either base64 or base58 into raw bytes.
///
/// # Why Try Both Encodings?
///
/// Different Solana tools use different encodings:
/// - **Wallets (Phantom, Solflare)**: base58
/// - **Solana CLI (`solana transfer --dump-transaction-message`)**: base64
/// - **Some dApps**: either, depending on their library
///
/// We try base64 first because it's a superset (more strings are valid base64 than base58).
/// If base64 fails, we fall back to base58. Either way, we get the same raw bytes.
///
/// # Example
/// ```ignore
/// // Same bytes, different encoding:
/// let base64 = "AgAAAEBCDwAAAAA=";
/// let base58 = "3Bxs3zzLZLQ3";
/// // Both decode to [2, 0, 0, 0, 64, 66, 15, 0, 0, 0, 0, 0]
/// ```
fn decode_to_bytes(input: &str) -> Result<Vec<u8>, ObsrvError> {
    // Try base64 first (most CLI tools use this)
    if let Ok(bytes) = STANDARD.decode(input) {
        return Ok(bytes);
    }

    // Fall back to base58 (most wallets use this)
    if let Ok(bytes) = bs58::decode(input).into_vec() {
        return Ok(bytes);
    }

    Err(ObsrvError::DecodeFailed)
}

/// Validates that the decoded bytes are within Solana's transaction size limit.
///
/// Solana transactions are sent over UDP with a Maximum Transmission Unit (MTU)
/// of 1232 bytes. Anything larger will be rejected by validators.
///
/// This early check prevents
fn validate_byte_size(bytes: &[u8]) -> Result<(), ObsrvError> {
    if bytes.len() > MAX_TX_BYTES {
        return Err(ObsrvError::TransactionTooLarge);
    }
    Ok(())
}

/// Deserialize raw bytes into one of three Solana transaction types.
///
/// # How It Detects Transaction Type
///
/// Bincode deserialization works by matching the bytes against the struct layout.
/// Solana has three binary formats:
///
/// ## 1. VersionedTransaction (modern, v0)
/// Binary structure:
/// ```text
/// [num_signatures: u8][signatures...][version_byte: 0x80][message...]
///  ^^^^^^^^^^^^^^^^^ always 1 or more
///                                    ^^^^ KEY: 0x80 = version 0
/// ```
/// If a version byte (0x80) exists after signatures, this is a VersionedTransaction.
/// Supports Address Lookup Tables (ALTs) to reference more accounts efficiently.
///
/// ## 2. Transaction (legacy)
/// Binary structure:
/// ```text
/// [num_signatures: u8][signatures...][message_header][accounts...][instructions...]
///  ^^^^^^^^^^^^^^^^^ always 1 or more
///                                     ^^^^ NO version byte
/// ```
/// If there are signatures but NO version byte, it's a legacy Transaction.
///
/// ## 3. Message (unsigned)
/// Binary structure:
/// ```text
/// [message_header: 3 bytes][num_accounts][accounts...][blockhash][num_ix][instructions...]
///  ^^^^ starts immediately with header, NO signatures
/// ```
/// If there are NO signatures at all, it's just a Message (unsigned).
///
/// ## Why Try In This Order?
/// VersionedTransaction → Transaction → Message
///
/// Because bincode is greedy: a `Transaction` *can* deserialize from v0 bytes,
/// but it will be wrong (it'll treat the version byte as part of the message).
/// By trying the most specific type first (VersionedTransaction), we get the
/// correct interpretation.
fn deserialize_payload(bytes: &[u8]) -> Result<DecodedPayload, ObsrvError> {
    // Try versioned transaction first (checks for version byte 0x80)
    if let Ok(tx) = bincode::deserialize::<VersionedTransaction>(bytes)
        && validate_versioned_transaction(&tx).is_ok()
    {
        return Ok(DecodedPayload::VersionedTransaction(tx));
    }

    // Try full signed Transaction (legacy format with signatures)
    if let Ok(tx) = bincode::deserialize::<Transaction>(bytes)
        && validate_transaction(&tx).is_ok()
    {
        return Ok(DecodedPayload::Transaction(tx));
    }

    // Try Message (unsigned transaction, no signatures)
    if let Ok(msg) = bincode::deserialize::<Message>(bytes)
        && validate_message(&msg).is_ok()
    {
        return Ok(DecodedPayload::Message(msg));
    }

    Err(ObsrvError::DeserializationFailed(
        "not a valid Solana transaction or message. \
         paste raw transaction bytes, not a signature or wallet address"
            .to_string(),
    ))
}

/// Validates a legacy Transaction has non-empty instructions and accounts.
fn validate_transaction(tx: &Transaction) -> Result<(), ObsrvError> {
    if tx.message.instructions.is_empty() {
        return Err(ObsrvError::NoInstructions);
    }
    if tx.message.account_keys.is_empty() {
        return Err(ObsrvError::NoAccounts);
    }
    Ok(())
}

/// Validates a VersionedTransaction has non-empty instructions and accounts.
fn validate_versioned_transaction(tx: &VersionedTransaction) -> Result<(), ObsrvError> {
    let msg = tx.message.instructions();
    if msg.is_empty() {
        return Err(ObsrvError::NoInstructions);
    }
    if tx.message.static_account_keys().is_empty() {
        return Err(ObsrvError::NoAccounts);
    }
    Ok(())
}

/// Validates an unsigned Message has non-empty instructions and accounts.
fn validate_message(msg: &Message) -> Result<(), ObsrvError> {
    if msg.instructions.is_empty() {
        return Err(ObsrvError::NoInstructions);
    }
    if msg.account_keys.is_empty() {
        return Err(ObsrvError::NoAccounts);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        decoder::{DecodedPayload, decode_payload},
        errors::ObsrvError,
    };

    // TEST CONSTANTS

    const _REAL_TX1_BASE64: &str = "AQACBSRTT74scpJ7FVv/013Nh3fWggozIkHFFuH4Luuc6pIDJB3S0+z7QzDVTGAZB8IB4ApeG8Esiu9oR6bRHEEWxx9X5Hu4Uw7x57kWc20SqvabmRcZbowKzBtvi9woQtilwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABqfVFxksVo7gioRfc9KXiM8DXDFFshqzRNgGLqlAAACZxxfuaL6vLVN0nmc2sUdzfW4Q9Ccc/rGffqvUKgSFOAIDAwEEAAQEAAAAAwIAAgwCAAAAgJaYAAAAAAA=";

    const TRANSFER_BASE64: &str = "ArAOUqQ7oxWybEhgs+eaN+Jf37elw4J/3/ubiCdFtXk2Z15wfPQL2SHjzfyJInzJeJCMY4ali37C96WwvZDjSgsb4ntCNMLB5zHqpcwor+79T7oBfwA7X5Zr864JuagAhvSjamqcGPlQCh6iiWNfDSDKx1U9+T0B4AjqcpZRRxAHAgECBstBHh0vcJ08ML2PK+CrYG2eUsLjr6LV+BA3YOrUsx86w3/UdAD7yk8RZZayWHvRrDOKtHElxcfkqeo28/WIcwKOgdP1VRFeYKOveSmFstGsi1YbvU368nT75Jv3nXFOwKR5ooAjh1ZcSUC2dM3QhHhO3XUWDp+l8DKdIthOThTtAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAGp9UXGSxWjuCKhF9z0peIzwNcMUWyGrNE2AYuqUAAACcRPuaMn0wUyfJGcdCXdA1mS9yWZ05ckcFfvGD8P2HdAgQDAgUBBAQAAAAEAgADDAIAAAAAZc0dAAAAAA==";
    const _TRANSFER_BASE: &str = "AgECBgAAAAAAAAAJAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAALAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAGp9UXGSxWjuCKhF9z0peIzwNcMUWyGrNE2AYuqUAAAAEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgQDAwUBBAQAAAAEAgACDAIAAAAAZc0dAAAAAA==";

    const VERSIONED_TX: &str = "AejVvBVaznV1vUp8AmqwK5MJ6cJIua3E0MdSF2jKOc5eOaOqdk25lHPDFNXvCDaPbm8RtqfwQB573B+S/myCmQeAAQABA8fgNv53zHIGm9S3SkjHb/tCzOdIULXvo6icmtx+4u/FAAAAAAAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQICAAEMAgAAAEBCDwAAAAAAAA==";

    fn print_payload(payload: &DecodedPayload) {
        match payload {
            DecodedPayload::VersionedTransaction(tx) => {
                println!("\n=== VERSIONED TRANSACTION ===");
                println!("  version:    {:?}", tx.version());
                println!("  signatures: {}", tx.signatures.len());

                println!("\n  static accounts:");
                for (i, key) in tx.message.static_account_keys().iter().enumerate() {
                    println!("    [{}]: {}", i, key);
                }

                println!("\n  address lookup tables:");
                match tx.message.address_table_lookups() {
                    Some(alts) if !alts.is_empty() => {
                        for alt in alts {
                            println!("    table:    {}", alt.account_key);
                            println!("    writable: {:?}", alt.writable_indexes);
                            println!("    readonly: {:?}", alt.readonly_indexes);
                        }
                    }
                    _ => println!("    none"),
                }

                println!("\n  instructions:");
                for (i, ix) in tx.message.instructions().iter().enumerate() {
                    let program = tx
                        .message
                        .static_account_keys()
                        .get(ix.program_id_index as usize)
                        .map(|k| k.to_string())
                        .unwrap_or("unknown".to_string());

                    println!("    ix[{}]:", i);
                    println!("      program:       {}", program);
                    println!("      accounts:      {:?}", ix.accounts);
                    println!("      data:          {:?}", ix.data);
                    if ix.data.len() >= 4 {
                        println!("      discriminator: {:?}", &ix.data[..4]);
                    }
                }
            }

            DecodedPayload::Transaction(tx) => {
                println!("\n=== LEGACY TRANSACTION ===");
                println!("  signatures: {}", tx.signatures.len());
                println!("  accounts:   {}", tx.message.account_keys.len());

                println!("\n  accounts:");
                for (i, key) in tx.message.account_keys.iter().enumerate() {
                    println!("    [{}]: {}", i, key);
                }

                println!("\n  instructions:");
                for (i, ix) in tx.message.instructions.iter().enumerate() {
                    let program = tx
                        .message
                        .account_keys
                        .get(ix.program_id_index as usize)
                        .map(|k| k.to_string())
                        .unwrap_or("unknown".to_string());

                    println!("    ix[{}]:", i);
                    println!("      program:       {}", program);
                    println!("      accounts:      {:?}", ix.accounts);
                    println!("      data:          {:?}", ix.data);
                    if ix.data.len() >= 4 {
                        println!("      discriminator: {:?}", &ix.data[..4]);
                    }
                }
            }

            DecodedPayload::Message(msg) => {
                println!("\n=== UNSIGNED MESSAGE ===");
                println!("  accounts: {}", msg.account_keys.len());

                println!("\n  accounts:");
                for (i, key) in msg.account_keys.iter().enumerate() {
                    println!("    [{}]: {}", i, key);
                }

                println!("\n  instructions:");
                for (i, ix) in msg.instructions.iter().enumerate() {
                    let program = msg
                        .account_keys
                        .get(ix.program_id_index as usize)
                        .map(|k| k.to_string())
                        .unwrap_or("unknown".to_string());

                    println!("    ix[{}]:", i);
                    println!("      program:       {}", program);
                    println!("      accounts:      {:?}", ix.accounts);
                    println!("      data:          {:?}", ix.data);
                    if ix.data.len() >= 4 {
                        println!("      discriminator: {:?}", &ix.data[..4]);
                    }
                }
            }
        }
    }

    // TESTS
    #[test]
    fn test_inspect_payload() {
        let result = decode_payload(
            "AukAKPbB8rPK4qZ7EGKy2M2a+GnUyX4wry2UT5EXSIpFDGBr+rwRD1HC8kH7/nJb6oZO0cJF7Ezs5URlUU4WuA/37jOm75tp07RoLiuLc08vwCiUn+Tv8m+xPfZYWyG6LfSEqVORnDUJmyf53HXDXTwiRDUdt8cSye6BdUUg8lYIAgADBWiNcZcYDnkp5OTTxY0B9GmSnTN5CLKJ46weqKGhBtvZ7Lj/onXRjJjSnW9XMScUZdras+qnZWqZ2ocMESEuUjcAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAan1RcZLFaO4IqEX3PSl4jPA1wxRbIas0TYBi6pQAAABqfVFxksXFEhjMlMPUrxf1ja7gibof1E49vZigAAAABKHN45hIcZJ4znfSoWaId/bEZIO7mv14GfnPQTbGQvhAICAgABNAAAAAAAFxYAAAAAAFAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAwEDBCQGAAAAqlb0h9splNSOkTL/Ph7XFAZS0zouGeS08qepaqISsLw=",
        );
        assert!(result.is_ok());
        print_payload(&result.unwrap());
    }

    #[test]
    fn test_transfer_message_inspect() {
        let result = decode_payload(TRANSFER_BASE64);
        assert!(result.is_ok(), "got: {:?}", result.err());
        print_payload(&result.unwrap());
    }

    #[test]
    fn test_inspect_versioned_payload() {
        let result = decode_payload(VERSIONED_TX);
        assert!(result.is_ok(), "got: {:?}", result.err());
        print_payload(&result.unwrap());
    }

    #[test]
    fn test_empty_input_returns_error() {
        let result = decode_payload("");
        assert!(result.is_err());
        match result.unwrap_err() {
            ObsrvError::EmptyInput => {}
            e => panic!("expected EmptyInput, got: {:#?}", e),
        }
    }

    #[test]
    fn test_large_input_rejected() {
        let huge = "A".repeat(20_000);
        let result = decode_payload(&huge);
        assert!(matches!(result.unwrap_err(), ObsrvError::InputTooLarge));
    }

    #[test]
    fn test_invalid_input_fails_cleanly() {
        let result = decode_payload("notavalidtx");
        assert!(result.is_err());
        println!("error: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_signature_rejected_with_helpful_error() {
        let sig = "4nMHq9vSWGHGGmSMRFbHqRMBJCzBbkFMmkzJMsXu7tXnKuQUuWFEumEHhQXEzd1XBe8W54LBwW2RiX9DVWXrDvp";
        let result = decode_payload(sig);
        assert!(result.is_err());
        println!("sig error: {:?}", result.unwrap_err());
    }
}
