//! Execute simulations for ensuring that the slot game implementations are accurate.
//!
//! The objective is to make sure that the observed hit percentage of combos, and the resulting
//! return to player are as close as possible (within some error boundary) to the theoretical
//! values.
//!
//! It is however currently hard-coded to work only for the `generic` and `blaze7` (RTP-only) games
//! implemented in the [project's repo](https://github.com/Krow10/learn-rust/blob/main/slot-machine/data/games).
use itertools::Itertools;
use rand::Rng;
use std::collections::HashMap;
use std::time::Instant;

use slot_machine::par_table::{ParTable, ParTableFiles};

/// Starts the simulation for the given game files.
///
/// For each of the `n_simulations` loop it will draw a random spin and store the resulting combo as
/// well as update the hit ratio if the combo is winning.
///
/// The results are displayed directly to the console in the form of a table with variations from the
/// hard-coded theoretical values.
fn run_simulation(files: Vec<String>, n_simulations: u64) {
    const EXPECTED_PAYOUTS_RATIO: [f64; 3] = [0.76080, 0.85495, 0.9270];
    const EXPECTED_HIT_RATIO: f64 = 0.14212;

    let mut table = ParTable::default();
    table
        .parse_from_csv(ParTableFiles::try_from(files).unwrap())
        .expect("Could not parse par table from csv files");

    let mut simulated_payout_ratio = 0.0f64;
    let mut simulated_hit_ratio = 0.0f64;
    let mut draws = HashMap::<String, f64>::new();

    println!("[*] Starting {} spin simulations", n_simulations);

    let n_coins = 1;
    let non_winning_combo = String::from("--");

    let now = Instant::now();
    for _i in 1..n_simulations {
        let rng_iter = rand::thread_rng().sample_iter(rand::distributions::Uniform::from(
            0..=table.reels.len() - 1, // Account for indexes, start at 0
        ));
        // In real-life applications the numbers are being constantly re-generated and picked just on input
        let rng_result: Vec<usize> = rng_iter.take(3).collect();
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

    /*    let expected_prob: Vec<(String, f64)> = vec![
        ("B7 B7 DJ".to_string(), 0.0),
        ("B7 B7 B7".to_string(), 0.0),
        ("R7 R7 DJ".to_string(), 0.0),
        ("R7 R7 R7".to_string(), 0.0),
        ("A7 A7 DJ".to_string(), 0.0),
        ("A7 A7 A7".to_string(), 0.0),
        ("3B 3B DJ".to_string(), 0.0),
        ("3B 3B 3B".to_string(), 0.0),
        ("2B 2B DJ".to_string(), 0.0),
        ("2B 2B 2B".to_string(), 0.0),
        ("1B 1B DJ".to_string(), 0.0),
        ("1B 1B 1B".to_string(), 0.0),
        ("AB AB DJ".to_string(), 0.0),
        ("AB AB AB".to_string(), 0.0),
        ("BL BL DJ".to_string(), 0.0),
        ("BL BL BL".to_string(), 0.0)
    ];*/

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

/// Runs a million rools for the `generic` game.
fn main() {
    run_simulation(
        vec![
            "data/generic/symbols.csv".to_string(),
            "data/generic/paytable.csv".to_string(),
            "data/generic/reels.csv".to_string(),
            "".to_string(),
        ],
        10u64.pow(6),
    );
}
