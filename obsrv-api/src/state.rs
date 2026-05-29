use crate::{config::Config, ws::WsEvent};
use helius::{Helius, types::Cluster};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

#[derive(Clone)]
pub struct AppState {
    pub helius: Arc<Helius>,
    pub config: Arc<Config>,
    pub db: PgPool,
    pub ws_tx: broadcast::Sender<WsEvent>,
    pub reload_tx: mpsc::Sender<()>,
}

impl AppState {
    pub fn new(
        config: Config,
        db: PgPool,
        ws_tx: broadcast::Sender<WsEvent>,
        reload_tx: mpsc::Sender<()>,
    ) -> Self {
        let cluster = if config.helius_rpc_url.contains("devnet") {
            Cluster::Devnet
        } else {
            Cluster::MainnetBeta
        };

        let helius =
            Helius::new(&config.helius_api_key, cluster).expect("failed to create Helius client");

        AppState {
            helius: Arc::new(helius),
            config: Arc::new(config),
            db,
            ws_tx,
            reload_tx,
        }
    }
}
