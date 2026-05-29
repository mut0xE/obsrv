use crate::{
    errors::ApiError,
    response::{build_analysis, build_balances, build_instructions, build_meta, extract_cu_budget},
    state::AppState,
};
use axum::{Json, extract::State};
use obsrv_core::{
    account_decoder::DecodedAccountState,
    analyzer::analyze,
    decoder::decode_payload,
    types::{
        ApiResponse, InstructionType, ReplacementBlockhash, SimulateRequest, SimulationCompute,
        SimulationFee, TokenBalance, TxResponse, TxSimulation,
    },
};
use solana_client::rpc_config::{
    CommitmentConfig, RpcSimulateTransactionAccountsConfig, RpcSimulateTransactionConfig,
};
use solana_client::rpc_response::UiAccountEncoding;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{UiTransactionTokenBalance, option_serializer::OptionSerializer};
use std::str::FromStr;

pub async fn handle(
    State(state): State<AppState>,
    Json(req): Json<SimulateRequest>,
) -> Result<Json<ApiResponse>, ApiError> {
    tracing::info!(input_len = req.raw_tx.len(), "POST /simulate");

    let payload = decode_payload(&req.raw_tx).map_err(ApiError::from)?;
    let report = analyze(&payload).map_err(ApiError::from)?;

    let versioned_tx = extract_versioned_tx(&payload).ok_or_else(|| {
        ApiError::BadRequest(
            "simulation requires a signed transaction, not an unsigned message".to_string(),
        )
    })?;

    let rpc = state.helius.connection();

    let pubkeys: Vec<Pubkey> = report
        .account_keys
        .iter()
        .filter_map(|k| Pubkey::from_str(k).ok())
        .collect();

    let fetched_accounts = rpc
        .get_multiple_accounts(&pubkeys)
        .unwrap_or_else(|_| vec![None; pubkeys.len()]);

    let fallback_pre_sol = fetched_accounts
        .iter()
        .map(|opt| opt.as_ref().map(|a| a.lamports).unwrap_or(0))
        .collect::<Vec<u64>>();

    let decoded_accounts = fetched_accounts
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| {
            let account = opt.as_ref()?;
            let pubkey = pubkeys.get(i)?;

            match obsrv_core::account_decoder::decode_account_state(
                pubkey,
                &account.owner,
                &account.data,
            ) {
                Some(decoded) => {
                    println!("{:#?}", decoded);
                    Some(decoded)
                }

                None => Some(DecodedAccountState {
                    address: pubkey.to_string(),
                    owner: account.owner.to_string(),

                    // fallback label
                    program: match account.owner.to_string().as_str() {
                        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" => "token-2022",
                        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "spl-token",
                        "11111111111111111111111111111111" => "system",
                        _ => "unknown",
                    }
                    .to_string(),

                    parsed: serde_json::json!({
                        "note": "account parser unavailable",
                        "data_len": account.data.len(),
                    }),

                    space: account.data.len() as u64,
                }),
            }
        })
        .collect::<Vec<_>>();

    for account in &decoded_accounts {
        tracing::debug!(
            address = %account.address,
            owner = %account.owner,
            program = %account.program,
            space = account.space,
            "decoded simulation account"
        );
    }
    let config = RpcSimulateTransactionConfig {
        sig_verify: false,
        replace_recent_blockhash: true,
        commitment: Some(CommitmentConfig::confirmed()),
        encoding: None,
        accounts: Some(RpcSimulateTransactionAccountsConfig {
            encoding: Some(UiAccountEncoding::Base64),
            addresses: report.account_keys.clone(),
        }),
        min_context_slot: None,
        inner_instructions: false,
    };

    let sim_result = rpc
        .simulate_transaction_with_config(&versioned_tx, config)
        .map_err(|e| ApiError::InternalError(format!("simulation RPC failed: {}", e)))?;

    let sim_value = sim_result.value;

    let pre_sol = sim_value
        .pre_balances
        .clone()
        .unwrap_or_else(|| fallback_pre_sol.clone());

    let post_sol = sim_value
        .post_balances
        .clone()
        .unwrap_or_else(|| pre_sol.clone());

    let pre_tokens = convert_ui_token_balances(
        &sim_value.pre_token_balances.clone().unwrap_or_default(),
        &report.account_keys,
    );

    let post_tokens = convert_ui_token_balances(
        &sim_value.post_token_balances.clone().unwrap_or_default(),
        &report.account_keys,
    );

    let cu_budget = extract_cu_budget(&report);
    let cu_consumed = sim_value.units_consumed;
    let cu_usage_pct = match (cu_consumed, cu_budget) {
        (Some(consumed), Some(budget)) if budget > 0 => {
            Some(consumed as f64 / budget as f64 * 100.0)
        }
        _ => None,
    };

    let success = sim_value.err.is_none();
    let error = sim_value.err.as_ref().map(|e| format!("{:?}", e));

    let fee_lamports = sim_value.fee.unwrap_or_else(|| {
        let fee_payer_pre = pre_sol.first().copied().unwrap_or(0);
        let fee_payer_post = post_sol.first().copied().unwrap_or(0);

        if fee_payer_pre > fee_payer_post {
            let transfer_out: u64 = report
                .instructions
                .iter()
                .filter(|ix| matches!(ix.instruction_type, InstructionType::Transfer))
                .filter_map(|ix| {
                    let from = ix.details.get("from")?;
                    if from == &report.fee_payer {
                        ix.details.get("lamports")?.parse::<u64>().ok()
                    } else {
                        None
                    }
                })
                .sum();

            fee_payer_pre
                .saturating_sub(fee_payer_post)
                .saturating_sub(transfer_out)
        } else {
            5000
        }
    });

    let fee_sol = fee_lamports as f64 / solana_sdk::native_token::LAMPORTS_PER_SOL as f64;
    let logs = sim_value.logs.unwrap_or_default();

    let cu_budget = extract_cu_budget(&report).or_else(|| extract_cu_budget_from_logs(&logs));
    let cu_consumed = sim_value.units_consumed;

    let replacement_blockhash =
        sim_value
            .replacement_blockhash
            .as_ref()
            .map(|b| ReplacementBlockhash {
                blockhash: b.blockhash.clone(),
                last_valid_block_height: b.last_valid_block_height,
            });

    let simulation = TxSimulation {
        success,
        error,
        fee: SimulationFee {
            lamports: fee_lamports,
            sol: fee_sol,
        },
        compute: SimulationCompute {
            consumed: cu_consumed,
            budget: cu_budget,
            usage_pct: cu_usage_pct,
        },
        logs,
        replacement_blockhash,
        accounts: decoded_accounts,
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
            simulation: Some(simulation),
            forensics: None,
        },
    }))
}

fn extract_versioned_tx(
    payload: &obsrv_core::decoder::DecodedPayload,
) -> Option<solana_sdk::transaction::VersionedTransaction> {
    use obsrv_core::decoder::DecodedPayload;

    match payload {
        DecodedPayload::VersionedTransaction(tx) => Some(tx.clone()),
        DecodedPayload::Transaction(tx) => Some(
            solana_sdk::transaction::VersionedTransaction::from(tx.clone()),
        ),
        DecodedPayload::Message(_) => None,
    }
}

pub fn convert_ui_token_balances(
    balances: &[UiTransactionTokenBalance],
    account_keys: &[String],
) -> Vec<TokenBalance> {
    balances
        .iter()
        .filter_map(|tb| {
            Some(TokenBalance {
                account_index: Some(tb.account_index),
                address: account_keys.get(tb.account_index as usize)?.clone(),
                mint: tb.mint.clone(),
                amount: tb.ui_token_amount.amount.clone(),
                decimals: tb.ui_token_amount.decimals,
                ui_amount: Some(tb.ui_token_amount.ui_amount_string.clone()),
                owner: match &tb.owner {
                    OptionSerializer::Some(owner) => Some(owner.clone()),
                    _ => None,
                },
                program_id: match &tb.program_id {
                    OptionSerializer::Some(program_id) => Some(program_id.clone()),
                    _ => None,
                },
            })
        })
        .collect()
}

fn extract_cu_budget_from_logs(logs: &[String]) -> Option<u64> {
    logs.iter().find_map(|log| {
        let (_, after_of) = log.split_once(" of ")?;
        let (budget, _) = after_of.split_once(" compute units")?;
        budget.parse::<u64>().ok()
    })
}
