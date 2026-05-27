use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObsrvError {
    // input errors
    #[error("empty input provided")]
    EmptyInput,

    #[error("input too large (max 10000 chars)")]
    InputTooLarge,

    #[error("transaction too large (max 1232 bytes)")]
    TransactionTooLarge,

    // transaction errors
    #[error("transaction has no instructions")]
    NoInstructions,

    #[error("transaction has no accounts")]
    NoAccounts,

    #[error("invalid account index: {0}")]
    InvalidAccountIndex(usize),

    // decode errors
    #[error("base64 decode failed: {0}")]
    Base64DecodeFailed(String),

    #[error("base58 decode failed: {0}")]
    Base58DecodeFailed(String),

    #[error("decode failed: input is neither valid base64 nor base58")]
    DecodeFailed,

    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("wrong input type: {0}")]
    WrongInputType(String),

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
