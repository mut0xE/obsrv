/// Application configuration loaded from environment variables.
///
/// Required env vars:
///   RPC_URL        — Solana JSON-RPC endpoint (e.g. Helius, QuickNode, or local)
///   DATABASE_URL   — PostgreSQL connection string
///
/// Optional env vars:
///   GRPC_ENDPOINT  — Yellowstone gRPC endpoint (default: ParaFi mainnet)
///   GRPC_X_TOKEN   — auth token for gRPC endpoints that require one (ParaFi does not)
///   PORT           — HTTP listen port (default: 3001)
#[derive(Debug, Clone)]
pub struct Config {
    /// Solana JSON-RPC URL used by handlers (forensics, simulate, nonce inspect).
    pub rpc_url: String,

    /// Yellowstone gRPC endpoint for real-time transaction streaming.
    pub grpc_endpoint: String,

    /// Optional x-token for authenticated gRPC endpoints.
    /// ParaFi (solana-rpc.parafi.tech) does not require one.
    pub grpc_x_token: Option<String>,

    /// Telegram bot token for sending alerts (get from @BotFather).
    /// If not set, Telegram alerts are disabled (WebSocket alerts still work).
    pub telegram_bot_token: Option<String>,

    /// HTTP server listen port.
    pub port: u16,

    /// PostgreSQL connection string.
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let port: u16 = std::env::var("PORT")
            .unwrap_or("3001".to_string())
            .parse()
            .unwrap_or(3001);

        let grpc_x_token = std::env::var("GRPC_X_TOKEN").ok().filter(|s| !s.is_empty());
        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());

        Config {
            rpc_url: std::env::var("RPC_URL").expect("RPC_URL must be set"),
            grpc_endpoint: std::env::var("GRPC_ENDPOINT")
                .unwrap_or("https://solana-rpc.parafi.tech:10443".to_string()),
            grpc_x_token,
            telegram_bot_token,
            port,
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        }
    }
}
