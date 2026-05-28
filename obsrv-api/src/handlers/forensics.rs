use std::str::FromStr;

use axum::{Json, extract::State};
use obsrv_core::{
    analyzer::analyze,
    types::{ApiResponse, ForensicsRequest, SimulationFee, TxForensics, TxResponse},
};
use solana_client::rpc_config::{CommitmentConfig, RpcTransactionConfig};
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    UiInstruction, UiParsedInstruction, UiTransactionEncoding, option_serializer::OptionSerializer,
};

use crate::{
    errors::ApiError,
    handlers::simulate::convert_ui_token_balances,
    response::{build_analysis, build_balances, build_instructions, build_meta},
    state::AppState,
};

pub async fn handle(
    State(state): State<AppState>,
    Json(req): Json<ForensicsRequest>,
) -> Result<Json<ApiResponse>, ApiError> {
    tracing::info!(signature = %req.signature, "POST /forensics");

    let sig = Signature::from_str(&req.signature)
        .map_err(|e| ApiError::BadRequest(format!("invalid signature: {}", e)))?;

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Base64),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let tx_response = state
        .helius
        .connection()
        .get_transaction_with_config(&sig, config)
        .map_err(|e| ApiError::InternalError(format!("RPC error: {}", e)))?;

    let tx_with_meta = tx_response.transaction;

    let meta = tx_with_meta
        .meta
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("transaction has no metadata".to_string()))?;

    let decoded_tx = tx_with_meta
        .transaction
        .decode()
        .ok_or_else(|| ApiError::InternalError("could not decode transaction".to_string()))?;

    let payload = obsrv_core::decoder::DecodedPayload::VersionedTransaction(decoded_tx);
    let report = analyze(&payload).map_err(ApiError::from)?;

    let execution_status = if meta.err.is_none() {
        "success".to_string()
    } else {
        "failed".to_string()
    };

    let failure_reason = meta.clone().err.as_ref().map(|e| format!("{:?}", e));
    let cu_consumed = meta.clone().compute_units_consumed.map(|c| c as u64);
    let fee_lamports = meta.fee;
    let fee_sol = fee_lamports as f64 / solana_sdk::native_token::LAMPORTS_PER_SOL as f64;

    let mut logs = meta
        .log_messages
        .clone()
        .map(|v| v.to_vec())
        .unwrap_or_default();
    logs.extend(build_inner_instruction_logs(meta, &report.account_keys));

    let pre_ui_tokens = match &meta.pre_token_balances {
        OptionSerializer::Some(v) => v.clone(),
        _ => vec![],
    };

    let post_ui_tokens = match &meta.post_token_balances {
        OptionSerializer::Some(v) => v.clone(),
        _ => vec![],
    };

    let pre_tokens = convert_ui_token_balances(&pre_ui_tokens, &report.account_keys);
    let post_tokens = convert_ui_token_balances(&post_ui_tokens, &report.account_keys);

    let forensics = TxForensics {
        execution_status,
        failure_reason,
        cu_consumed,
        fee: SimulationFee {
            lamports: fee_lamports,
            sol: fee_sol,
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
                meta.pre_balances.as_slice(),
                meta.post_balances.as_slice(),
                &pre_tokens,
                &post_tokens,
            ),
            simulation: None,
            forensics: Some(forensics),
        },
    }))
}

fn build_inner_instruction_logs(
    meta: &solana_transaction_status::UiTransactionStatusMeta,
    account_keys: &[String],
) -> Vec<String> {
    meta.inner_instructions
        .clone()
        .map(|v| v.to_vec())
        .unwrap_or_default()
        .into_iter()
        .flat_map(|group| {
            group.instructions.into_iter().map({
                let account_keys = account_keys.to_vec();
                move |ix| match ix {
                    UiInstruction::Compiled(compiled) => {
                        let program = account_keys
                            .get(compiled.program_id_index as usize)
                            .cloned()
                            .unwrap_or_else(|| "unknown".to_string());

                        let accounts = compiled
                            .accounts
                            .iter()
                            .filter_map(|i| account_keys.get(*i as usize).cloned())
                            .collect::<Vec<_>>();

                        format!(
                            "[inner ix {}] program: {} accounts: {:?} data: {}",
                            group.index, program, accounts, compiled.data
                        )
                    }
                    UiInstruction::Parsed(parsed) => match parsed {
                        UiParsedInstruction::Parsed(p) => {
                            format!(
                                "[inner ix {}] parsed_program: {} parsed: {:?}",
                                group.index, p.program_id, p.parsed
                            )
                        }
                        UiParsedInstruction::PartiallyDecoded(p) => {
                            format!(
                                "[inner ix {}] program: {} accounts: {:?} data: {}",
                                group.index, p.program_id, p.accounts, p.data
                            )
                        }
                    },
                }
            })
        })
        .collect()
}
