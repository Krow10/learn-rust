#![warn(missing_docs)]
#![doc = include_str!("../docs/slot_machine.md")]

pub mod par_table;
pub mod protocol;
pub mod utils;
/// Generated build information made available in the code by the `built` crate.
pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Client / server UNIX socket file path.
pub const SOCKET_PATH: &str = "/tmp/slot_machine.sock";
/// Maximum amount of bytes for a single socket read for both client and server.
pub const MAX_BYTES_READ: u64 = 4096;
/// Games folder path.
pub const GAMES_FOLDER: &str = "./data/games/";
