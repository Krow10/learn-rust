use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientCommand {
    Init { game: String },
    Play { game: String, bet: usize },
    Status { clock: SystemTime },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerStatus {
    Stopped,
    Disconnected,
    Connected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub server_status: ServerStatus,
    pub uptime: Duration,
    pub latency: Duration,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            server_status: ServerStatus::Stopped,
            uptime: Duration::new(0, 0),
            latency: Duration::new(0, 0),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerResponse {
    Init {
        balance: u64,
        max_bet: u64,
    },
    Spin {
        win: u64,
        balance: u64,
        result: Vec<usize>,
    },
    Error {
        code: u64,
        message: String,
    },
    Status(Status),
}
