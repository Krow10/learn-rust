//! Socket messages exchanged between the client and the server.
//!
//! Leveraging `serde`, the messages are serialized to JSON before being sent over the socket connection.

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

/// The client commands that can be sent to the server.
#[derive(Debug, Serialize, Deserialize)]
pub enum ClientCommand {
    /// Sent at the start of the game to retrieve the balance and maximum bet allowed for the game.
    Init {
        /// Game string identifier (subfolder name in `GAMES_FOLDER`).
        game: String,
    },
    /// Sent to ask the server for a random spin result. The bet information is needed to compute
    /// the eventual payout.
    Play {
        /// Game string identifier (subfolder name in `GAMES_FOLDER`).
        game: String,
        /// Bet size. It corresponds to the index of the payouts vector stored in the par table.
        /// So the interface actually displays this value + 1.
        bet: usize,
    },
    /// Sent to retrieve the status of the server. The `clock` is used by the server to compute the
    /// latency of the client.
    Status {
        /// Timestamp of the client system time at the time of the request.
        clock: SystemTime,
    },
}

/// The states of the client / server connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerStatus {
    /// Server is not running.
    Stopped,
    /// Server is running but the client is not connected.
    Disconnected,
    /// Client is connected to the server.
    Connected,
}

/// Information about the client connection from the server point-of-view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    /// Connection status.
    pub server_status: ServerStatus,
    /// Uptime of the client connection.
    pub uptime: Duration,
    /// Latency of the client connection.
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

/// The server responses that will be sent to the client. 
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerResponse {
    /// In response to the client starting a new game.
    Init {
        /// Client balance. It is shared across all games that the client plays. Resets on every connection.
        balance: u64,
        /// Maximum bet that the requested game allows.
        max_bet: u64,
    },
    /// In response to the client requesting a spin. 
    Spin {
        /// The amount won on the spin.
        win: u64,
        /// The new balance adjusted for the cost of the bet and win amount.
        balance: u64,
        /// The spin result as a vector of reels position. Hence, the size of the vector is equal
        /// to the number of reels of the game.
        result: Vec<usize>,
    },
    /// Sent when an invalid request is received or when a request could not be fulfilled.
    Error {
        /// Error code identifier.
        code: u64,
        /// Error message.
        message: String,
    },
    /// In response to the client requesting the server status.
    Status(Status),
}
