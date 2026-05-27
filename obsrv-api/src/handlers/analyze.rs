use axum::Json;
use obsrv_core::{
    analyzer,
    decoder::decode_payload,
    types::{AnalyzeResponse, AnalyzeTxRequest},
};

use crate::errors::ApiError;

pub async fn handle(Json(req): Json<AnalyzeTxRequest>) -> Result<Json<AnalyzeResponse>, ApiError> {
    tracing::info!(input_len = req.raw_tx.len(), "POST /analyze");

    // step 1: decode raw bytes
    let payload = decode_payload(&req.raw_tx).map_err(ApiError::from)?;
    // step 2: run full analysis pipeline
    let report = analyzer::analyze(&payload).map_err(ApiError::from)?;

    tracing::info!(
        risk_score    = report.risk_score,
        recommendation = %report.recommendation,
        is_durable_nonce = report.is_durable_nonce,
        "analysis complete"
    );
    Ok(Json(AnalyzeResponse { report }))
}
