use crate::{errors::ApiError, state::AppState};
use axum::{Json, extract::State};
use obsrv_core::types::{NonceInspectRequest, NonceInspectResponse};
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use std::str::FromStr;

/*
[0..4]  = Versions discriminant  u32 le  0=Legacy 1=Current
[4..8]  = State discriminant     u32 le  0=Uninitialized 1=Initialized
[8..40] = authority              Pubkey  32 raw bytes
[40..72]= nonce value            Hash    32 raw bytes
[72..80]= lamports_per_sig       u64 le  = 5000
*/

pub async fn handle(
    State(state): State<AppState>,
    Json(req): Json<NonceInspectRequest>,
) -> Result<Json<NonceInspectResponse>, ApiError> {
    tracing::info!(nonce_account = %req.nonce_account, "POST /nonce/inspect");

    // parse pubkey
    let pubkey = Pubkey::from_str(&req.nonce_account)
        .map_err(|e| ApiError::BadRequest(format!("invalid pubkey: {}", e)))?;

    // fetch account from chain
    let account = state
        .helius
        .connection()
        .get_account(&pubkey)
        .map_err(|e| ApiError::InternalError(format!("RPC error: {}", e)))?;

    let lamports = account.lamports;
    let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
    let data = &account.data;

    tracing::debug!("nonce account data: {} bytes {:?}", data.len(), data);

    // validate nonce account size
    if data.len() != 80 {
        return Err(ApiError::BadRequest(format!(
            "account data is {} bytes, expected 80 — not a nonce account",
            data.len()
        )));
    }

    // bytes [0..4] = Versions discriminant
    let versions_bytes: [u8; 4] = data[0..4]
        .try_into()
        .map_err(|_| ApiError::InternalError("failed to read versions bytes".to_string()))?;
    let versions_val = u32::from_le_bytes(versions_bytes);
    let versions_str = match versions_val {
        0 => "legacy",
        1 => "current",
        _ => "unknown",
    };

    // bytes [4..8] = State discriminant
    let state_bytes: [u8; 4] = data[4..8]
        .try_into()
        .map_err(|_| ApiError::InternalError("failed to read state bytes".to_string()))?;
    let state_val = u32::from_le_bytes(state_bytes);
    let nonce_state = match state_val {
        0 => "uninitialized",
        1 => "initialized",
        _ => "unknown",
    };

    // bytes [8..40] = authority pubkey
    let authority_bytes: [u8; 32] = data[8..40]
        .try_into()
        .map_err(|_| ApiError::InternalError("failed to read authority bytes".to_string()))?;
    let authority = Pubkey::from(authority_bytes);

    // bytes [40..72] = nonce value (blockhash)
    let nonce_bytes: [u8; 32] = data[40..72]
        .try_into()
        .map_err(|_| ApiError::InternalError("failed to read nonce value bytes".to_string()))?;
    let nonce_value = solana_sdk::hash::Hash::new_from_array(nonce_bytes).to_string();

    // bytes [72..80] = lamports per signature
    let fee_bytes: [u8; 8] = data[72..80]
        .try_into()
        .map_err(|_| ApiError::InternalError("failed to read fee calculator bytes".to_string()))?;
    let lamports_per_sig = u64::from_le_bytes(fee_bytes);

    // build risk flags
    let mut risk_flags = vec![];

    if nonce_state == "initialized" {
        risk_flags.push(format!(
            "nonce account is ACTIVE — authority: {}",
            authority.to_string()
        ));
        risk_flags
            .push("any transaction using this nonce as blockhash will never expire".to_string());
        risk_flags.push(format!("fee: {} lamports per signature", lamports_per_sig));
    }

    if nonce_state == "uninitialized" {
        risk_flags.push("nonce account is not yet initialized".to_string());
        risk_flags.push("cannot be used for durable nonce transactions yet".to_string());
    }

    if versions_str == "legacy" {
        risk_flags.push("legacy nonce account format — consider upgrading".to_string());
    }

    let risk_level = if nonce_state == "initialized" {
        "warning"
    } else {
        "info"
    };

    tracing::info!(
        nonce_account    = %req.nonce_account,
        versions         = %versions_str,
        state            = %nonce_state,
        authority        = %authority,
        lamports_per_sig = lamports_per_sig,
        "nonce inspect complete"
    );

    Ok(Json(NonceInspectResponse {
        nonce_account: req.nonce_account,
        version: versions_str.to_string(),
        state: nonce_state.to_string(),
        authority: authority.to_string(),
        nonce_value,
        lamports,
        sol,
        lamports_per_sig,
        risk_flags,
        risk_level: risk_level.to_string(),
    }))
}
