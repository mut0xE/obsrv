#[derive(Debug, Clone)]
pub struct Config {
    pub helius_api_key: String,
    pub helius_rpc_url: String,
    pub port: u16,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            helius_api_key: std::env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY must be set"),
            helius_rpc_url: std::env::var("HELIUS_RPC_URL")
                .unwrap_or("https://devnet.helius-rpc.com/".to_string()),
            port: std::env::var("PORT")
                .unwrap_or("3001".to_string())
                .parse()
                .unwrap_or(3001),
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        }
    }

    #[allow(dead_code)]
    pub fn rpc_url(&self) -> String {
        format!("{}/?api-key={}", self.helius_rpc_url, self.helius_api_key)
    }
}
