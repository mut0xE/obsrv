use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// severity level for each instruction

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    None,
    Info,
    Warning,
    Critical,
}

// instruction type as enum
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum InstructionType {
    // nonce instructions
    NonceAdvance,
    NonceInitialize,
    NonceWithdraw,
    NonceAuthorize,

    // system program
    CreateAccount,
    Transfer,

    // token program
    TokenTransfer,
    TokenTransferChecked,
    TokenCloseAccount,
    TokenSetAuthority,
    TokenApprove,
    // compute budget
    SetComputeUnitLimit,
    SetComputeUnitPrice,

    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProgramType {
    System,
    SplToken,
    Token2022,
    ComputeBudget,
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecodedInstruction {
    // position in transaction (0 = first)
    pub index: usize,

    // which program: "System Program", "SPL Token"
    pub program: ProgramType,

    // what type of instruction
    pub instruction_type: InstructionType,

    // example: {"from": "ABC...", "amount": "100 SOL"}
    pub details: HashMap<String, String>,

    // true ONLY if this is nonceAdvance
    pub is_nonce_advance: bool,

    // risk warnings for this instruction
    pub risk_flags: Vec<String>,

    // severity for frontend color coding
    pub severity: Severity,
}

// full analysis report returned to API
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionReport {
    // true if instruction[0] is nonceAdvance
    pub is_durable_nonce: bool,

    // nonce account address if durable nonce found
    pub nonce_account: Option<String>,

    // who controls the nonce account
    pub nonce_authority: Option<String>,

    // all instructions decoded in order
    pub instructions: Vec<DecodedInstruction>,

    // risk score 1-10
    pub risk_score: u8,

    // low, medium, high, critical
    pub risk_level: Severity,

    // SIGN, REVIEW CAREFULLY, DO NOT SIGN
    pub recommendation: String,

    // plain English summary of what this tx does
    pub summary: String,

    // all accounts involved
    pub account_keys: Vec<String>,

    // who pays the fee (first account)
    pub fee_payer: String,
}

// REQUEST STRUCTS

/// user pastes raw transaction bytes
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeTxRequest {
    pub raw_tx: String,
}

/// user pastes a confirmed transaction signature from Explorer
#[derive(Debug, Serialize, Deserialize)]
pub struct ForensicsRequest {
    pub signature: String,
}

/// user adds a wallet address to monitor
#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorWalletRequest {
    pub wallet: String,
    pub telegram_chat_id: String,
}

/// user pastes a nonce account address
#[derive(Debug, Serialize, Deserialize)]
pub struct NonceInspectRequest {
    pub nonce_account: String,
}

// RESPONSE STRUCTS

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeResponse {
    pub report: TransactionReport,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForensicsResponse {
    pub report: TransactionReport,
    /// "success" or "failed"
    pub execution_status: String,
    /// before/after changes for each account
    pub account_diffs: Vec<AccountDiff>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountDiff {
    pub address: String,
    /// SOL balance before in lamports
    pub before_lamports: u64,
    /// SOL balance after in lamports
    pub after_lamports: u64,
    /// token balance before if token account
    pub before_tokens: Option<String>,
    /// token balance after if token account
    pub after_tokens: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorResponse {
    pub watching: bool,
    pub wallet: String,
    pub webhook_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceInspectResponse {
    pub nonce_account: String,
    pub authority: String,
    pub nonce_value: String,
    pub created_slot: Option<u64>,
    pub risk_flags: Vec<String>,
}
