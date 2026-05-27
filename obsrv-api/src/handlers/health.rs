use axum::Json;
use serde_json::{Value, json};

pub async fn handle() -> Json<Value> {
    Json(json!(
        {
        "status":  "ok",
        "service": "obsrv-api",
        "version": "0.1.0"
        }
    ))
}
