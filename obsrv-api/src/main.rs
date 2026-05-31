use axum::{
    Router,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

mod auth;
mod config;
mod db;
mod errors;
mod handlers;
mod response;
mod state;
mod streams;
mod telegram_bot;
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

    let state = AppState::new(config, pool, ws_tx);

    // spawn Yellowstone gRPC stream as a background task
    tokio::spawn(streams::run_stream(
        state.config.clone(),
        state.db.clone(),
        state.ws_tx.clone(),
    ));

    // spawn Telegram bot if token is configured
    if let Some(ref token) = state.config.telegram_bot_token {
        let token = token.clone();
        let pool = state.db.clone();
        tokio::spawn(telegram_bot::run_bot(token, pool));
    }

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
        .route("/monitor/wallet", post(handlers::monitor::add_wallet))
        .route(
            "/monitor/wallet/remove",
            post(handlers::monitor::remove_wallet),
        )
        .route("/monitor/program", post(handlers::monitor::add_program))
        .route(
            "/monitor/program/remove",
            post(handlers::monitor::remove_program),
        )
        .route("/monitor/list", get(handlers::monitor::list))
        .route("/analytics/wallet", get(handlers::analytics::wallet))
        .route("/analytics/program", get(handlers::analytics::program))
        .route(
            "/analytics/programs",
            get(handlers::analytics::top_programs),
        )
        .route("/analytics/alerts", get(handlers::analytics::alerts))
        .route("/ws", get(handlers::ws::handle))
        // stream query API
        .route(
            "/stream/transactions",
            get(handlers::stream::get_transactions),
        )
        .route("/stream/tx", get(handlers::stream::get_transaction))
        .route("/stream/stats", get(handlers::stream::get_stats))
        .route(
            "/stream/wallet/history",
            get(handlers::stream::get_wallet_history),
        )
        .with_state(state.clone())
        .layer(cors);

    let addr = format!("0.0.0.0:{}", state.config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("obsrv-api listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
