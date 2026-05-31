use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{errors::ApiError, state::AppState};

#[derive(Debug, Deserialize)]
pub struct WalletAnalyticsParam {
    pub wallet: String,
}

#[derive(Debug, Deserialize)]
pub struct ProgramAnalyticsParam {
    pub program_id: String,
}

#[derive(Debug, Deserialize)]
pub struct LimitParam {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AlertsParam {
    pub wallet: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct WalletAnalyticsResponse {
    pub summary: Option<WalletSummary>,
    pub programs: Vec<WalletProgramRow>,
    pub instructions: Vec<WalletInstructionRow>,
    pub alerts_24h: i64,
    pub last_alert: Option<AlertRow>,
}

#[derive(Debug, Serialize)]
pub struct ProgramAnalyticsResponse {
    pub summary: Option<ProgramSummary>,
    pub instructions: Vec<InstructionWithTrend>,
}

#[derive(Debug, Serialize)]
pub struct TopProgramsResponse {
    pub programs: Vec<ProgramSummary>,
}

#[derive(Debug, Serialize)]
pub struct AlertsResponse {
    pub alerts: Vec<AlertRow>,
}

#[derive(Debug, Serialize)]
pub struct WalletSummary {
    pub wallet: String,
    pub total_txs: i64,
    pub failed_txs: i64,
    pub success_txs: i64,
    pub success_rate: f64,
    pub total_cu: i64,
    pub avg_cu: f64,
    pub total_fees: i64,
    pub high_risk_txs: i64,
    pub risk_distribution: JsonValue,
    pub last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct WalletProgramRow {
    pub program_id: String,
    pub program_name: Option<String>,
    pub call_count: i64,
    pub failed_count: i64,
    pub success_count: i64,
    pub success_rate: f64,
    pub total_cu: i64,
    pub avg_cu: f64,
    pub last_called: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct WalletInstructionRow {
    pub program_id: String,
    pub instruction_type: String,
    pub call_count: i64,
    pub failed_count: i64,
    pub success_count: i64,
    pub success_rate: f64,
    pub total_cu: i64,
    pub avg_cu: f64,
    pub last_called: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ProgramSummary {
    pub program_id: String,
    pub program_name: Option<String>,
    pub total_calls: i64,
    pub failed_calls: i64,
    pub success_calls: i64,
    pub success_rate: f64,
    pub total_cu: i64,
    pub avg_cu: f64,
    pub peak_cu: i64,
    pub unique_wallets: i64,
    pub risk_distribution: JsonValue,
    pub first_seen: Option<i64>,
    pub last_seen: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct InstructionWithTrend {
    pub program_id: String,
    pub instruction_type: String,
    pub call_count: i64,
    pub failed_count: i64,
    pub success_count: i64,
    pub success_rate: f64,
    pub total_cu: i64,
    pub avg_cu: f64,
    pub peak_cu: i64,
    pub unique_callers: i64,
    pub risk_distribution: JsonValue,
    pub first_called: Option<i64>,
    pub last_called: Option<i64>,
    pub trend_7d: Vec<InstructionDailyRow>,
}

#[derive(Debug, Serialize)]
pub struct InstructionDailyRow {
    pub date: String,
    pub call_count: i64,
    pub failed_count: i64,
    pub success_count: i64,
    pub avg_cu: f64,
    pub peak_cu: i64,
}

#[derive(Debug, Serialize)]
pub struct AlertRow {
    pub wallet: String,
    pub signature: String,
    pub slot: i64,
    pub risk_score: i32,
    pub summary: String,
    pub programs: Option<JsonValue>,
    pub is_durable_nonce: bool,
    pub created_at: i64,
}

fn success_rate(total: i64, failed: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        ((total - failed) as f64 / total as f64) * 100.0
    }
}

pub async fn wallet(
    State(state): State<AppState>,
    Query(params): Query<WalletAnalyticsParam>,
) -> Result<Json<WalletAnalyticsResponse>, ApiError> {
    let summary_row = sqlx::query!(
        r#"
        SELECT wallet, total_txs, failed_txs, total_cu, avg_cu,
               total_fees, high_risk_txs, last_updated,
               COALESCE(risk_distribution, '{}'::jsonb) as "risk_distribution!: JsonValue"
        FROM wallet_analytics
        WHERE wallet = $1
        "#,
        params.wallet,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let summary = summary_row.map(|r| WalletSummary {
        wallet: r.wallet,
        total_txs: r.total_txs,
        failed_txs: r.failed_txs,
        success_txs: r.total_txs - r.failed_txs,
        success_rate: success_rate(r.total_txs, r.failed_txs),
        total_cu: r.total_cu,
        avg_cu: r.avg_cu,
        total_fees: r.total_fees,
        high_risk_txs: r.high_risk_txs,
        risk_distribution: r.risk_distribution,
        last_updated: r.last_updated,
    });

    let program_rows = sqlx::query!(
        r#"
        SELECT w.program_id, p.name as "program_name?",
               w.call_count, w.failed_count, w.total_cu, w.avg_cu, w.last_called
        FROM wallet_program_stats w
        LEFT JOIN watched_programs p ON p.program_id = w.program_id AND p.active = true
        WHERE w.wallet = $1
        ORDER BY w.call_count DESC
        LIMIT 25
        "#,
        params.wallet,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let programs = program_rows
        .into_iter()
        .map(|r| WalletProgramRow {
            program_id: r.program_id,
            program_name: r.program_name,
            call_count: r.call_count,
            failed_count: r.failed_count,
            success_count: r.call_count - r.failed_count,
            success_rate: success_rate(r.call_count, r.failed_count),
            total_cu: r.total_cu,
            avg_cu: r.avg_cu,
            last_called: r.last_called,
        })
        .collect();

    let instruction_rows = sqlx::query!(
        r#"
        SELECT program_id, instruction_type, call_count, failed_count,
               total_cu, avg_cu, last_called
        FROM wallet_ix_stats
        WHERE wallet = $1
        ORDER BY call_count DESC
        LIMIT 50
        "#,
        params.wallet,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let instructions = instruction_rows
        .into_iter()
        .map(|r| WalletInstructionRow {
            program_id: r.program_id,
            instruction_type: r.instruction_type,
            call_count: r.call_count,
            failed_count: r.failed_count,
            success_count: r.call_count - r.failed_count,
            success_rate: success_rate(r.call_count, r.failed_count),
            total_cu: r.total_cu,
            avg_cu: r.avg_cu,
            last_called: r.last_called,
        })
        .collect();

    let alerts_24h = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM alerts_history
        WHERE wallet = $1
          AND created_at > EXTRACT(EPOCH FROM NOW() - INTERVAL '24 hours')::bigint
        "#,
        params.wallet,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let last_alert = fetch_alerts(&state.db, Some(&params.wallet), 1)
        .await?
        .into_iter()
        .next();

    Ok(Json(WalletAnalyticsResponse {
        summary,
        programs,
        instructions,
        alerts_24h,
        last_alert,
    }))
}

pub async fn program(
    State(state): State<AppState>,
    Query(params): Query<ProgramAnalyticsParam>,
) -> Result<Json<ProgramAnalyticsResponse>, ApiError> {
    let summary = fetch_program_summary(&state.db, &params.program_id).await?;

    let instruction_rows = sqlx::query!(
        r#"
        SELECT program_id, instruction_type,
               call_count::bigint as "call_count!",
               failed_count::bigint as "failed_count!",
               total_cu::bigint as "total_cu!",
               avg_cu::float8 as "avg_cu!",
               peak_cu::bigint as "peak_cu!",
               unique_callers::bigint as "unique_callers!",
               first_called::bigint as first_called,
               last_called::bigint as last_called,
               COALESCE(risk_distribution, '{}'::jsonb) as "risk_distribution!: JsonValue"
        FROM instruction_analytics
        WHERE program_id = $1
        ORDER BY call_count DESC
        LIMIT 50
        "#,
        params.program_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let mut instructions = Vec::with_capacity(instruction_rows.len());
    for r in instruction_rows {
        let trend_7d =
            fetch_instruction_trend(&state.db, &r.program_id, &r.instruction_type).await?;

        instructions.push(InstructionWithTrend {
            program_id: r.program_id,
            instruction_type: r.instruction_type,
            call_count: r.call_count,
            failed_count: r.failed_count,
            success_count: r.call_count - r.failed_count,
            success_rate: success_rate(r.call_count, r.failed_count),
            total_cu: r.total_cu,
            avg_cu: r.avg_cu,
            peak_cu: r.peak_cu,
            unique_callers: r.unique_callers,
            risk_distribution: r.risk_distribution,
            first_called: r.first_called,
            last_called: r.last_called,
            trend_7d,
        });
    }

    Ok(Json(ProgramAnalyticsResponse {
        summary,
        instructions,
    }))
}

pub async fn top_programs(
    State(state): State<AppState>,
    Query(params): Query<LimitParam>,
) -> Result<Json<TopProgramsResponse>, ApiError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);

    let rows = sqlx::query!(
        r#"
        SELECT program_id, program_name, total_calls, failed_calls, total_cu,
               avg_cu, peak_cu, unique_wallets, first_seen, last_seen,
               COALESCE(risk_distribution, '{}'::jsonb) as "risk_distribution!: JsonValue"
        FROM program_analytics
        ORDER BY total_calls DESC
        LIMIT $1
        "#,
        limit,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let programs = rows
        .into_iter()
        .map(|r| ProgramSummary {
            program_id: r.program_id,
            program_name: r.program_name,
            total_calls: r.total_calls,
            failed_calls: r.failed_calls,
            success_calls: r.total_calls - r.failed_calls,
            success_rate: success_rate(r.total_calls, r.failed_calls),
            total_cu: r.total_cu,
            avg_cu: r.avg_cu,
            peak_cu: r.peak_cu,
            unique_wallets: r.unique_wallets,
            risk_distribution: r.risk_distribution,
            first_seen: r.first_seen,
            last_seen: r.last_seen,
        })
        .collect();

    Ok(Json(TopProgramsResponse { programs }))
}

pub async fn alerts(
    State(state): State<AppState>,
    Query(params): Query<AlertsParam>,
) -> Result<Json<AlertsResponse>, ApiError> {
    let limit = params.limit.unwrap_or(25).clamp(1, 100);
    let alerts = fetch_alerts(&state.db, params.wallet.as_deref(), limit).await?;
    Ok(Json(AlertsResponse { alerts }))
}

async fn fetch_program_summary(
    db: &sqlx::PgPool,
    program_id: &str,
) -> Result<Option<ProgramSummary>, ApiError> {
    let row = sqlx::query!(
        r#"
        SELECT program_id, program_name, total_calls, failed_calls, total_cu,
               avg_cu, peak_cu, unique_wallets, first_seen, last_seen,
               COALESCE(risk_distribution, '{}'::jsonb) as "risk_distribution!: JsonValue"
        FROM program_analytics
        WHERE program_id = $1
        "#,
        program_id,
    )
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    Ok(row.map(|r| ProgramSummary {
        program_id: r.program_id,
        program_name: r.program_name,
        total_calls: r.total_calls,
        failed_calls: r.failed_calls,
        success_calls: r.total_calls - r.failed_calls,
        success_rate: success_rate(r.total_calls, r.failed_calls),
        total_cu: r.total_cu,
        avg_cu: r.avg_cu,
        peak_cu: r.peak_cu,
        unique_wallets: r.unique_wallets,
        risk_distribution: r.risk_distribution,
        first_seen: r.first_seen,
        last_seen: r.last_seen,
    }))
}

async fn fetch_instruction_trend(
    db: &sqlx::PgPool,
    program_id: &str,
    instruction_type: &str,
) -> Result<Vec<InstructionDailyRow>, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT date::text as "date!", call_count, failed_count, avg_cu, peak_cu
        FROM instruction_daily
        WHERE program_id = $1
          AND instruction_type = $2
          AND date::date >= CURRENT_DATE - INTERVAL '7 days'
        ORDER BY date ASC
        "#,
        program_id,
        instruction_type,
    )
    .fetch_all(db)
    .await
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(|r| InstructionDailyRow {
            date: r.date,
            call_count: r.call_count,
            failed_count: r.failed_count,
            success_count: r.call_count - r.failed_count,
            avg_cu: r.avg_cu,
            peak_cu: r.peak_cu,
        })
        .collect())
}

async fn fetch_alerts(
    db: &sqlx::PgPool,
    wallet: Option<&str>,
    limit: i64,
) -> Result<Vec<AlertRow>, ApiError> {
    let rows = if let Some(wallet) = wallet {
        sqlx::query_as!(
            AlertRow,
            r#"
            SELECT wallet, signature, slot, risk_score, summary, programs,
                   is_durable_nonce, created_at
            FROM alerts_history
            WHERE wallet = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            wallet,
            limit,
        )
        .fetch_all(db)
        .await
    } else {
        sqlx::query_as!(
            AlertRow,
            r#"
            SELECT wallet, signature, slot, risk_score, summary, programs,
                   is_durable_nonce, created_at
            FROM alerts_history
            ORDER BY created_at DESC
            LIMIT $1
            "#,
            limit,
        )
        .fetch_all(db)
        .await
    }
    .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    Ok(rows)
}
