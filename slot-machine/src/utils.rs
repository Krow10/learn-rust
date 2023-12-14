//! Utility functions used across the different binaries. 

use std::{io::Write, os::unix::net::UnixStream};

/// Returns the binary representation of a 64bit integer grouped by bytes.
pub fn format_binary(n: u64) -> String {
    format!(
        "{:0>8b} {:0>8b} {:0>8b} {:0>8b}",
        (n >> 24) & 255,
        (n >> 16) & 255,
        (n >> 8) & 255,
        n & 255
    )
}

/// Write a message to a socket stream and appending a newline character at the end.
/// The stream is also flushed after the write operation.
pub fn send_socket_message(stream: &mut UnixStream, message: String) {
    writeln!(stream, "{}", message).expect("Could not send message to server");
    stream.flush().expect("Could not flush");
}
