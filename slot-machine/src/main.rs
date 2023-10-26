use itertools::Itertools;
use rand::Rng;
use std::time::Instant;
use std::{collections::HashMap, io};

mod par_table;
mod utils;
use par_table::ParTable;

fn clear_screen() {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char); // Clear screen control sequence
}

fn get_user_input() -> Option<String> {
    let mut user_input = String::new();

    io::stdin()
        .read_line(&mut user_input)
        .expect("Failed to read user input");

    Some(user_input.trim().to_owned())
}

const BLANK: &'static str = "⬜⬜⬜";
const BAR: &'static str = "♉♉♉";
const BAR5: &'static str = "♓♓♓";
const BAR7: &'static str = "♍♍♍";
const JACKPOT: &'static str = "💝💝💝";

fn run_simulation(files: [&str; 4], n_simulations: u64) {
    const EXPECTED_PAYOUTS_RATIO: [f64; 2] = [0.76080, 0.85495];
    const EXPECTED_HIT_RATIO: f64 = 0.14212;

    let mut table = ParTable::default();
    table
        .parse_from_csv(files[0..3].try_into().unwrap())
        .expect("Could not parse par table from csv files");

    let mut simulated_payout_ratio = 0.0f64;
    let mut simulated_hit_ratio = 0.0f64;
    let mut draws = HashMap::<String, f64>::new();

    clear_screen();
    println!("[*] Starting {} spin simulations", n_simulations);

    let n_coins = 2;
    let non_winning_combo = String::from("--");

    let now = Instant::now();
    for _i in 1..n_simulations {
        let rng_iter = rand::thread_rng().sample_iter(rand::distributions::Uniform::from(
            0..=table.reels.len() - 1, // Account for indexes, start at 0
        ));
        let rng_result: Vec<usize> = rng_iter.take(3).collect(); // In real-life applications the numbers are being constantly re-generated and picked just on input
        let spin_result: Vec<u64> = rng_result
            .iter()
            .enumerate()
            .map(|(reel, rng)| table.reels[*rng][reel])
            .collect();
        let (winning_combo, win_amount) = table
            .calculate_win(spin_result.clone(), n_coins - 1)
            .unwrap_or((vec![0, 0, 0], 0));

        simulated_payout_ratio += win_amount as f64;

        if winning_combo.cmp(&vec![0, 0, 0]).is_ne() {
            simulated_hit_ratio += 1.0
        }

        let combo_string = winning_combo
            .iter()
            .map(|x| {
                table
                    .symbol_num_mapping
                    .get(x)
                    .unwrap_or(&non_winning_combo)
            })
            .join(" ");
        draws.entry(combo_string.clone()).or_insert(0.0);
        draws.entry(combo_string.clone()).and_modify(|e| *e += 1.0);
        //print!("Progress: {}/{}\r", i, n_simulations);
    }

    draws.values_mut().for_each(|v| *v /= n_simulations as f64);

    println!(
        "[+] {} spin simulations finished ({:.2?} s)",
        n_simulations,
        now.elapsed().as_secs()
    );

    println!(
        "{:<9} {:<12} {:<12} {:<12}",
        "Ratio", "Observed", "Expected", "Difference"
    );
    println!(
        "{:-<5}{:5}{:-<8}{:5}{:-<8}{:5}{:-<10}",
        "", "", "", "", "", "", ""
    );

    println!(
        "{:<9} {:<10.10} {:<10.10} {:<+10.10}",
        "Hit",
        simulated_hit_ratio / n_simulations as f64,
        EXPECTED_HIT_RATIO,
        simulated_hit_ratio / n_simulations as f64 - EXPECTED_HIT_RATIO
    );

    println!(
        "{:<9} {:<10.10} {:<10.10} {:<+10.10}",
        "Payout",
        simulated_payout_ratio / (n_simulations * n_coins as u64) as f64,
        EXPECTED_PAYOUTS_RATIO[n_coins - 1],
        simulated_payout_ratio / (n_simulations * n_coins as u64) as f64
            - EXPECTED_PAYOUTS_RATIO[n_coins - 1]
    );

    println!("");

    println!(
        "{:<9} {:<12} {:<12} {:<12}",
        "Combo", "Observed", "Expected", "Difference"
    );
    println!(
        "{:-<5}{:5}{:-<8}{:5}{:-<8}{:5}{:-<10}",
        "", "", "", "", "", "", ""
    );

    let expected_prob: Vec<(String, f64)> = vec![
        ("JW XX XX".to_string(), 0.025665283203125),
        ("XX JW XX".to_string(), 0.025054931640625),
        ("XX XX JW".to_string(), 0.024200439453125),
        ("JW JW XX".to_string(), 0.000640869140625),
        ("JW XX JW".to_string(), 0.000579833984375),
        ("XX JW JW".to_string(), 0.000518798828125),
        ("AB AB AB".to_string(), 0.045135498046875),
        ("1J 1J 1J".to_string(), 0.017059326171875),
        ("5J 5J 5J".to_string(), 0.003021240234375),
        ("7J 7J 7J".to_string(), 0.000213623046875),
        ("JW JW JW".to_string(), 0.000030517578125),
    ];

    expected_prob.iter().for_each(|(s, v)| {
        let draw_entry_value = draws.get(s).unwrap_or(&0.0);
        println!(
            "{:<9} {:<10.10} {:<10.10} {:<+10.10}",
            s,
            draw_entry_value,
            v,
            draw_entry_value - v
        );
    });
}

fn main() {
    let mut generic_par_table = ParTable::default();
    generic_par_table
        .parse_from_csv([
            "data/generic/symbols.csv",
            "data/generic/paytable.csv",
            "data/generic/reels.csv",
        ])
        .expect("Could not parse par table from csv files");

    let mut balance = 100u64;

    /*    run_simulation(
        [
            "data/generic/symbols.csv",
            "data/generic/paytable.csv",
            "data/generic/reels.csv",
            "",
        ],
        10u64.pow(5),
    );*/

    println!("\nEnter any key to start the game...\n");
    get_user_input();
    clear_screen();

    let mut sorted_keys: Vec<u64> = generic_par_table
        .symbol_num_mapping
        .keys()
        .copied()
        .collect();
    sorted_keys.sort();
    let display_symbols: HashMap<&u64, &&str> = sorted_keys
        .iter()
        .take(5)
        .zip([BLANK, BAR, BAR5, BAR7, JACKPOT].iter())
        .collect();

    loop {
        let rng_iter = rand::thread_rng().sample_iter(rand::distributions::Uniform::from(
            0..=generic_par_table.reels.len() - 1, // Account for indexes, start at 0
        ));
        let rng_result: Vec<usize> = rng_iter.take(3).collect(); // In real-life applications the numbers are being constantly re-generated and picked just on input
        let spin_result: Vec<u64> = rng_result
            .iter()
            .enumerate()
            .map(|(reel, rng)| generic_par_table.reels[*rng][reel])
            .collect();
        let (_, win) = generic_par_table
            .calculate_win(spin_result.clone(), 0)
            .unwrap_or((spin_result, 0));

        if balance > 0 {
            println!("Your balance is : {:?} credits", balance);
            println!("Enter any input to start a spin (1 credit) or enter 'r' to see the rules!");

            match get_user_input().as_deref() {
                Some("r") => {
                    println!(
                        "
🆎 = ♉ or ♓ or ♍ or 💝
❎ = All except 💝
🅰 = ♉ or 💝
🅱 = ♓ or 💝
🅾 = ♍ or 💝

Combo     Pay
--------  ---
💝 ❎ ❎  2
❎ 💝 ❎  2
❎ ❎ 💝  2
💝 💝 ❎  5
💝 ❎ 💝  5
❎ 💝 💝  5
🆎 🆎 🆎  5
🅰 🅰 🅰  10
🅱 🅱 🅱  50
🅾 🅾 🅾  200
💝 💝 💝  400
                    "
                    );
                    println!("\nEnter any key to resume...");
                    get_user_input();
                }
                _ => (),
            }

            clear_screen();
        } else {
            println!("Balance is empty :(");
            break;
        }

        println!("{:=<25}", "");
        println!(
            "{} | {} | {}",
            display_symbols
                .get(&generic_par_table.reels[rng_result[0]][0])
                .unwrap(),
            display_symbols
                .get(&generic_par_table.reels[rng_result[1]][1])
                .unwrap(),
            display_symbols
                .get(&generic_par_table.reels[rng_result[2]][2])
                .unwrap(),
        );
        println!("{:=<25}", "");
        println!("Win: {}\n", win);

        balance = balance - 1 + win;
    }
}
