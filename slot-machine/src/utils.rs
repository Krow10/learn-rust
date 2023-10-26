pub fn format_binary(n: u64) -> String {
    format!(
        "{:0>8b} {:0>8b} {:0>8b} {:0>8b}",
        (n >> 24) & 255,
        (n >> 16) & 255,
        (n >> 8) & 255,
        n & 255
    )
}
