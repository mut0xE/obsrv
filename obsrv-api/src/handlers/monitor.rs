use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{db::queries, errors::ApiError, state::AppState};

// REQUEST / RESPONSE TYPES
#[derive(Debug, Deserialize)]
pub struct AddWalletRequest {
    pub wallet: String,
    pub telegram_chat_id: Option<String>, // optional: Telegram alerts coming soon
    pub alert_threshold: Option<i32>, // default 7
}

#[derive(Debug, Deserialize)]
pub struct RemoveWalletRequest {
    pub wallet: String,
}

#[derive(Debug, Deserialize)]
pub struct AddProgramRequest {
    pub program_id: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RemoveProgramRequest {
    pub program_id: String,
}

#[derive(Debug, Serialize)]
pub struct MonitorResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub already_watching: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MonitorListResponse {
    pub wallets: Vec<WalletEntry>,
    pub programs: Vec<ProgramEntry>,
}

#[derive(Debug, Serialize)]
pub struct WalletEntry {
    pub wallet: String,
    pub telegram_chat_id: Option<String>,
    pub alert_threshold: i32,
    pub created_at: i64,
    pub active: bool,
}

#[derive(Debug, Serialize)]
pub struct ProgramEntry {
    pub program_id: String,
    pub name: Option<String>,
    pub created_at: i64,
    pub active: bool,
}

// POST /monitor/wallet
pub async fn add_wallet(
    State(state): State<AppState>,
    Json(req): Json<AddWalletRequest>,
) -> Result<Json<MonitorResponse>, ApiError> {
    tracing::info!(wallet = %req.wallet, "POST /monitor/wallet");

    // validate wallet address
    validate_pubkey(&req.wallet)?;

    // validate telegram chat id (commented out - feature coming soon)
    // if let Some(ref chat_id) = req.telegram_chat_id {
    //     validate_telegram_chat_id(chat_id)?;
    // }

    let threshold = req.alert_threshold.unwrap_or(7).clamp(1, 10);

    // check if already watching with exact same settings
    if let Some(existing) = queries::get_watched_wallet(&state.db, &req.wallet).await {
        if existing.active
            && existing.telegram_chat_id == req.telegram_chat_id  // ← WatchedWallet has this field
            && existing.alert_threshold  == threshold
        {
            tracing::info!(
                wallet = %req.wallet,
                "wallet already watched with same settings"
            );
            return Ok(Json(MonitorResponse {
                success: true,
                already_watching: Some(true),
                message: format!(
                    "already watching {} with threshold {}",
                    &req.wallet[..8],
                    threshold
                ),
            }));
        }

        // same wallet different settings → update
        if existing.active {
            tracing::info!(
                wallet    = %req.wallet,
                threshold = threshold,
                "updating wallet monitor settings"
            );
        } else {
            // was removed → reactivating
            tracing::info!(
                wallet = %req.wallet,
                "reactivating wallet monitor"
            );
        }
    }

    // insert or update
    queries::insert_watched_wallet(&state.db, &req.wallet, &req.telegram_chat_id, threshold)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    tracing::info!(
        wallet    = %req.wallet,
        threshold = threshold,
        "wallet added to monitor"
    );

    Ok(Json(MonitorResponse {
        success: true,
        already_watching: Some(false),
        message: format!(
            "watching {} - alerts when risk >= {}",
            req.wallet, threshold
        ),
    }))
}

// POST /monitor/wallet/remove
pub async fn remove_wallet(
    State(state): State<AppState>,
    Json(req): Json<RemoveWalletRequest>,
) -> Result<Json<MonitorResponse>, ApiError> {
    tracing::info!(wallet = %req.wallet, "POST /monitor/wallet/remove");

    validate_pubkey(&req.wallet)?;

    // check if wallet is actually being watched
    match queries::get_watched_wallet(&state.db, &req.wallet).await {
        None => {
            return Ok(Json(MonitorResponse {
                success: false,
                already_watching: Some(false),
                message: format!("{} was never added to monitor", &req.wallet),
            }));
        }
        Some(w) if !w.active => {
            return Ok(Json(MonitorResponse {
                success: false,
                already_watching: Some(false),
                message: format!("{} is already not being watched", &req.wallet),
            }));
        }
        _ => {}
    }

    // deactivate
    queries::deactivate_watched_wallet(&state.db, &req.wallet)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    tracing::info!(wallet = %req.wallet, "wallet removed from monitor");

    Ok(Json(MonitorResponse {
        success: true,
        already_watching: Some(false),
        message: format!(
            "stopped watching {} - historical data preserved",
            req.wallet
        ),
    }))
}

// POST /monitor/program
pub async fn add_program(
    State(state): State<AppState>,
    Json(req): Json<AddProgramRequest>,
) -> Result<Json<MonitorResponse>, ApiError> {
    tracing::info!(program_id = %req.program_id, "POST /monitor/program");

    validate_pubkey(&req.program_id)?;

    // check if already watching with same settings
    if let Some(existing) = queries::get_watched_program(&state.db, &req.program_id).await {
        if existing.active {
            // check if name is same or new name provided
            let same_name = match (&existing.name, &req.name) {
                (Some(a), Some(b)) => a == b,
                (None, None) => true,
                _ => false,
            };

            if same_name {
                tracing::info!(
                    program_id = %req.program_id,
                    "program already watched with same settings"
                );
                return Ok(Json(MonitorResponse {
                    success: true,
                    already_watching: Some(true),
                    message: format!(
                        "already watching program {} ({})",
                        &req.program_id[..8],
                        existing.name.as_deref().unwrap_or("no name")
                    ),
                }));
            }

            // active but different name  update name
            tracing::info!(
                program_id = %req.program_id,
                "updating program name"
            );
        } else {
            // was removed reactivating
            tracing::info!(
                program_id = %req.program_id,
                "reactivating program monitor"
            );
        }
    }

    queries::insert_watched_program(&state.db, &req.program_id, req.name.as_deref())
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    let name = req.name.as_deref().unwrap_or("unknown");

    tracing::info!(
        program_id = %req.program_id,
        name       = %name,
        "program added to monitor"
    );

    Ok(Json(MonitorResponse {
        success: true,
        already_watching: Some(false),
        message: format!("watching program {} ({})", req.program_id, name),
    }))
}

// POST /monitor/program/remove
pub async fn remove_program(
    State(state): State<AppState>,
    Json(req): Json<RemoveProgramRequest>,
) -> Result<Json<MonitorResponse>, ApiError> {
    tracing::info!(program_id = %req.program_id, "POST /monitor/program/remove");

    validate_pubkey(&req.program_id)?;

    // check if program is actually being watched
    match queries::get_watched_program(&state.db, &req.program_id).await {
        None => {
            return Ok(Json(MonitorResponse {
                success: false,
                already_watching: Some(false),
                message: format!(
                    "program {} was never added to monitor",
                    &req.program_id[..8]
                ),
            }));
        }
        Some(p) if !p.active => {
            return Ok(Json(MonitorResponse {
                success: false,
                already_watching: Some(false),
                message: format!(
                    "program {} is already not being watched",
                    &req.program_id[..8]
                ),
            }));
        }
        _ => {}
    }

    queries::deactivate_watched_program(&state.db, &req.program_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    tracing::info!(
        program_id = %req.program_id,
        "program removed from monitor"
    );

    Ok(Json(MonitorResponse {
        success: true,
        already_watching: Some(false),
        message: format!(
            "stopped watching program {} - historical data preserved",
            req.program_id
        ),
    }))
}
// GET /monitor/list
pub async fn list(State(state): State<AppState>) -> Result<Json<MonitorListResponse>, ApiError> {
    tracing::info!("GET /monitor/list");
    let wallets = queries::get_watched_wallets(&state.db)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
    let programs = queries::get_watched_programs(&state.db)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;

    Ok(Json(MonitorListResponse {
        wallets: wallets
            .iter()
            .map(|w| WalletEntry {
                wallet: w.wallet.clone(),
                telegram_chat_id: w.telegram_chat_id.clone(),
                alert_threshold: w.alert_threshold,
                created_at: w.created_at,
                active: w.active,
            })
            .collect(),
        programs: programs
            .into_iter()
            .map(|p| ProgramEntry {
                program_id: p.program_id,
                name: p.name,
                created_at: p.created_at,
                active: p.active,
            })
            .collect(),
    }))
}

fn validate_pubkey(key: &str) -> Result<(), ApiError> {
    if key.len() < 32 || key.len() > 44 {
        return Err(ApiError::BadRequest(format!(
            "invalid address: {} — must be 32-44 chars base58",
            key
        )));
    }

    // check all chars are valid base58
    let valid = key
        .chars()
        .all(|c| "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c));

    if !valid {
        return Err(ApiError::BadRequest(format!(
            "invalid address: {} — contains non-base58 characters",
            key
        )));
    }

    Ok(())
}

fn validate_telegram_chat_id(chat_id: &str) -> Result<(), ApiError> {
    if chat_id.is_empty() {
        return Err(ApiError::BadRequest(
            "telegram_chat_id cannot be empty".to_string(),
        ));
    }

    // telegram chat IDs are numeric, sometimes negative for groups
    let valid = chat_id
        .trim_start_matches('-')
        .chars()
        .all(|c| c.is_ascii_digit());

    if !valid {
        return Err(ApiError::BadRequest(format!(
            "invalid telegram_chat_id: {} \
             — must be numeric (get it from @userinfobot on Telegram)",
            chat_id
        )));
    }

    Ok(())
}
