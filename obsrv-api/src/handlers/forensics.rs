use std::str::FromStr;

use axum::{Json, extract::State};
use obsrv_core::{
    analyzer::analyze,
    decoder::decode_payload,
    types::{ApiResponse, ForensicsRequest, SimulationFee, TokenBalance, TxForensics, TxResponse},
};
use serde_json::Value;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{CommitmentConfig, RpcTransactionConfig},
};
use solana_sdk::signature::Signature;
use solana_transaction_status::UiTransactionEncoding;

use crate::{
    errors::ApiError,
    response::{build_analysis, build_balances, build_instructions, build_meta},
    state::AppState,
};

pub async fn handle(
    State(state): State<AppState>,
    Json(req): Json<ForensicsRequest>,
) -> Result<Json<ApiResponse>, ApiError> {
    tracing::info!(signature = %req.signature, "POST /forensics");

    let result = fetch_transaction(&state.config.rpc_url, &req.signature).await?;
    let raw_tx = result
        .get("transaction")
        .and_then(|tx| tx.get(0))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ApiError::InternalError("RPC response missing base64 transaction".to_string())
        })?;

    let meta = result
        .get("meta")
        .filter(|v| !v.is_null())
        .ok_or_else(|| ApiError::InternalError("transaction has no metadata".to_string()))?;

    let payload = decode_payload(raw_tx).map_err(ApiError::from)?;
    let report = analyze(&payload).map_err(ApiError::from)?;

    let err = meta.get("err").filter(|v| !v.is_null());
    let execution_status = if err.is_none() {
        "success".to_string()
    } else {
        "failed".to_string()
    };

    let fee_lamports = meta.get("fee").and_then(Value::as_u64).unwrap_or(0);
    let mut logs = meta
        .get("logMessages")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    logs.extend(build_inner_instruction_logs(meta, &report.account_keys));

    let pre_sol = read_u64_array(meta.get("preBalances"));
    let post_sol = read_u64_array(meta.get("postBalances"));
    let pre_tokens = read_token_balances(meta.get("preTokenBalances"), &report.account_keys);
    let post_tokens = read_token_balances(meta.get("postTokenBalances"), &report.account_keys);

    let forensics = TxForensics {
        execution_status,
        failure_reason: err.map(ToString::to_string),
        cu_consumed: meta.get("computeUnitsConsumed").and_then(Value::as_u64),
        fee: SimulationFee {
            lamports: fee_lamports,
            sol: fee_lamports as f64 / solana_sdk::native_token::LAMPORTS_PER_SOL as f64,
        },
        logs,
    };

    Ok(Json(ApiResponse {
        tx: TxResponse {
            meta: build_meta(&report),
            analysis: build_analysis(&report),
            instructions: build_instructions(&report),
            balances: build_balances(
                &report.account_keys,
                &pre_sol,
                &post_sol,
                &pre_tokens,
                &post_tokens,
            ),
            simulation: None,
            forensics: Some(forensics),
        },
    }))
}

async fn fetch_transaction(rpc_url: &str, signature: &str) -> Result<Value, ApiError> {
    let tx_sig = Signature::from_str(signature)
        .map_err(|e| ApiError::BadRequest(format!("invalid signature: {}", e)))?;

    // Try finalized first, then fall back to confirmed — some RPC nodes haven't
    // propagated finality yet even for older transactions.
    let commitments = [CommitmentConfig::finalized(), CommitmentConfig::confirmed()];

    for commitment in commitments {
        let client = RpcClient::new_with_commitment(rpc_url.to_string(), commitment);

        let config = RpcTransactionConfig {
            commitment: Some(commitment),
            encoding: Some(UiTransactionEncoding::Base64),
            max_supported_transaction_version: Some(0),
        };

        match client.get_transaction_with_config(&tx_sig, config) {
            Ok(transaction) => {
                return serde_json::to_value(transaction)
                    .map_err(|e| ApiError::InternalError(format!("serialization failed: {}", e)));
            }
            Err(e) => {
                let msg = e.to_string();
                // Serde null error = RPC returned null = tx not found at this commitment.
                if msg.contains("invalid type: null") || msg.contains("not found") {
                    tracing::debug!(
                        commitment = ?commitment,
                        "transaction not found at this commitment level, trying next"
                    );
                    continue;
                }
                // Any other RPC error is a real failure — surface it immediately.
                return Err(ApiError::InternalError(format!(
                    "getTransaction failed: {}",
                    e
                )));
            }
        }
    }

    Err(ApiError::BadRequest(format!(
        "transaction not found: {} (not finalized or confirmed on this RPC node)",
        signature
    )))
}

fn read_u64_array(value: Option<&Value>) -> Vec<u64> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_u64).collect())
        .unwrap_or_default()
}

fn read_token_balances(value: Option<&Value>, account_keys: &[String]) -> Vec<TokenBalance> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|tb| {
                    let account_index = tb
                        .get("accountIndex")
                        .and_then(Value::as_u64)
                        .and_then(|i| u8::try_from(i).ok())?;
                    let ui_amount = tb.get("uiTokenAmount")?;

                    Some(TokenBalance {
                        account_index: Some(account_index),
                        address: account_keys.get(account_index as usize)?.clone(),
                        mint: tb.get("mint")?.as_str()?.to_string(),
                        amount: ui_amount.get("amount")?.as_str()?.to_string(),
                        decimals: ui_amount
                            .get("decimals")
                            .and_then(Value::as_u64)
                            .and_then(|d| u8::try_from(d).ok())
                            .unwrap_or(0),
                        ui_amount: ui_amount
                            .get("uiAmountString")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        owner: tb
                            .get("owner")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        program_id: tb
                            .get("programId")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_inner_instruction_logs(meta: &Value, account_keys: &[String]) -> Vec<String> {
    meta.get("innerInstructions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .flat_map(|group| {
            let index = group.get("index").and_then(Value::as_u64).unwrap_or(0);
            group
                .get("instructions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map({
                    let account_keys = account_keys.to_vec();
                    move |ix| {
                        if let Some(program_id_index) =
                            ix.get("programIdIndex").and_then(Value::as_u64)
                        {
                            let program = account_keys
                                .get(program_id_index as usize)
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_string());

                            let accounts = ix
                                .get("accounts")
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_u64)
                                .filter_map(|i| account_keys.get(i as usize).cloned())
                                .collect::<Vec<_>>();

                            format!(
                                "[inner ix {}] program: {} accounts: {:?} data: {}",
                                index,
                                program,
                                accounts,
                                ix.get("data").and_then(Value::as_str).unwrap_or_default()
                            )
                        } else {
                            let program = ix
                                .get("program")
                                .or_else(|| ix.get("programId"))
                                .and_then(Value::as_str)
                                .unwrap_or("unknown");
                            let parsed = ix
                                .get("parsed")
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "{}".to_string());
                            format!(
                                "[inner ix {}] parsed_program: {} parsed: {}",
                                index, program, parsed
                            )
                        }
                    }
                })
        })
        .collect()
}
