use std::sync::Arc;

use helius::{Helius, types::Cluster};

use crate::config::Config;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub helius: Arc<Helius>,
    pub config: Arc<Config>,
}
impl AppState {
    pub fn new(config: Config) -> Self {
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
        }
    }
}
