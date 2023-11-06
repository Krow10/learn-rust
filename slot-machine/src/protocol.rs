use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientCommand {
    Balance,
    Play { game: String, bet: usize },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerResponse {
    Balance {
        balance: u64,
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
}
