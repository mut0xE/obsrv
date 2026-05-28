use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    None,
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum InstructionType {
    NonceAdvance,
    NonceInitialize,
    NonceWithdraw,
    NonceAuthorize,
    CreateAccount,
    Transfer,
    TokenTransfer,
    TokenTransferChecked,
    TokenCloseAccount,
    TokenSetAuthority,
    TokenApprove,
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
    pub index: usize,
    pub program: ProgramType,
    pub instruction_type: InstructionType,
    pub details: HashMap<String, String>,
    pub is_nonce_advance: bool,
    pub risk_flags: Vec<String>,
    pub severity: Severity,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionReport {
    pub is_durable_nonce: bool,
    pub nonce_account: Option<String>,
    pub nonce_authority: Option<String>,
    pub instructions: Vec<DecodedInstruction>,
    pub risk_score: u8,
    pub risk_level: Severity,
    pub recommendation: String,
    pub risk_flags: Vec<String>,
    pub summary: String,
    pub account_keys: Vec<String>,
    pub fee_payer: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeTxRequest {
    pub raw_tx: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForensicsRequest {
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulateRequest {
    pub raw_tx: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub tx: TxResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxResponse {
    pub meta: TxMeta,
    pub analysis: TxAnalysis,
    pub instructions: Vec<TxInstruction>,
    pub balances: TxBalances,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation: Option<TxSimulation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forensics: Option<TxForensics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxMeta {
    pub fee_payer: String,
    pub is_durable_nonce: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<NonceDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceDetail {
    pub account: String,
    pub authority: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxAnalysis {
    pub risk_score: u8,
    pub risk_level: String,
    pub recommendation: String,
    pub summary: String,
    pub flags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxInstruction {
    pub index: usize,
    pub program: String,
    #[serde(rename = "type")]
    pub ix_type: String,
    pub severity: String,
    pub accounts: InstructionAccounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<InstructionToken>,
    pub flags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstructionAccounts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstructionToken {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint: Option<String>,
    pub amount_raw: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_ui: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxBalances {
    pub changes: Vec<BalanceChange>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceChange {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sol: Option<SolChange>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tokens: Vec<TokenChange>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SolChange {
    pub pre_lamports: u64,
    pub post_lamports: u64,
    pub change_lamports: i64,
    pub pre_sol: f64,
    pub post_sol: f64,
    pub change_sol: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenChange {
    pub account_index: Option<u8>,
    pub mint: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub program_id: Option<String>,

    pub pre_raw: String,
    pub post_raw: String,
    pub change_raw: String,

    pub pre_ui: String,
    pub post_ui: String,
    pub change_ui: String,

    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub account_index: Option<u8>,
    pub address: String,
    pub mint: String,
    pub amount: String,
    pub decimals: u8,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_amount: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub program_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxSimulation {
    pub success: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    pub fee: SimulationFee,
    pub compute: SimulationCompute,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_blockhash: Option<ReplacementBlockhash>,

    pub logs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReplacementBlockhash {
    pub blockhash: String,
    pub last_valid_block_height: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationFee {
    pub lamports: u64,
    pub sol: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationCompute {
    pub consumed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_pct: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxForensics {
    pub execution_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cu_consumed: Option<u64>,
    pub fee: SimulationFee,
    pub logs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorWalletRequest {
    pub wallet: String,
    pub telegram_chat_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceInspectRequest {
    pub nonce_account: String,
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
