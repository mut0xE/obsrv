use std::str::FromStr;

use axum::{Json, extract::State};
use obsrv_core::{
    analyzer::analyze,
    types::{AccountDiff, ForensicsRequest, ForensicsResponse},
};
use solana_client::rpc_config::{CommitmentConfig, RpcTransactionConfig};
use solana_sdk::signature::Signature;
use solana_transaction_status::{UiInstruction, UiParsedInstruction, UiTransactionEncoding};

use crate::{errors::ApiError, state::AppState};

pub async fn handle(
    State(state): State<AppState>,
    Json(req): Json<ForensicsRequest>,
) -> Result<Json<ForensicsResponse>, ApiError> {
    tracing::info!(signature = %req.signature, "POST /forensics");

    //parse signature
    let sig = Signature::from_str(&req.signature)
        .map_err(|e| ApiError::BadRequest(format!("invalid signature: {}", e)))?;

    // fetch transaction from chain via Helius
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Base64),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0), // versioned tx
    };

    let tx_response = state
        .helius
        .connection()
        .get_transaction_with_config(&sig, config)
        .map_err(|e| ApiError::InternalError(format!("RPC error: {}", e)))?;

    tracing::debug!("tx_response: {:#?}", tx_response);

    let tx_with_meta = tx_response.transaction;

    tracing::debug!("tx_with_meta: {:#?}", tx_with_meta);
    // extract meta

    let meta = tx_with_meta
        .meta
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("transaction has no metadata".to_string()))?;
    tracing::info!("meta: {:#?}", meta);

    let decoded_tx = tx_with_meta
        .transaction
        .decode()
        .ok_or_else(|| ApiError::InternalError("could not decode transaction".to_string()))?;

    let payload = obsrv_core::decoder::DecodedPayload::VersionedTransaction(decoded_tx);

    let report = analyze(&payload).map_err(ApiError::from)?;

    // execution status
    let execution_status = if meta.err.is_none() {
        "success".to_string()
    } else {
        "failed".to_string()
    };

    let failure_reason = meta.err.as_ref().map(|e| format!("{:?}", e));

    let cu_consumed = meta.compute_units_consumed.clone().map(|c| c as u64);

    let fee_lamports = meta.fee;

    let inner_logs: Vec<String> = meta
        .inner_instructions
        .clone()
        .map(|v| v.to_vec())
        .unwrap_or_default()
        .into_iter()
        .flat_map(|group| {
            group.instructions.into_iter().map({
                let account_keys = report.account_keys.clone();

                move |ix| match ix {
                    UiInstruction::Compiled(compiled) => {
                        let program = account_keys
                            .get(compiled.program_id_index as usize)
                            .cloned()
                            .unwrap_or_else(|| "unknown".to_string());

                        let accounts: Vec<String> = compiled
                            .accounts
                            .iter()
                            .filter_map(|i| account_keys.get(*i as usize).cloned())
                            .collect();

                        format!(
                            "[inner ix {}]\n  program: {}\n  accounts: {:?}\n  data: {}",
                            group.index, program, accounts, compiled.data
                        )
                    }

                    UiInstruction::Parsed(parsed) => match parsed {
                        UiParsedInstruction::Parsed(p) => {
                            format!(
                                "[inner ix {}]\n  parsed_program: {}\n  parsed: {:?}",
                                group.index, p.program_id, p.parsed
                            )
                        }

                        UiParsedInstruction::PartiallyDecoded(p) => {
                            format!(
                                "[inner ix {}]\n  program: {}\n  accounts: {:?}\n  data: {}",
                                group.index, p.program_id, p.accounts, p.data
                            )
                        }
                    },
                }
            })
        })
        .collect();

    let mut logs = meta
        .log_messages
        .clone()
        .map(|v| v.to_vec())
        .unwrap_or_default();

    logs.extend(inner_logs);
    // build account diffs
    let account_diffs = build_account_diffs(
        &report.account_keys,
        meta.pre_balances.as_slice(),
        meta.post_balances.as_slice(),
    );

    tracing::info!(
        signature       = %req.signature,
        status          = %execution_status,
        risk_score      = report.risk_score,
        cu              = ?cu_consumed,
        "forensics complete"
    );

    Ok(Json(ForensicsResponse {
        report,
        execution_status,
        failure_reason,
        cu_consumed,
        fee_lamports,
        logs,
        account_diffs,
    }))
}

fn build_account_diffs(
    account_keys: &[String],
    pre_balances: &[u64],
    post_balances: &[u64],
) -> Vec<AccountDiff> {
    account_keys
        .iter()
        .enumerate()
        .map(|(i, address)| {
            let before = pre_balances.get(i).copied().unwrap_or(0);
            let after = post_balances.get(i).copied().unwrap_or(0);
            let change = after as i64 - before as i64;

            AccountDiff {
                address: address.clone(),
                before_lamports: before,
                after_lamports: after,
                change_lamports: change,
                before_tokens: None,
                after_tokens: None,
            }
        })
        .collect()
}
