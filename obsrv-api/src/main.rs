use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::{
    config::Config,
    handlers::{analyze, forensics, health},
    state::AppState,
};

mod config;
mod errors;
mod handlers;
mod state;
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // init logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_env();
    let state = AppState::new(config);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/health", get(health::handle))
        .route("/analyze", post(analyze::handle))
        .route("/forensics", post(forensics::handle))
        .with_state(state)
        .layer(cors);

    let addr = "0.0.0.0:3001";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    tracing::info!("obsrv-api listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
