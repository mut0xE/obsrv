use crate::{config::Config, ws::WsEvent};
use solana_client::rpc_client::RpcClient;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Shared application state passed to all Axum handlers.
///
/// - `rpc`    — Solana JSON-RPC client for on-demand queries (forensics, simulate, nonce).
/// - `config` — Immutable app configuration (RPC URL, gRPC endpoint, etc.).
/// - `db`     — PostgreSQL connection pool.
/// - `ws_tx`  — Broadcast channel for pushing real-time events to WebSocket clients.
///
/// No `reload_tx` needed — the gRPC stream loop polls the DB every 30s for
/// address changes and resubscribes automatically. This is simpler and more
/// reliable than a reload signal channel since:
///   1. No coupling between HTTP handlers and the stream task
///   2. Works even if the stream reconnects after a disconnect
///   3. No risk of missed signals during reconnection windows
#[derive(Clone)]
pub struct AppState {
    pub rpc: Arc<RpcClient>,
    pub config: Arc<Config>,
    pub db: PgPool,
    pub ws_tx: broadcast::Sender<WsEvent>,
}

impl AppState {
    pub fn new(config: Config, db: PgPool, ws_tx: broadcast::Sender<WsEvent>) -> Self {
        let rpc = RpcClient::new(config.rpc_url.clone());

        AppState {
            rpc: Arc::new(rpc),
            config: Arc::new(config),
            db,
            ws_tx,
        }
    }
}
