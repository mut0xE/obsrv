use axum::{Json, extract::State};
use obsrv_core::{
    analyzer,
    decoder::decode_payload,
    types::{ApiResponse, TxResponse},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    errors::ApiError,
    response::{build_analysis, build_expected_balances, build_instructions, build_meta},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub raw_tx: Option<String>,
    pub signature: Option<String>,
}

pub async fn handle(
    State(state): State<AppState>,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<ApiResponse>, ApiError> {
    tracing::info!("POST /analyze");

    let payload = if let Some(raw_tx) = req.raw_tx.as_deref().filter(|s| !s.trim().is_empty()) {
        decode_payload(raw_tx).map_err(ApiError::from)?
    } else if let Some(signature) = req.signature.as_deref().filter(|s| !s.trim().is_empty()) {
        let raw_tx = fetch_transaction_base64(&state.config.rpc_url, signature).await?;
        decode_payload(&raw_tx).map_err(ApiError::from)?
    } else {
        return Err(ApiError::BadRequest(
            "expected raw_tx or signature in request body".to_string(),
        ));
    };

    let report = analyzer::analyze(&payload).map_err(ApiError::from)?;

    Ok(Json(ApiResponse {
        tx: TxResponse {
            meta: build_meta(&report),
            analysis: build_analysis(&report),
            instructions: build_instructions(&report),
            balances: build_expected_balances(&report),
            simulation: None,
            forensics: None,
        },
    }))
}

async fn fetch_transaction_base64(rpc_url: &str, signature: &str) -> Result<String, ApiError> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            signature,
            {
                "encoding": "base64",
                "commitment": "confirmed",
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let value: serde_json::Value = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("RPC request failed: {}", e)))?
        .json()
        .await
        .map_err(|e| ApiError::InternalError(format!("RPC response decode failed: {}", e)))?;

    if let Some(error) = value.get("error") {
        return Err(ApiError::InternalError(format!("RPC error: {}", error)));
    }

    let result = value
        .get("result")
        .filter(|v| !v.is_null())
        .ok_or_else(|| ApiError::BadRequest("transaction not found or not finalized yet".to_string()))?;

    result
        .get("transaction")
        .and_then(|tx| tx.get(0))
        .and_then(|raw| raw.as_str())
        .map(ToString::to_string)
        .ok_or_else(|| ApiError::InternalError("RPC response missing base64 transaction".to_string()))
}
