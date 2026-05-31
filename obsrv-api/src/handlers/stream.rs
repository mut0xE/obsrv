use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{errors::ApiError, state::AppState};

// ── query params ──

#[derive(Debug, Deserialize)]
pub struct TxHistoryParams {
    pub wallet: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct WalletParam {
    pub wallet: String,
}

#[derive(Debug, Deserialize)]
pub struct TxParam {
    pub signature: String,
}

// ── response types ──

#[derive(Debug, Serialize)]
pub struct StreamTxRow {
    pub id: i64,
    pub signature: String,
    pub slot: i64,
    pub wallet: String,
    pub risk_score: i32,
    pub execution_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    pub cu_consumed: Option<i64>,
    pub fee_lamports: i64,
    pub is_durable_nonce: bool,
    pub programs_called: Option<JsonValue>,
    pub block_time: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct TxHistoryResponse {
    pub transactions: Vec<StreamTxRow>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct StreamStatsResponse {
    pub total_transactions: i64,
    pub watched_wallets: i64,
    pub watched_programs: i64,
    pub latest_slot: Option<i64>,
}

// ── internal DB row ──

#[derive(Debug, sqlx::FromRow)]
struct ForensicsRow {
    id: i64,
    signature: String,
    slot: i64,
    wallet: String,
    risk_score: i32,
    execution_status: String,
    failure_reason: Option<String>,
    cu_consumed: Option<i64>,
    fee_lamports: i64,
    is_durable_nonce: bool,
    programs_called: Option<JsonValue>,
    block_time: Option<i64>,
    created_at: i64,
}

impl From<ForensicsRow> for StreamTxRow {
    fn from(r: ForensicsRow) -> Self {
        StreamTxRow {
            id: r.id,
            signature: r.signature,
            slot: r.slot,
            wallet: r.wallet,
            risk_score: r.risk_score,
            execution_status: r.execution_status,
            failure_reason: r.failure_reason,
            cu_consumed: r.cu_consumed,
            fee_lamports: r.fee_lamports,
            is_durable_nonce: r.is_durable_nonce,
            programs_called: r.programs_called,
            block_time: r.block_time,
            created_at: r.created_at,
        }
    }
}

// ── GET /stream/transactions ──

pub async fn get_transactions(
    State(state): State<AppState>,
    Query(params): Query<TxHistoryParams>,
) -> Result<Json<TxHistoryResponse>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    let (rows, total) = if let Some(ref wallet) = params.wallet {
        let rows = sqlx::query_as!(
            ForensicsRow,
            r#"
            SELECT id, signature, slot, wallet, risk_score, execution_status,
                   failure_reason, cu_consumed, fee_lamports, is_durable_nonce,
                   programs_called, block_time, created_at
            FROM forensics_history
            WHERE wallet = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            wallet,
            limit,
            offset,
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM forensics_history WHERE wallet = $1"#,
            wallet,
        )
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

        (rows, total)
    } else {
        let rows = sqlx::query_as!(
            ForensicsRow,
            r#"
            SELECT id, signature, slot, wallet, risk_score, execution_status,
                   failure_reason, cu_consumed, fee_lamports, is_durable_nonce,
                   programs_called, block_time, created_at
            FROM forensics_history
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset,
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

        let total = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM forensics_history"#)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

        (rows, total)
    };

    let transactions = rows.into_iter().map(|r| r.into()).collect();
    Ok(Json(TxHistoryResponse {
        transactions,
        total,
    }))
}

// ── GET /stream/tx?signature=... ──

pub async fn get_transaction(
    State(state): State<AppState>,
    Query(params): Query<TxParam>,
) -> Result<Json<Option<StreamTxRow>>, ApiError> {
    let row = sqlx::query_as!(
        ForensicsRow,
        r#"
        SELECT id, signature, slot, wallet, risk_score, execution_status,
               failure_reason, cu_consumed, fee_lamports, is_durable_nonce,
               programs_called, block_time, created_at
        FROM forensics_history
        WHERE signature = $1
        "#,
        params.signature,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    Ok(Json(row.map(|r| r.into())))
}

// ── GET /stream/stats ──

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<StreamStatsResponse>, ApiError> {
    let total_transactions =
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM forensics_history"#)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let watched_wallets = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM watched_wallets WHERE active = TRUE"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let watched_programs = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM watched_programs WHERE active = TRUE"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let latest_slot =
        sqlx::query_scalar!(r#"SELECT last_slot FROM stream_checkpoint ORDER BY id DESC LIMIT 1"#)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    Ok(Json(StreamStatsResponse {
        total_transactions,
        watched_wallets,
        watched_programs,
        latest_slot,
    }))
}

// ── GET /stream/wallet/history ──

pub async fn get_wallet_history(
    State(state): State<AppState>,
    Query(params): Query<WalletParam>,
) -> Result<Json<TxHistoryResponse>, ApiError> {
    let rows = sqlx::query_as!(
        ForensicsRow,
        r#"
        SELECT id, signature, slot, wallet, risk_score, execution_status,
               failure_reason, cu_consumed, fee_lamports, is_durable_nonce,
               programs_called, block_time, created_at
        FROM forensics_history
        WHERE wallet = $1
        ORDER BY created_at DESC
        LIMIT 100
        "#,
        params.wallet,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let total = rows.len() as i64;
    let transactions = rows.into_iter().map(|r| r.into()).collect();
    Ok(Json(TxHistoryResponse {
        transactions,
        total,
    }))
}
