// ============================================================
// streams/processor.rs — transaction processing pipeline
//
// Receives a single transaction from the Yellowstone gRPC stream
// and runs the full obsrv analytics pipeline:
//
//   1. Convert protobuf Transaction → solana_sdk VersionedTransaction
//   2. Run obsrv-core analyze() → TransactionReport (risk score, etc.)
//   3. Extract balance changes from protobuf TransactionStatusMeta
//   4. Determine which watched wallets/programs matched this tx
//   5. Build StreamTxEvent and broadcast to WebSocket
//   6. Store forensics_history for EACH matched wallet
//   7. Update wallet/program/instruction aggregate tables
//   8. Fire alerts (WebSocket + Telegram) when risk >= threshold
//   9. Update stream_checkpoint with current slot
// ============================================================

use chrono::Utc;
use obsrv_core::{
    analyzer::analyze,
    decoder::DecodedPayload,
    types::{StreamSolChange, StreamTokenChange, StreamTxEvent, TxStatus, WalletAlert},
};
use solana_sdk::{
    hash::Hash,
    message::{
        self, MessageHeader, VersionedMessage, compiled_instruction::CompiledInstruction, v0,
    },
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use sqlx::PgPool;
use std::collections::HashSet;
use tokio::sync::broadcast;
use yellowstone_grpc_proto::solana::storage::confirmed_block as proto;

use crate::{
    db::queries,
    response::{ix_type_str, program_id_str, program_str, risk_level_str},
    streams::telegram,
    ws::WsEvent,
};

// ============================================================
// MAIN ENTRY POINT
// ============================================================

/// Process a single transaction received from the Yellowstone gRPC stream.
pub async fn process_transaction(
    meta: &proto::TransactionStatusMeta,
    tx_proto: &proto::Transaction,
    signature_str: &str,
    slot: u64,
    pool: &PgPool,
    ws_tx: &broadcast::Sender<WsEvent>,
    telegram_bot_token: &Option<String>,
) {
    let now = Utc::now().timestamp();

    // ── 1. Convert protobuf → VersionedTransaction ──────────

    let versioned_tx = match convert_proto_transaction(tx_proto) {
        Some(tx) => tx,
        None => {
            tracing::warn!(signature = %signature_str, "failed to convert protobuf tx");
            return;
        }
    };

    // ── 2. Run obsrv-core analyze() ─────────────────────────

    let payload = DecodedPayload::VersionedTransaction(versioned_tx);
    let report = match analyze(&payload) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(signature = %signature_str, error = %e, "analyze failed");
            return;
        }
    };

    let fee_payer = &report.fee_payer;
    let risk = report.risk_score;
    let is_durable_nonce = report.is_durable_nonce;
    let failed = meta.err.is_some();
    let cu_consumed = meta.compute_units_consumed;
    let fee = meta.fee;
    let slot_i64 = slot as i64;

    // ── 3. Extract balance changes from protobuf meta ───────

    let sol_changes = extract_sol_changes(
        &report.account_keys,
        &meta.pre_balances,
        &meta.post_balances,
    );
    let token_changes = extract_token_changes(&meta.pre_token_balances, &meta.post_token_balances);

    // ── 4. Collect unique programs called ────────────────────

    let programs: Vec<String> = {
        let mut seen = HashSet::new();
        report
            .instructions
            .iter()
            .map(|ix| program_str(&ix.program))
            .filter(|p| seen.insert(p.clone()))
            .collect()
    };

    let program_ids: Vec<String> = {
        let mut seen = HashSet::new();
        report
            .instructions
            .iter()
            .map(|ix| program_id_str(&ix.program))
            .filter(|p| seen.insert(p.clone()))
            .collect()
    };

    let log_program_ids = extract_invoked_programs_from_logs(&meta.log_messages);
    let all_program_ids: HashSet<String> = program_ids
        .iter()
        .cloned()
        .chain(log_program_ids.iter().cloned())
        .collect();

    // ── 5. Determine which watched wallets/programs matched ──

    // Stream processor needs the global "watched by anyone" list — a single
    // tx might match different users' rows. Per-user fan-out happens later
    // when each subscriber reads the broadcast.
    let watched_wallets = queries::get_all_watched_wallets(pool)
        .await
        .unwrap_or_default();
    let watched_programs = queries::get_all_watched_programs(pool)
        .await
        .unwrap_or_default();

    // Match wallets that appear ANYWHERE in the tx account keys
    let matched_wallets: Vec<String> = watched_wallets
        .iter()
        .filter(|w| report.account_keys.contains(&w.wallet))
        .map(|w| w.wallet.clone())
        .collect();

    let matched_programs: Vec<String> = watched_programs
        .iter()
        .filter(|p| all_program_ids.contains(&p.program_id))
        .map(|p| p.program_id.clone())
        .collect();

    // Skip if no watched wallet or program matched
    if matched_wallets.is_empty() && matched_programs.is_empty() {
        return;
    }

    // ── 6. Build StreamTxEvent ───────────────────────────────

    let _execution_status = if failed { "failed" } else { "success" };

    let tx_event = StreamTxEvent {
        signature: signature_str.to_string(),
        slot,
        block_time: None,
        fee_payer: fee_payer.clone(),
        status: if failed {
            TxStatus::Failed
        } else {
            TxStatus::Success
        },
        failure_reason: meta.err.as_ref().map(|e| format!("{:?}", e)),
        fee_lamports: fee,
        cu_consumed,
        matched_wallets: matched_wallets.clone(),
        matched_programs: matched_programs.clone(),
        risk_score: risk,
        risk_level: risk_level_str(&report.risk_level),
        summary: report.summary.clone(),
        flags: report.risk_flags.clone(),
        sol_changes: sol_changes.clone(),
        token_changes: token_changes.clone(),
        programs_called: programs.clone(),
        logs: meta.log_messages.clone(),
    };

    // ── 7. Broadcast to WebSocket ────────────────────────────

    let _ = ws_tx.send(WsEvent::TxProcessed(tx_event));

    // ── 8. Store forensics_history for EACH matched wallet ───
    //    This ensures /stream/transactions?wallet=X returns txs
    //    where that wallet appears anywhere, not just as fee_payer.

    // for wallet in &matched_wallets {
    //     let _ = queries::insert_forensics_history(
    //         pool,
    //         signature_str,
    //         slot_i64,
    //         wallet,
    //         risk as i32,
    //         execution_status,
    //         meta.err.as_ref().map(|e| format!("{:?}", e)).as_deref(),
    //         cu_consumed.map(|c| c as i64),
    //         fee as i64,
    //         is_durable_nonce,
    //         serde_json::to_value(&programs).ok(),
    //         None,
    //     )
    //     .await;
    // }

    // // If no matched wallet but matched programs, store under fee_payer
    // if matched_wallets.is_empty() && !matched_programs.is_empty() {
    //     let _ = queries::insert_forensics_history(
    //         pool,
    //         signature_str,
    //         slot_i64,
    //         fee_payer,
    //         risk as i32,
    //         execution_status,
    //         meta.err.as_ref().map(|e| format!("{:?}", e)).as_deref(),
    //         cu_consumed.map(|c| c as i64),
    //         fee as i64,
    //         is_durable_nonce,
    //         serde_json::to_value(&programs).ok(),
    //         None,
    //     )
    //     .await;
    // }

    // let should_store = risk >= 7 || is_durable_nonce || failed;

    // if should_store {
    //     for wallet in &matched_wallets {
    //         let _ = queries::insert_forensics_history(
    //             pool,
    //             signature_str,
    //             slot_i64,
    //             wallet,
    //             risk as i32,
    //             execution_status,
    //             meta.err.as_ref().map(|e| format!("{:?}", e)).as_deref(),
    //             cu_consumed.map(|c| c as i64),
    //             fee as i64,
    //             is_durable_nonce,
    //             serde_json::to_value(&programs).ok(),
    //             None,
    //         )
    //         .await;
    //     }

    //     if matched_wallets.is_empty() && !matched_programs.is_empty() {
    //         let _ = queries::insert_forensics_history(
    //             pool,
    //             signature_str,
    //             slot_i64,
    //             fee_payer,
    //             risk as i32,
    //             execution_status,
    //             meta.err.as_ref().map(|e| format!("{:?}", e)).as_deref(),
    //             cu_consumed.map(|c| c as i64),
    //             fee as i64,
    //             is_durable_nonce,
    //             serde_json::to_value(&programs).ok(),
    //             None,
    //         )
    //         .await;
    //     }
    // }

    // ── 9. Update wallet analytics for each matched wallet ───

    for watched in &watched_wallets {
        if !report.account_keys.contains(&watched.wallet) {
            continue;
        }

        let cu_i64 = cu_consumed.unwrap_or(0) as i64;
        let cu_f64 = cu_i64 as f64;
        let failed_i64: i64 = if failed { 1 } else { 0 };
        let high_risk_i64: i64 = if risk >= 7 { 1 } else { 0 };
        let risk_key = risk.to_string();

        let _ = sqlx::query!(
            r#"
            INSERT INTO wallet_analytics
                (wallet, total_txs, failed_txs, total_cu, avg_cu,
                 total_fees, high_risk_txs, last_updated, risk_distribution)
            VALUES ($1, 1, $2, $3, $4, $5, $6, $7, jsonb_build_object($8::text, 1))
            ON CONFLICT(wallet) DO UPDATE SET
                total_txs     = wallet_analytics.total_txs + 1,
                failed_txs    = wallet_analytics.failed_txs + $2,
                total_cu      = wallet_analytics.total_cu + $3,
                avg_cu        = (wallet_analytics.total_cu + $3)::float8
                                / (wallet_analytics.total_txs + 1),
                total_fees    = wallet_analytics.total_fees + $5,
                high_risk_txs = wallet_analytics.high_risk_txs + $6,
                last_updated  = $7,
                risk_distribution = jsonb_set(
                    COALESCE(wallet_analytics.risk_distribution, '{}'::jsonb),
                    ARRAY[$8::text],
                    to_jsonb(COALESCE((wallet_analytics.risk_distribution->>$8)::bigint, 0) + 1)
                )
            "#,
            &watched.wallet,
            failed_i64,
            cu_i64,
            cu_f64,
            fee as i64,
            high_risk_i64,
            now,
            risk_key,
        )
        .execute(pool)
        .await;

        for ix in &report.instructions {
            let prog = program_id_str(&ix.program);
            let ixt = ix_type_str(&ix.instruction_type);

            let _ = sqlx::query!(
                r#"
                INSERT INTO wallet_program_stats
                    (wallet, program_id, call_count, failed_count,
                     total_cu, avg_cu, last_called)
                VALUES ($1, $2, 1, $3, $4, $5, $6)
                ON CONFLICT(wallet, program_id) DO UPDATE SET
                    call_count   = wallet_program_stats.call_count + 1,
                    failed_count = wallet_program_stats.failed_count + $3,
                    total_cu     = wallet_program_stats.total_cu + $4,
                    avg_cu       = (wallet_program_stats.total_cu + $4)::float8
                                   / (wallet_program_stats.call_count + 1),
                    last_called  = $6
                "#,
                &watched.wallet,
                prog,
                failed_i64,
                cu_i64,
                cu_f64,
                now
            )
            .execute(pool)
            .await;

            let _ = sqlx::query!(
                r#"
                INSERT INTO wallet_ix_stats
                    (wallet, program_id, instruction_type,
                     call_count, failed_count, total_cu, avg_cu, last_called)
                VALUES ($1, $2, $3, 1, $4, $5, $6, $7)
                ON CONFLICT(wallet, program_id, instruction_type) DO UPDATE SET
                    call_count   = wallet_ix_stats.call_count + 1,
                    failed_count = wallet_ix_stats.failed_count + $4,
                    total_cu     = wallet_ix_stats.total_cu + $5,
                    avg_cu       = (wallet_ix_stats.total_cu + $5)::float8
                                   / (wallet_ix_stats.call_count + 1),
                    last_called  = $7
                "#,
                &watched.wallet,
                prog,
                ixt,
                failed_i64,
                cu_i64,
                cu_f64,
                now
            )
            .execute(pool)
            .await;
        }

        // ── Alert if risk >= threshold ───────────────────────
        if risk >= watched.alert_threshold as u8 {
            let alert = WalletAlert {
                wallet: watched.wallet.clone(),
                telegram_chat_id: watched.telegram_chat_id.clone().unwrap_or_default(),
                signature: signature_str.to_string(),
                slot,
                risk_score: risk,
                risk_level: risk_level_str(&report.risk_level),
                summary: report.summary.clone(),
                flags: report.risk_flags.clone(),
                fee_lamports: fee,
                cu_consumed,
                sol_changes: sol_changes.clone(),
                token_changes: token_changes.clone(),
            };

            let _ = ws_tx.send(WsEvent::Alert(alert.clone()));

            // Store alert in DB
            let _ = sqlx::query!(
                r#"
                INSERT INTO alerts_history
                    (wallet, signature, slot, risk_score, summary,
                     programs, is_durable_nonce, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
                &watched.wallet,
                signature_str,
                slot_i64,
                risk as i32,
                &report.summary,
                serde_json::to_value(&programs).unwrap_or_default(),
                is_durable_nonce,
                now,
            )
            .execute(pool)
            .await;

            // Send Telegram alert
            if let Some(token) = telegram_bot_token {
                if let Some(ref chat_id) = watched.telegram_chat_id {
                    if !chat_id.is_empty() {
                        telegram::send_alert(token, &alert).await;
                    }
                }
            }

            tracing::warn!(
                wallet    = %watched.wallet,
                risk      = risk,
                threshold = watched.alert_threshold,
                signature = %signature_str,
                "ALERT: high-risk tx detected"
            );
        }
    }

    // ── 10. Update program analytics for matched programs ────

    let mut analytics_events: Vec<(String, String)> = report
        .instructions
        .iter()
        .map(|ix| {
            (
                program_id_str(&ix.program),
                ix_type_str(&ix.instruction_type),
            )
        })
        .collect();

    let instruction_programs: HashSet<String> = analytics_events
        .iter()
        .map(|(pid, _)| pid.clone())
        .collect();

    for pid in &matched_programs {
        if !instruction_programs.contains(pid) {
            analytics_events.push((pid.clone(), "cpi_invoke".to_string()));
        }
    }

    let mut seen_events = HashSet::new();
    for (prog, ixt) in analytics_events {
        if !matched_programs.contains(&prog) || !seen_events.insert((prog.clone(), ixt.clone())) {
            continue;
        }

        let cu_i64 = cu_consumed.unwrap_or(0) as i64;
        let cu_f64 = cu_i64 as f64;
        let failed_i64: i64 = if failed { 1 } else { 0 };
        let risk_key = risk.to_string();

        let is_new: bool = sqlx::query_scalar!(
            r#"SELECT NOT EXISTS(
                SELECT 1 FROM wallet_program_stats
                WHERE wallet = $1 AND program_id = $2
            ) as "is_new!""#,
            fee_payer,
            prog
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        let nw: i64 = if is_new { 1 } else { 0 };

        let _ = sqlx::query!(
            r#"
            INSERT INTO program_analytics
                (program_id, total_calls, failed_calls, total_cu,
                 avg_cu, peak_cu, unique_wallets, first_seen, last_seen,
                 risk_distribution)
            VALUES ($1, 1, $2, $3, $4, $3, $5, $6, $6,
                    jsonb_build_object($7::text, 1))
            ON CONFLICT(program_id) DO UPDATE SET
                total_calls    = program_analytics.total_calls + 1,
                failed_calls   = program_analytics.failed_calls + $2,
                total_cu       = program_analytics.total_cu + $3,
                avg_cu         = (program_analytics.total_cu + $3)::float8
                                 / (program_analytics.total_calls + 1),
                peak_cu        = GREATEST(program_analytics.peak_cu, $3),
                unique_wallets = program_analytics.unique_wallets + $5,
                last_seen      = $6,
                risk_distribution = jsonb_set(
                    COALESCE(program_analytics.risk_distribution, '{}'::jsonb),
                    ARRAY[$7::text],
                    to_jsonb(COALESCE((program_analytics.risk_distribution->>$7)::bigint, 0) + 1)
                )
            "#,
            prog,
            failed_i64,
            cu_i64,
            cu_f64,
            nw,
            now,
            risk_key
        )
        .execute(pool)
        .await;

        let f_i32 = if failed { 1_i32 } else { 0_i32 };
        let cu_i32 = cu_i64 as i32;
        let cu_f32 = cu_i64 as f32;
        let now_i32 = now as i32;

        let _ = sqlx::query!(
            r#"
            INSERT INTO instruction_analytics
                (program_id, instruction_type,
                 call_count, failed_count, total_cu, avg_cu,
                 peak_cu, unique_callers, first_called, last_called,
                 risk_distribution)
            VALUES ($1, $2, 1, $3, $4, $5, $4, 1, $6, $6,
                    jsonb_build_object($7::text, 1))
            ON CONFLICT(program_id, instruction_type) DO UPDATE SET
                call_count   = instruction_analytics.call_count + 1,
                failed_count = instruction_analytics.failed_count + $3,
                total_cu     = instruction_analytics.total_cu + $4,
                avg_cu       = (instruction_analytics.total_cu + $4)::real
                               / (instruction_analytics.call_count + 1),
                peak_cu      = GREATEST(instruction_analytics.peak_cu, $4),
                last_called  = $6,
                risk_distribution = jsonb_set(
                    COALESCE(instruction_analytics.risk_distribution, '{}'::jsonb),
                    ARRAY[$7::text],
                    to_jsonb(COALESCE((instruction_analytics.risk_distribution->>$7)::bigint, 0) + 1)
                )
            "#,
            prog,
            ixt,
            f_i32,
            cu_i32,
            cu_f32,
            now_i32,
            risk_key
        )
        .execute(pool)
        .await;

        let date = Utc::now().format("%Y-%m-%d").to_string();
        let _ = sqlx::query!(
            r#"
            INSERT INTO instruction_daily
                (program_id, instruction_type, date,
                 call_count, failed_count, total_cu, avg_cu, peak_cu)
            VALUES ($1, $2, $3, 1, $4, $5, $6, $5)
            ON CONFLICT(program_id, instruction_type, date) DO UPDATE SET
                call_count   = instruction_daily.call_count + 1,
                failed_count = instruction_daily.failed_count + $4,
                total_cu     = instruction_daily.total_cu + $5,
                avg_cu       = (instruction_daily.total_cu + $5)::float8
                               / (instruction_daily.call_count + 1),
                peak_cu      = GREATEST(instruction_daily.peak_cu, $5)
            "#,
            prog,
            ixt,
            date,
            failed_i64,
            cu_i64,
            cu_f64
        )
        .execute(pool)
        .await;
    }

    // ── 11. Update stream checkpoint ─────────────────────────

    let _ = queries::update_stream_checkpoint(pool, slot_i64).await;

    tracing::info!(
        signature = %signature_str,
        fee_payer = %fee_payer,
        risk      = risk,
        cu        = ?cu_consumed,
        failed    = failed,
        slot      = slot,
        matched_wallets  = ?matched_wallets,
        matched_programs = ?matched_programs,
        "tx processed"
    );
}

// ============================================================
// HELPER: extract invoked program IDs from log messages
// ============================================================

fn extract_invoked_programs_from_logs(logs: &[String]) -> HashSet<String> {
    logs.iter()
        .filter_map(|log| {
            let rest = log.strip_prefix("Program ")?;
            let (program_id, action) = rest.split_once(' ')?;
            action.starts_with("invoke").then(|| program_id.to_string())
        })
        .collect()
}

// ============================================================
// BALANCE CHANGE EXTRACTION FROM PROTOBUF META
// ============================================================

fn extract_sol_changes(
    account_keys: &[String],
    pre_balances: &[u64],
    post_balances: &[u64],
) -> Vec<StreamSolChange> {
    account_keys
        .iter()
        .enumerate()
        .filter_map(|(i, address)| {
            let pre = pre_balances.get(i).copied().unwrap_or(0);
            let post = post_balances.get(i).copied().unwrap_or(pre);
            let diff = post as i64 - pre as i64;
            if diff == 0 {
                return None;
            }
            Some(StreamSolChange {
                address: address.clone(),
                pre_lamports: pre,
                post_lamports: post,
                change_lamports: diff,
                pre_sol: pre as f64 / solana_sdk::native_token::LAMPORTS_PER_SOL as f64,
                post_sol: post as f64 / solana_sdk::native_token::LAMPORTS_PER_SOL as f64,
                change_sol: diff as f64 / solana_sdk::native_token::LAMPORTS_PER_SOL as f64,
            })
        })
        .collect()
}

fn extract_token_changes(
    pre_tokens: &[proto::TokenBalance],
    post_tokens: &[proto::TokenBalance],
) -> Vec<StreamTokenChange> {
    let mut changes = Vec::new();

    let post_map: std::collections::HashMap<(u32, &str), &proto::TokenBalance> = post_tokens
        .iter()
        .map(|tb| ((tb.account_index, tb.mint.as_str()), tb))
        .collect();

    let mut seen = std::collections::HashSet::new();
    for pre in pre_tokens {
        let key = (pre.account_index, pre.mint.as_str());
        seen.insert((pre.account_index, pre.mint.clone()));

        let pre_amount = pre
            .ui_token_amount
            .as_ref()
            .map(|a| a.amount.clone())
            .unwrap_or_else(|| "0".to_string());
        let pre_i: i128 = pre_amount.parse().unwrap_or(0);

        let (post_amount, decimals) = if let Some(post) = post_map.get(&key) {
            let amt = post
                .ui_token_amount
                .as_ref()
                .map(|a| a.amount.clone())
                .unwrap_or_else(|| "0".to_string());
            let dec = post
                .ui_token_amount
                .as_ref()
                .map(|a| a.decimals)
                .unwrap_or(0);
            (amt, dec)
        } else {
            (
                "0".to_string(),
                pre.ui_token_amount
                    .as_ref()
                    .map(|a| a.decimals)
                    .unwrap_or(0),
            )
        };

        let post_i: i128 = post_amount.parse().unwrap_or(0);
        let diff = post_i - pre_i;

        if diff == 0 {
            continue;
        }

        changes.push(StreamTokenChange {
            address: format!("account_{}", pre.account_index),
            owner: if pre.owner.is_empty() {
                None
            } else {
                Some(pre.owner.clone())
            },
            mint: pre.mint.clone(),
            pre_raw: pre_amount.clone(),
            post_raw: post_amount.clone(),
            change_raw: diff.to_string(),
            pre_ui: format_token_ui(pre_i, decimals),
            post_ui: format_token_ui(post_i, decimals),
            change_ui: format_token_ui(diff, decimals),
            decimals: decimals as u8,
        });
    }

    // post-only entries (new token accounts created in this tx)
    for post in post_tokens {
        if seen.contains(&(post.account_index, post.mint.clone())) {
            continue;
        }

        let post_amount = post
            .ui_token_amount
            .as_ref()
            .map(|a| a.amount.clone())
            .unwrap_or_else(|| "0".to_string());
        let post_i: i128 = post_amount.parse().unwrap_or(0);
        let decimals = post
            .ui_token_amount
            .as_ref()
            .map(|a| a.decimals)
            .unwrap_or(0);

        if post_i == 0 {
            continue;
        }

        changes.push(StreamTokenChange {
            address: format!("account_{}", post.account_index),
            owner: if post.owner.is_empty() {
                None
            } else {
                Some(post.owner.clone())
            },
            mint: post.mint.clone(),
            pre_raw: "0".to_string(),
            post_raw: post_amount.clone(),
            change_raw: post_i.to_string(),
            pre_ui: "0".to_string(),
            post_ui: format_token_ui(post_i, decimals),
            change_ui: format_token_ui(post_i, decimals),
            decimals: decimals as u8,
        });
    }

    changes
}

fn format_token_ui(value: i128, decimals: u32) -> String {
    if decimals == 0 {
        return value.to_string();
    }
    let divisor = 10_i128.pow(decimals);
    let abs = value.unsigned_abs();
    let whole = abs / divisor as u128;
    let frac = abs % divisor as u128;
    let sign = if value < 0 { "-" } else { "" };

    if frac == 0 {
        format!("{}{}", sign, whole)
    } else {
        let frac_str = format!("{:0width$}", frac, width = decimals as usize);
        format!("{}{}.{}", sign, whole, frac_str.trim_end_matches('0'))
    }
}

// ============================================================
// PROTOBUF → SOLANA SDK CONVERSION
// ============================================================

fn convert_proto_transaction(proto_tx: &proto::Transaction) -> Option<VersionedTransaction> {
    let msg = proto_tx.message.as_ref()?;

    let signatures: Vec<Signature> = proto_tx
        .signatures
        .iter()
        .filter_map(|s| {
            let bytes: [u8; 64] = s.as_slice().try_into().ok()?;
            Some(Signature::from(bytes))
        })
        .collect();

    if signatures.is_empty() {
        return None;
    }

    let account_keys: Vec<Pubkey> = msg
        .account_keys
        .iter()
        .filter_map(|k| {
            let bytes: [u8; 32] = k.as_slice().try_into().ok()?;
            Some(Pubkey::from(bytes))
        })
        .collect();

    let recent_blockhash_bytes: [u8; 32] = msg.recent_blockhash.as_slice().try_into().ok()?;
    let recent_blockhash = Hash::from(recent_blockhash_bytes);

    let header = msg.header.as_ref()?;
    let message_header = MessageHeader {
        num_required_signatures: header.num_required_signatures as u8,
        num_readonly_signed_accounts: header.num_readonly_signed_accounts as u8,
        num_readonly_unsigned_accounts: header.num_readonly_unsigned_accounts as u8,
    };

    let instructions: Vec<CompiledInstruction> = msg
        .instructions
        .iter()
        .map(|ix| CompiledInstruction {
            program_id_index: ix.program_id_index as u8,
            accounts: ix.accounts.clone(),
            data: ix.data.clone(),
        })
        .collect();

    let message = if msg.versioned {
        let address_table_lookups: Vec<solana_sdk::message::v0::MessageAddressTableLookup> = msg
            .address_table_lookups
            .iter()
            .filter_map(|atl| {
                let key_bytes: [u8; 32] = atl.account_key.as_slice().try_into().ok()?;
                Some(solana_sdk::message::v0::MessageAddressTableLookup {
                    account_key: Pubkey::from(key_bytes),
                    writable_indexes: atl.writable_indexes.clone(),
                    readonly_indexes: atl.readonly_indexes.clone(),
                })
            })
            .collect();

        VersionedMessage::V0(v0::Message {
            header: message_header,
            account_keys,
            recent_blockhash,
            instructions,
            address_table_lookups,
        })
    } else {
        VersionedMessage::Legacy(message::legacy::Message {
            header: message_header,
            account_keys,
            recent_blockhash,
            instructions,
        })
    };

    Some(VersionedTransaction {
        signatures,
        message,
    })
}
