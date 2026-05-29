use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    TxProcessed {
        signature: String,
        wallet: String,
        risk: u8,
        summary: String,
        programs: Vec<String>,
    },
    Alert {
        wallet: String,
        risk: u8,
        signature: String,
        summary: String,
    },
    SpikeDetected {
        program_id: String,
        instruction_type: String,
        magnitude: f64,
        today_calls: i64,
        baseline_avg: f64,
    },
}
