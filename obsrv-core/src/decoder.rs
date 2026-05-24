// Takes raw input string (base64 or base58)
// Returns deserialized Transaction or error message

use base64::{Engine, engine::general_purpose::STANDARD};
use solana_sdk::transaction::Transaction;

use crate::errors::ObsrvError;

pub fn decode_transaction(input: &str) -> Result<Transaction, ObsrvError> {
    let input = input.trim();

    if input.is_empty() {
        return Err(ObsrvError::EmptyInput);
    }
    let bytes = decode_to_bytes(input)?;
    deserialize_transaction(&bytes)
}

fn deserialize_transaction(bytes: &[u8]) -> Result<Transaction, ObsrvError> {
    let tx = bincode::deserialize::<Transaction>(bytes)
        .map_err(|e| ObsrvError::DeserializationFailed(e.to_string()));
    println!("tx:{:#?}", tx);
    tx
}

fn decode_to_bytes(input: &str) -> Result<Vec<u8>, ObsrvError> {
    let is_base_64 = input.contains('+') || input.contains('/') || input.contains("=");
    if is_base_64 {
        decode_base64(input)
    } else {
        decode_base58(input)
    }
}

fn decode_base58(input: &str) -> Result<Vec<u8>, ObsrvError> {
    let bytes = bs58::decode(input)
        .into_vec()
        .map_err(|e| ObsrvError::Base58DecodeFailed(e.to_string()));
    println!("base58 {:?}", bytes);
    bytes
}

fn decode_base64(input: &str) -> Result<Vec<u8>, ObsrvError> {
    let bytes = STANDARD
        .decode(input)
        .map_err(|e| ObsrvError::Base64DecodeFailed(e.to_string()));
    println!("base64 {:?}", bytes);
    bytes
}

#[cfg(test)]
mod tests {
    use crate::{decoder::decode_transaction, errors::ObsrvError};

    const REAL_TX_BASE64: &str = "";

    const REAL_TX2_BASE64: &str = "";

    #[test]
    fn test_empty_input_returns_error() {
        let result = decode_transaction("");
        assert!(result.is_err());

        match result.unwrap_err() {
            ObsrvError::EmptyInput => {}
            e => panic!("expected EmptyInput,got: {:#?}", e),
        }
    }
}
