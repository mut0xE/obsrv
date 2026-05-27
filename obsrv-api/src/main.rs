use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};

use crate::handlers::{analyze, health};

mod errors;
mod handlers;
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // init logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/health", get(handlers::health::handle))
        .route("/analyze", post(handlers::analyze::handle))
        .layer(cors);

    let addr = "0.0.0.0:3001";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    tracing::info!("obsrv-api listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
