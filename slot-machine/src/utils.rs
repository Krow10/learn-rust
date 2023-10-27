use std::io::{self};

pub fn format_binary(n: u64) -> String {
    format!(
        "{:0>8b} {:0>8b} {:0>8b} {:0>8b}",
        (n >> 24) & 255,
        (n >> 16) & 255,
        (n >> 8) & 255,
        n & 255
    )
}

pub fn clear_screen() {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char); // Clear screen control sequence
}

pub fn get_user_input() -> Option<String> {
    let mut user_input = String::new();

    io::stdin()
        .read_line(&mut user_input)
        .expect("Failed to read user input");

    Some(user_input.trim().to_owned())
}
