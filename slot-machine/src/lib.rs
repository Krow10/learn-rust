pub mod par_table;
pub mod protocol;
pub mod utils;
pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub const SOCKET_PATH: &str = "/tmp/slot_machine.sock";
pub const MAX_BYTES_READ: u64 = 4096;
pub const GAMES_FOLDER: &str = "./data/games/";
