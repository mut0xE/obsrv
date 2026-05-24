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

    // compute budget
    SetComputeUnitLimit,
    SetComputeUnitPrice,

    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProgramType {
    System,
    SplToken,
    ComputeBudget,
    Unknown,
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
