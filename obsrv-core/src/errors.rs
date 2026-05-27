use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObsrvError {
    // input validation
    #[error("input is empty")]
    EmptyInput,

    #[error("input too large")]
    InputTooLarge,

    #[error("transaction too large")]
    TransactionTooLarge,

    // decoding
    #[error("base64 decode failed: {0}")]
    Base64DecodeFailed(String),

    #[error("base58 decode failed: {0}")]
    Base58DecodeFailed(String),

    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("decode failed: input is neither valid base64 nor base58")]
    DecodeFailed,

    // validation
    #[error("transaction has no instructions")]
    NoInstructions,

    #[error("transaction has no accounts")]
    NoAccounts,

    // analysis
    #[error("analysis failed: {0}")]
    AnalysisFailed(String),

    // RPC (for forensics and monitor endpoints)
    #[error("RPC request failed: {0}")]
    RpcFailed(String),

    #[error("account not found: {0}")]
    AccountNotFound(String),

    #[error("invalid nonce account data")]
    InvalidNonceAccount,
}
