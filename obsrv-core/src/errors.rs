use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObsrvError {
    // input decoding errors
    #[error("base64 decode failed: {0}")]
    Base64DecodeFailed(String),

    #[error("base58 decode failed: {0}")]
    Base58DecodeFailed(String),

    #[error("empty input provided")]
    EmptyInput,

    // transaction errors
    #[error("transaction deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("transaction has no instructions")]
    NoInstructions,

    #[error("invalid account index: {0}")]
    InvalidAccountIndex(usize),

    // nonce errors
    #[error("nonce account not found: {0}")]
    NonceAccountNotFound(String),

    #[error("failed to fetch nonce account: {0}")]
    NonceAccountFetchFailed(String),

    // rpc errors
    #[error("rpc call failed: {0}")]
    RpcFailed(String),

    #[error("simulation failed: {0}")]
    SimulationFailed(String),
}
