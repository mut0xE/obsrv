use obsrv_core::types::{StreamTxEvent, WalletAlert};
use serde::{Deserialize, Serialize};

/// Events broadcast to connected WebSocket clients in real time.
///
/// Clients receive these as JSON messages tagged by `type`:
///   - `tx_processed`       — a transaction matched a watched address
///   - `alert`              — a high-risk tx triggered a wallet alert
///   - `spike_detected`     — instruction call volume spike detected
///   - `stream_status`      — gRPC stream connected/disconnected
///   - `watch_list_updated` — watched addresses changed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    /// A transaction arrived for a watched wallet or program.
    TxProcessed(StreamTxEvent),

    /// A high-risk transaction triggered a wallet alert.
    Alert(WalletAlert),

    /// Instruction call volume spike detected for a watched program.
    SpikeDetected {
        program_id: String,
        instruction_type: String,
        magnitude: f64,
        today_calls: i64,
        baseline_avg: f64,
    },

    /// Stream connectivity status change.
    StreamStatus {
        connected: bool,
        endpoint: String,
        watched_count: usize,
    },

    /// Watched address list was modified.
    WatchListUpdated {
        wallets: Vec<String>,
        programs: Vec<String>,
    },
}
