use axum::{
    Router,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::{broadcast, mpsc};
use tower_http::cors::{Any, CorsLayer};

mod config;
mod db;
mod errors;
mod handlers;
mod response;
mod state;
mod ws;

use state::AppState;
use ws::WsEvent;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt().with_env_filter("info").init();

    let config = config::Config::from_env();

    // connect to Postgres
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await
        .expect("failed to connect to database");

    // run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    tracing::info!("database ready");

    // WebSocket broadcast channel
    let (ws_tx, _) = broadcast::channel::<WsEvent>(256);

    // stream reload signal channel
    let (reload_tx, _reload_rx) = mpsc::channel::<()>(10);

    let state = AppState::new(config, pool, ws_tx, reload_tx);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(handlers::health::handle))
        .route("/analyze", post(handlers::analyze::handle))
        .route("/forensics", post(handlers::forensics::handle))
        .route("/simulate", post(handlers::simulate::handle))
        .route("/nonce/inspect", post(handlers::nonce_inspect::handle))
        .with_state(state)
        .layer(cors);

    let addr = "0.0.0.0:3001";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    tracing::info!("obsrv-api listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
