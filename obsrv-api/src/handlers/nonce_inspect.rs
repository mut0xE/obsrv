use crate::{errors::ApiError, state::AppState};

use axum::{Json, extract::State};

use obsrv_core::types::{NonceInspectRequest, NonceInspectResponse};

use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};

use std::str::FromStr;

pub async fn handle(
    State(state): State<AppState>,
    Json(req): Json<NonceInspectRequest>,
) -> Result<Json<NonceInspectResponse>, ApiError> {
    tracing::info!(
        nonce_account = %req.nonce_account,
        "POST /nonce/inspect"
    );

    // 1. PARSE PUBKEY

    let pubkey = Pubkey::from_str(&req.nonce_account)
        .map_err(|e| ApiError::BadRequest(format!("invalid pubkey: {}", e)))?;

    // 2. FETCH ACCOUNT FROM RPC
    let account = state
        .rpc
        .get_account(&pubkey)
        .map_err(|e| ApiError::InternalError(format!("RPC error: {}", e)))?;

    // 3. PARSE ACCOUNT USING solana-account-decoder
    let parsed_account =
        obsrv_core::account_decoder::decode_account_state(&pubkey, &account.owner, &account.data)
            .ok_or_else(|| ApiError::BadRequest("account could not be parsed".to_string()))?;

    tracing::debug!(
        ?parsed_account,
        "parsed nonce account via solana-account-decoder"
    );

    // 4. ENSURE THIS IS A NONCE ACCOUNT
    if parsed_account.program != "nonce" {
        return Err(ApiError::BadRequest(format!(
            "account is a {} account, expected nonce account",
            parsed_account.program
        )));
    }

    // 5. EXTRACT PARSED JSON OBJECT
    let parsed = parsed_account
        .parsed
        .as_object()
        .ok_or_else(|| ApiError::InternalError("invalid parsed nonce account".to_string()))?;

    // 6. READ NONCE STATE
    let nonce_state = parsed
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // 7. READ INFO OBJECT
    let info = parsed
        .get("info")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ApiError::InternalError("parsed nonce account missing info".to_string()))?;

    // 8. EXTRACT AUTHORITY
    let authority = info
        .get("authority")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // 9. EXTRACT DURABLE NONCE VALUE
    let nonce_value = info
        .get("blockhash")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // 10. EXTRACT FEE CALCULATOR
    let lamports_per_sig = info
        .get("feeCalculator")
        .and_then(|v| v.get("lamportsPerSignature"))
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    // 11. ACCOUNT BALANCE
    let lamports = account.lamports;

    let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;

    // 12. VERSION
    let version = match u32::from_le_bytes(account.data[0..4].try_into().unwrap()) {
        0 => "legacy",
        1 => "current",
        _ => "unknown",
    };

    // 13. BUILD RISK FLAGS
    let mut risk_flags = vec![];

    match nonce_state {
        "initialized" => {
            risk_flags.push(format!(
                "ACTIVE durable nonce account controlled by {}",
                authority
            ));

            risk_flags
                .push("transactions using this nonce may remain valid indefinitely".to_string());

            risk_flags
                .push("durable nonce transactions bypass normal blockhash expiry".to_string());

            risk_flags.push(format!(
                "{} lamports charged per signature",
                lamports_per_sig
            ));
        }

        "uninitialized" => {
            risk_flags.push("nonce account exists but is not initialized".to_string());

            risk_flags.push("cannot yet be used for durable nonce transactions".to_string());
        }

        _ => {
            risk_flags.push("unknown nonce account state detected".to_string());
        }
    }

    // 14. RISK LEVEL
    let risk_level = match nonce_state {
        "initialized" => "warning",
        "uninitialized" => "info",
        _ => "warning",
    };

    // 15. LOG RESULT
    tracing::info!(
        nonce_account    = %req.nonce_account,
        nonce_state      = %nonce_state,
        authority        = %authority,
        nonce_value      = %nonce_value,
        lamports_per_sig = lamports_per_sig,
        "nonce inspect complete"
    );

    // 16. RESPONSE
    Ok(Json(NonceInspectResponse {
        nonce_account: req.nonce_account,
        version: version.to_string(),
        state: nonce_state.to_string(),
        authority: authority.to_string(),
        nonce_value: nonce_value.to_string(),
        lamports,
        sol,
        lamports_per_sig,
        risk_flags,
        risk_level: risk_level.to_string(),
    }))
}
