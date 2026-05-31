use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use obsrv_core::errors::ObsrvError;
use serde_json::json;
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    UnprocessableEntity(String),
    InternalError(String),
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::UnprocessableEntity(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        let body = Json(json!({
            "error":  message,
            "status": status.as_u16()
        }));
        (status, body).into_response()
    }
}

impl From<ObsrvError> for ApiError {
    fn from(e: ObsrvError) -> Self {
        match e {
            ObsrvError::EmptyInput
            | ObsrvError::InputTooLarge
            | ObsrvError::TransactionTooLarge
            | ObsrvError::Base64DecodeFailed(_)
            | ObsrvError::Base58DecodeFailed(_)
            | ObsrvError::DeserializationFailed(_)
            | ObsrvError::DecodeFailed => ApiError::BadRequest(e.to_string()),

            ObsrvError::NoInstructions | ObsrvError::NoAccounts | ObsrvError::AnalysisFailed(_) => {
                ApiError::UnprocessableEntity(e.to_string())
            }

            ObsrvError::RpcFailed(_)
            | ObsrvError::AccountNotFound(_)
            | ObsrvError::InvalidNonceAccount => ApiError::InternalError(e.to_string()),
        }
    }
}
