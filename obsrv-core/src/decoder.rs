// Takes raw input string (base64 or base58)
// Returns deserialized Transaction or error message

use base64::{Engine, engine::general_purpose::STANDARD};
use solana_sdk::{message::Message, transaction::Transaction};

use crate::errors::ObsrvError;

const MAX_INPUT_CHARS: usize = 10_000;
const MAX_TX_BYTES: usize = 1232;

pub enum InputType {
    RawTransaction,
    TxSignature,
    WalletAddress,
    Unknown,
}

#[derive(Debug)]
pub enum DecodedPayload {
    Transaction(Transaction),
    Message(Message),
}

pub fn decode_payload(input: &str) -> Result<DecodedPayload, ObsrvError> {
    // clean and validate input string
    let input = normalize_input(input)?;

    // detect encoding and convert to bytes
    let bytes = decode_to_bytes(input)?;

    // check byte size
    validate_byte_size(&bytes)?;

    // deserialize bytes into Transaction or Message
    let result = deserialize_payload(&bytes);
    println!("deserialize_payload:{:#?}", result);
    result
}

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

fn looks_like_base64(input: &str) -> bool {
    input.chars().any(|c| matches!(c, '+' | '/' | '='))
}

fn decode_to_bytes(input: &str) -> Result<Vec<u8>, ObsrvError> {
    if looks_like_base64(input) {
        decode_base64(input)
    } else {
        decode_base58(input)
    }
}

fn decode_base58(input: &str) -> Result<Vec<u8>, ObsrvError> {
    bs58::decode(input)
        .into_vec()
        .map_err(|e| ObsrvError::Base58DecodeFailed(e.to_string()))
}

fn decode_base64(input: &str) -> Result<Vec<u8>, ObsrvError> {
    STANDARD
        .decode(input)
        .map_err(|e| ObsrvError::Base64DecodeFailed(e.to_string()))
}

fn validate_byte_size(bytes: &[u8]) -> Result<(), ObsrvError> {
    if bytes.len() > MAX_TX_BYTES {
        return Err(ObsrvError::TransactionTooLarge);
    }
    Ok(())
}

fn deserialize_payload(bytes: &[u8]) -> Result<DecodedPayload, ObsrvError> {
    // try full signed Transaction
    if let Ok(tx) = bincode::deserialize::<Transaction>(bytes) {
        validate_transaction(&tx)?;
        return Ok(DecodedPayload::Transaction(tx));
    }

    // try unsigned Message
    if let Ok(msg) = bincode::deserialize::<Message>(bytes) {
        validate_message(&msg)?;
        return Ok(DecodedPayload::Message(msg));
    }

    Err(ObsrvError::DeserializationFailed(
        "not a valid Solana transaction or message. \
         paste raw transaction bytes not a signature \
         or wallet address"
            .to_string(),
    ))
}

fn validate_transaction(tx: &Transaction) -> Result<(), ObsrvError> {
    if tx.message.instructions.is_empty() {
        return Err(ObsrvError::NoInstructions);
    }
    if tx.message.account_keys.is_empty() {
        return Err(ObsrvError::NoAccounts);
    }
    Ok(())
}

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

    const REAL_TX1_BASE64: &str = "AQACBSRTT74scpJ7FVv/013Nh3fWggozIkHFFuH4Luuc6pIDJB3S0+z7QzDVTGAZB8IB4ApeG8Esiu9oR6bRHEEWxx9X5Hu4Uw7x57kWc20SqvabmRcZbowKzBtvi9woQtilwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABqfVFxksVo7gioRfc9KXiM8DXDFFshqzRNgGLqlAAACZxxfuaL6vLVN0nmc2sUdzfW4Q9Ccc/rGffqvUKgSFOAIDAwEEAAQEAAAAAwIAAgwCAAAAgJaYAAAAAAA=";

    #[test]
    fn test_inspect_payload() {
        let result = decode_payload(REAL_TX1_BASE64);

        assert!(result.is_ok());

        match result.unwrap() {
            DecodedPayload::Transaction(tx) => {
                println!("{:#?}", tx);

                println!("instructions: {}", tx.message.instructions.len());

                println!("accounts: {}", tx.message.account_keys.len());
            }

            DecodedPayload::Message(msg) => {
                println!("{:#?}", msg);
            }
        }
    }

    #[test]
    fn test_empty_input_returns_error() {
        let result = decode_payload("");
        assert!(result.is_err());

        match result.unwrap_err() {
            ObsrvError::EmptyInput => {}
            e => panic!("expected EmptyInput,got: {:#?}", e),
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
    fn test_real_tx1_decodes() {
        let result = decode_payload(REAL_TX1_BASE64);
        assert!(result.is_ok(), "got: {:?}", result.err());

        match result.unwrap() {
            DecodedPayload::Transaction(tx) => {
                println!(
                    "TX1: {} instructions, {} accounts",
                    tx.message.instructions.len(),
                    tx.message.account_keys.len()
                );
            }
            DecodedPayload::Message(_) => {
                // also acceptable
                println!("TX1 decoded as Message");
            }
        }
    }
}
