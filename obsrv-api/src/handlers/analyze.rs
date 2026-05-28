use axum::{Json, extract::State};
use obsrv_core::{
    analyzer,
    decoder::decode_payload,
    types::{AnalyzeTxRequest, ApiResponse, TxResponse},
};

use crate::{
    errors::ApiError,
    response::{build_analysis, build_expected_balances, build_instructions, build_meta},
    state::AppState,
};

pub async fn handle(
    State(_state): State<AppState>,
    Json(req): Json<AnalyzeTxRequest>,
) -> Result<Json<ApiResponse>, ApiError> {
    tracing::info!(input_len = req.raw_tx.len(), "POST /analyze");

    let payload = decode_payload(&req.raw_tx).map_err(ApiError::from)?;
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
