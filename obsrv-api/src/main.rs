use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::{
    config::Config,
    handlers::{analyze, forensics, health, nonce_inspect, simulate},
    state::AppState,
};

mod config;
mod errors;
mod handlers;
pub mod response;
mod state;
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // init logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_env();
    let addr = format!("0.0.0.0:{}", config.port);
    let state = AppState::new(config);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/health", get(health::handle))
        .route("/analyze", post(analyze::handle))
        .route("/forensics", post(forensics::handle))
        .route("/simulate", post(simulate::handle))
        .route("/nonce/inspect", post(nonce_inspect::handle))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("obsrv-api listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
