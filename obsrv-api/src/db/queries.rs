use chrono::Utc;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

// WATCHED WALLETS
#[derive(Debug, sqlx::FromRow)]
pub struct WatchedWallet {
    pub wallet: String,
    pub telegram_chat_id: String,
    pub alert_threshold: i32,
    pub active: bool,
    pub created_at: i64,
}

pub async fn insert_watched_wallet(
    pool: &PgPool,
    wallet: &str,
    telegram_chat_id: &str,
    alert_threshold: i32,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().timestamp();

    sqlx::query!(
        r#"
        INSERT INTO watched_wallets
            (wallet, telegram_chat_id, alert_threshold, active, created_at)
        VALUES ($1, $2, $3, TRUE, $4)

        ON CONFLICT(wallet)
        DO UPDATE SET
            telegram_chat_id = EXCLUDED.telegram_chat_id,
            alert_threshold  = EXCLUDED.alert_threshold,
            active            = TRUE
        "#,
        wallet,
        telegram_chat_id,
        alert_threshold,
        now,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_watched_wallets(pool: &PgPool) -> Result<Vec<WatchedWallet>, sqlx::Error> {
    sqlx::query_as!(
        WatchedWallet,
        r#"
        SELECT wallet, telegram_chat_id, alert_threshold, active, created_at
        FROM watched_wallets
        WHERE active = TRUE
        "#
    )
    .fetch_all(pool)
    .await
}

pub async fn deactivate_watched_wallet(pool: &PgPool, wallet: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE watched_wallets
        SET active = FALSE
        WHERE wallet = $1
        "#,
        wallet
    )
    .execute(pool)
    .await?;

    Ok(())
}

// WATCHED PROGRAMS
#[derive(Debug, sqlx::FromRow)]
pub struct WatchedProgram {
    pub program_id: String,
    pub name: Option<String>,
    pub active: bool,
    pub created_at: i64,
}

pub async fn insert_watched_program(
    pool: &PgPool,
    program_id: &str,
    name: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().timestamp();

    sqlx::query!(
        r#"
        INSERT INTO watched_programs
            (program_id, name, active, created_at)
        VALUES ($1, $2, TRUE, $3)

        ON CONFLICT(program_id)
        DO UPDATE SET
            name = COALESCE(EXCLUDED.name, watched_programs.name),
            active = TRUE
        "#,
        program_id,
        name,
        now,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_watched_programs(pool: &PgPool) -> Result<Vec<WatchedProgram>, sqlx::Error> {
    sqlx::query_as!(
        WatchedProgram,
        r#"
        SELECT program_id, name, active, created_at
        FROM watched_programs
        WHERE active = TRUE
        "#
    )
    .fetch_all(pool)
    .await
}

// FORENSICS HISTORY
#[derive(Debug, sqlx::FromRow)]
pub struct ForensicsHistory {
    pub id: i64,
    pub signature: String,
    pub slot: i64,
    pub wallet: String,
    pub risk_score: i32,
    pub execution_status: String,
    pub failure_reason: Option<String>,
    pub cu_consumed: Option<i64>,
    pub fee_lamports: i64,
    pub is_durable_nonce: bool,
    pub programs_called: Option<JsonValue>,
    pub block_time: Option<i64>,
    pub created_at: i64,
}

pub async fn insert_forensics_history(
    pool: &PgPool,
    signature: &str,
    slot: i64,
    wallet: &str,
    risk_score: i32,
    execution_status: &str,
    failure_reason: Option<&str>,
    cu_consumed: Option<i64>,
    fee_lamports: i64,
    is_durable_nonce: bool,
    programs_called: Option<JsonValue>,
    block_time: Option<i64>,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().timestamp();

    sqlx::query!(
        r#"
        INSERT INTO forensics_history
            (signature, slot, wallet, risk_score, execution_status,
             failure_reason, cu_consumed, fee_lamports, is_durable_nonce,
             programs_called, block_time, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT(signature) DO NOTHING
        "#,
        signature,
        slot,
        wallet,
        risk_score,
        execution_status,
        failure_reason,
        cu_consumed,
        fee_lamports,
        is_durable_nonce,
        programs_called,
        block_time,
        now,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_forensics_by_wallet(
    pool: &PgPool,
    wallet: &str,
    limit: i64,
) -> Result<Vec<ForensicsHistory>, sqlx::Error> {
    sqlx::query_as!(
        ForensicsHistory,
        r#"
        SELECT id, signature, slot, wallet, risk_score, execution_status,
               failure_reason, cu_consumed, fee_lamports, is_durable_nonce,
               programs_called, block_time, created_at
        FROM forensics_history
        WHERE wallet = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
        wallet,
        limit
    )
    .fetch_all(pool)
    .await
}

pub async fn get_forensics_by_signature(
    pool: &PgPool,
    signature: &str,
) -> Result<Option<ForensicsHistory>, sqlx::Error> {
    sqlx::query_as!(
        ForensicsHistory,
        r#"
        SELECT id, signature, slot, wallet, risk_score, execution_status,
               failure_reason, cu_consumed, fee_lamports, is_durable_nonce,
               programs_called, block_time, created_at
        FROM forensics_history
        WHERE signature = $1
        "#,
        signature
    )
    .fetch_optional(pool)
    .await
}

// WALLET ANALYTICS
#[derive(Debug, sqlx::FromRow)]
pub struct WalletAnalytics {
    pub wallet: String,
    pub total_txs: i64,
    pub failed_txs: i64,
    pub total_cu: i64,
    pub avg_cu: f64,
    pub total_fees: i64,
    pub high_risk_txs: i64,
    pub last_updated: i64,
}

pub async fn upsert_wallet_analytics(
    pool: &PgPool,
    wallet: &str,
    total_txs: i64,
    failed_txs: i64,
    total_cu: i64,
    avg_cu: f64,
    total_fees: i64,
    high_risk_txs: i64,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().timestamp();

    sqlx::query!(
        r#"
        INSERT INTO wallet_analytics
            (wallet, total_txs, failed_txs, total_cu, avg_cu,
             total_fees, high_risk_txs, last_updated)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT(wallet)
        DO UPDATE SET
            total_txs = EXCLUDED.total_txs,
            failed_txs = EXCLUDED.failed_txs,
            total_cu = EXCLUDED.total_cu,
            avg_cu = EXCLUDED.avg_cu,
            total_fees = EXCLUDED.total_fees,
            high_risk_txs = EXCLUDED.high_risk_txs,
            last_updated = EXCLUDED.last_updated
        "#,
        wallet,
        total_txs,
        failed_txs,
        total_cu,
        avg_cu,
        total_fees,
        high_risk_txs,
        now,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_wallet_analytics(
    pool: &PgPool,
    wallet: &str,
) -> Result<Option<WalletAnalytics>, sqlx::Error> {
    sqlx::query_as!(
        WalletAnalytics,
        r#"
        SELECT wallet, total_txs, failed_txs, total_cu, avg_cu,
               total_fees, high_risk_txs, last_updated
        FROM wallet_analytics
        WHERE wallet = $1
        "#,
        wallet
    )
    .fetch_optional(pool)
    .await
}

// WALLET PROGRAM STATS
#[derive(Debug, sqlx::FromRow)]
pub struct WalletProgramStats {
    pub wallet: String,
    pub program_id: String,
    pub call_count: i64,
    pub failed_count: i64,
    pub total_cu: i64,
    pub avg_cu: f64,
    pub last_called: Option<i64>,
}

pub async fn upsert_wallet_program_stats(
    pool: &PgPool,
    wallet: &str,
    program_id: &str,
    call_count: i64,
    failed_count: i64,
    total_cu: i64,
    avg_cu: f64,
    last_called: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO wallet_program_stats
            (wallet, program_id, call_count, failed_count, total_cu, avg_cu, last_called)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT(wallet, program_id)
        DO UPDATE SET
            call_count = EXCLUDED.call_count,
            failed_count = EXCLUDED.failed_count,
            total_cu = EXCLUDED.total_cu,
            avg_cu = EXCLUDED.avg_cu,
            last_called = EXCLUDED.last_called
        "#,
        wallet,
        program_id,
        call_count,
        failed_count,
        total_cu,
        avg_cu,
        last_called,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_wallet_program_stats(
    pool: &PgPool,
    wallet: &str,
) -> Result<Vec<WalletProgramStats>, sqlx::Error> {
    sqlx::query_as!(
        WalletProgramStats,
        r#"
        SELECT wallet, program_id, call_count, failed_count,
               total_cu, avg_cu, last_called
        FROM wallet_program_stats
        WHERE wallet = $1
        ORDER BY call_count DESC
        "#,
        wallet
    )
    .fetch_all(pool)
    .await
}

// PROGRAM ANALYTICS
#[derive(Debug, sqlx::FromRow)]
pub struct ProgramAnalytics {
    pub program_id: String,
    pub program_name: Option<String>,
    pub total_calls: i64,
    pub failed_calls: i64,
    pub total_cu: i64,
    pub avg_cu: f64,
    pub peak_cu: i64,
    pub unique_wallets: i64,
    pub first_seen: Option<i64>,
    pub last_seen: Option<i64>,
}

pub async fn upsert_program_analytics(
    pool: &PgPool,
    program_id: &str,
    program_name: Option<&str>,
    total_calls: i64,
    failed_calls: i64,
    total_cu: i64,
    avg_cu: f64,
    peak_cu: i64,
    unique_wallets: i64,
    first_seen: Option<i64>,
    last_seen: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO program_analytics
            (program_id, program_name, total_calls, failed_calls,
             total_cu, avg_cu, peak_cu, unique_wallets,
             first_seen, last_seen)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT(program_id)
        DO UPDATE SET
            program_name = COALESCE(EXCLUDED.program_name, program_analytics.program_name),
            total_calls = EXCLUDED.total_calls,
            failed_calls = EXCLUDED.failed_calls,
            total_cu = EXCLUDED.total_cu,
            avg_cu = EXCLUDED.avg_cu,
            peak_cu = EXCLUDED.peak_cu,
            unique_wallets = EXCLUDED.unique_wallets,
            first_seen = COALESCE(program_analytics.first_seen, EXCLUDED.first_seen),
            last_seen = EXCLUDED.last_seen
        "#,
        program_id,
        program_name,
        total_calls,
        failed_calls,
        total_cu,
        avg_cu,
        peak_cu,
        unique_wallets,
        first_seen,
        last_seen,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_program_analytics(
    pool: &PgPool,
    program_id: &str,
) -> Result<Option<ProgramAnalytics>, sqlx::Error> {
    sqlx::query_as!(
        ProgramAnalytics,
        r#"
        SELECT program_id, program_name, total_calls, failed_calls,
               total_cu, avg_cu, peak_cu, unique_wallets,
               first_seen, last_seen
        FROM program_analytics
        WHERE program_id = $1
        "#,
        program_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_top_programs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ProgramAnalytics>, sqlx::Error> {
    sqlx::query_as!(
        ProgramAnalytics,
        r#"
        SELECT program_id, program_name, total_calls, failed_calls,
               total_cu, avg_cu, peak_cu, unique_wallets,
               first_seen, last_seen
        FROM program_analytics
        ORDER BY total_calls DESC
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}

// STREAM CHECKPOINT
pub async fn get_stream_checkpoint(pool: &PgPool) -> Result<Option<i64>, sqlx::Error> {
    // add explicit type annotation
    let result: Option<_> = sqlx::query!(
        r#"
        SELECT last_slot
        FROM stream_checkpoint
        ORDER BY id DESC
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| r.last_slot))
}

pub async fn update_stream_checkpoint(pool: &PgPool, last_slot: i64) -> Result<(), sqlx::Error> {
    let now = Utc::now().timestamp();

    sqlx::query!(
        r#"
        UPDATE stream_checkpoint
        SET last_slot  = $1,
            updated_at = $2
        WHERE id = 1
        "#,
        last_slot,
        now,
    )
    .execute(pool)
    .await?;

    Ok(())
}
