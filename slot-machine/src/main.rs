use ndarray::prelude::*;
use rand::Rng;
use std::{
    collections::HashMap,
    fmt::{self, Display},
    io,
};

const BLANK: &'static str = "✕✕✕";
const BAR: &'static str = "𝐵𝐵𝐵";
const BAR5: &'static str = "𝑩𝑩𝑩";
const BAR7: &'static str = "🅱🅱🅱";
const JACKPOT: &'static str = "𝔍❤𝔍";

struct Paytable {
    table: HashMap<u64, u64>,
}

impl Paytable {
    fn get_table_entry(&self, spin: u64) -> Option<(u64, u64)> {
        let mut hash_vec: Vec<(&u64, &u64)> = self.table.iter().collect();
        // Order of bitmask is important so need to sort first (i.e. to check for triples of the same symbol before the any symbol payout)
        hash_vec.sort_by(|a, b| b.1.cmp(&a.1));
        hash_vec.iter().find_map(|(combo, pay)| {
            if (*combo & spin) == spin {
                Some((**combo, **pay))
            } else {
                None
            }
        })
    }

    fn get_combo(&self, spin: u64) -> Option<u64> {
        self.get_table_entry(spin).map(|(combo, _)| combo)
    }

    fn calculate_win(&self, spin: u64) -> Option<u64> {
        self.get_table_entry(spin).map(|(_, win)| win)
    }
}

impl Display for Paytable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:^32}{:<3}", "Combos", "Pay").expect("Cannot format Paytable");
        writeln!(f, "{:->32}{:1}{:-<3}", "", "", "").expect("Cannot format Paytable");

        self.table.iter().for_each(|(combo, pay)| {
            writeln!(f, "{:0>32b} {:>3}", combo, pay).expect("Cannot format Paytable");
        });
        Ok(())
    }
}

fn create_combo(symbols: Vec<u64>) -> u64 {
    symbols
        .iter()
        .enumerate()
        .fold(0u64, |acc, (i, s)| (s << i * 8) | acc)
}

fn clear_screen() {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char); // Clear screen control sequence
}

fn get_user_input() -> Option<String> {
    let mut user_input = String::new();

    io::stdin()
        .read_line(&mut user_input)
        .expect("Failed to read user input");

    Some(user_input)
}

#[allow(dead_code)]
fn format_binary(n: u64) -> String {
    format!(
        "{:0>8b} {:0>8b} {:0>8b} {:0>8b}",
        (n & 4278190080) >> 24,
        (n & 16711680) >> 16,
        (n & 65280) >> 8,
        (n & 255)
    )
}

fn main() {
    // All values taken from the example here: https://easy.vegas/games/slots/par_sheets/generic-1987.gif
    let symbols = [BLANK, BAR, BAR5, BAR7, JACKPOT].to_vec();
    let symbol_mapping: HashMap<&str, u64> = HashMap::<_, _>::from_iter(
        symbols
            .iter()
            .enumerate()
            .map(|(i, symbol)| (*symbol, 1 << i as u64)),
    );

    let virtual_reels = [
        [BLANK, BLANK, BLANK],
        [BLANK, BLANK, BLANK],
        [BAR, BAR, BAR],
        [BAR, BLANK, BLANK],
        [BLANK, BLANK, BLANK],
        [BAR5, BAR5, BAR5],
        [BLANK, BAR5, BLANK],
        [BAR, BLANK, BLANK],
        [BLANK, BLANK, BAR],
        [BLANK, BAR, BLANK],
        [BAR7, BLANK, BLANK],
        [BLANK, BLANK, BAR7],
        [BLANK, BAR7, BLANK],
        [BAR, BLANK, BLANK],
        [BAR, BLANK, BAR],
        [BLANK, BAR, BLANK],
        [BLANK, BLANK, BLANK],
        [BAR5, BAR5, BAR5],
        [BLANK, BLANK, BLANK],
        [BAR, BLANK, BLANK],
        [BAR, BAR, BAR],
        [BLANK, BAR, BLANK],
        [BAR5, BLANK, BAR],
        [BLANK, BAR, BLANK],
        [BLANK, BLANK, BLANK],
        [JACKPOT, BLANK, BAR5],
        [BLANK, BAR5, BLANK],
        [BLANK, BLANK, BLANK],
        [BAR, BAR, BAR],
        [BAR, BLANK, BLANK],
        [BLANK, BLANK, BLANK],
        [BAR5, JACKPOT, JACKPOT],
    ];

    /*
        Bit masks
        ---------
        BLANK   = 0b00001
        BAR     = 0b00010
        BAR5    = 0b00100
        BAR7    = 0b01000
        JACKPOT = 0b10000

        JACKPOT | BLANK = 0b10001
        AB = JACKPOT | BAR | BAR5 | BAR7 = !BLANK = 0b11110

        (REEL 1) (REEL 2) (REEL 3)
        00000000 00000000 00000000

        JACKPOT  ANYOTHER ANYOTHER
        00010000 00001111 00001111 -> 2

        ANYOTHER JACKPOT  ANYOTHER
        00001111 00010000 00001111 -> 2

        ANYOTHER ANYOTHER JACKPOT
        00001111 00001111 00010000 -> 2

        JACKPOT  JACKPOT  ANYOTHER
        00010000 00010000 00001111 -> 5

        JACKPOT  ANYOTHER JACKPOT
        00010000 00001111 00010000 -> 5

        ANYOTHER JACKPOT  JACKPOT
        00001111 00010000 00010000 -> 5

        AB       AB       AB
        00011110 00011110 00011110 -> 5

        BAR | JW BAR | JW BAR | JW
        00010010 00010010 00010010 -> 10

        BR5 | JW BR5 | JW BR5 | JW
        00010100 00010100 00010100 -> 50

        BR7 | JW BR7 | JW BR7 | JW
        00011000 00011000 00011000 -> 200

        JACKPOT  JACKPOT  JACKPOT
        00010000 00010000 00010000 -> 400
    */

    let anyother = *symbol_mapping.get(BLANK).unwrap()
        | *symbol_mapping.get(BAR).unwrap()
        | *symbol_mapping.get(BAR5).unwrap()
        | *symbol_mapping.get(BAR7).unwrap();

    let c1: u64 = create_combo(vec![
        *symbol_mapping.get(JACKPOT).unwrap(),
        anyother,
        anyother,
    ]);
    let c2: u64 = create_combo(vec![
        anyother,
        *symbol_mapping.get(JACKPOT).unwrap(),
        anyother,
    ]);
    let c3: u64 = create_combo(vec![
        anyother,
        anyother,
        *symbol_mapping.get(JACKPOT).unwrap(),
    ]);
    let c4: u64 = create_combo(vec![
        *symbol_mapping.get(JACKPOT).unwrap(),
        *symbol_mapping.get(JACKPOT).unwrap(),
        anyother,
    ]);
    let c5: u64 = create_combo(vec![
        *symbol_mapping.get(JACKPOT).unwrap(),
        anyother,
        *symbol_mapping.get(JACKPOT).unwrap(),
    ]);
    let c6: u64 = create_combo(vec![
        anyother,
        *symbol_mapping.get(JACKPOT).unwrap(),
        *symbol_mapping.get(JACKPOT).unwrap(),
    ]);
    let c7: u64 = create_combo(vec![
        *symbol_mapping.get(BAR).unwrap()
            | *symbol_mapping.get(BAR5).unwrap()
            | *symbol_mapping.get(BAR7).unwrap()
            | *symbol_mapping.get(JACKPOT).unwrap();
        3
    ]);
    let c8: u64 = create_combo(vec![
        *symbol_mapping.get(BAR).unwrap()
            | *symbol_mapping.get(JACKPOT).unwrap();
        3
    ]);
    let c9: u64 = create_combo(vec![
        *symbol_mapping.get(BAR5).unwrap()
            | *symbol_mapping.get(JACKPOT).unwrap();
        3
    ]);
    let c10: u64 = create_combo(vec![
        *symbol_mapping.get(BAR7).unwrap()
            | *symbol_mapping.get(JACKPOT).unwrap();
        3
    ]);
    let c11: u64 = create_combo(vec![*symbol_mapping.get(JACKPOT).unwrap(); 3]);

    let paytable = Paytable {
        table: HashMap::<u64, u64>::from([
            (c1, 2),
            (c2, 2),
            (c3, 2),
            (c4, 5),
            (c5, 5),
            (c6, 5),
            (c7, 5),
            (c8, 10),
            (c9, 50),
            (c10, 200),
            (c11, 400),
        ]),
    };

    // This bit is mostly to practice manipulating data with the `ndarray` API as it will be more useful later on
    let expanded_reels = Array2::from_shape_vec(
        (32, 3),
        virtual_reels
            .to_vec()
            .iter()
            .map(|rows| rows.map(|symbol| *symbol_mapping.get(symbol).unwrap() as usize))
            .flatten()
            .collect::<Vec<usize>>(),
    )
    .unwrap()
    .reversed_axes();

    let mut balance = 100u64;

    const N_SIMULATIONS: u64 = 10u64.pow(6);
    let mut simulated_payout_ratio = 0.0f64;
    let mut draws = HashMap::<Vec<usize>, u64>::new();

    println!(
        "{:-<5} Starting {} spin simulations {:-<5}",
        "", N_SIMULATIONS, ""
    );
    for _i in 1..N_SIMULATIONS {
        let rng_iter = rand::thread_rng().sample_iter(rand::distributions::Uniform::from(
            0..=virtual_reels.len() - 1, // Account for indexes, start at 0
        ));
        let rng_result: Vec<usize> = rng_iter.take(3).collect(); // In real-life applications the numbers are being constantly re-generated and picked just on input
        let spin_result: Vec<usize> = rng_result
            .iter()
            .enumerate()
            .map(|(reel, rng)| expanded_reels[(reel, *rng)])
            .collect();
        let win = paytable
            .calculate_win(create_combo(
                spin_result.iter().map(|x| *x as u64).collect(),
            ))
            .unwrap_or(0);

        simulated_payout_ratio += win as f64;
        draws.entry(spin_result.clone()).or_insert(0);
        draws.entry(spin_result).and_modify(|e| *e += 1);
        //print!("Progress: {}/{}\r", i, N_SIMULATIONS);
    }

    clear_screen();
    println!(
        "{:-<5} {} spin simulations finished {:-<5}",
        "", N_SIMULATIONS, ""
    );
    println!("Target payout ratio: 0.7608");
    println!(
        "Final simulated payout ratio: {}\n",
        simulated_payout_ratio / N_SIMULATIONS as f64
    );

    println!(
        "{:<9} {:<12} {:<12} {:<12}",
        "Combo", "Observed", "Expected", "Difference"
    );
    println!(
        "{:-<5}{:5}{:-<8}{:5}{:-<8}{:5}{:-<10}",
        "", "", "", "", "", "", ""
    );

    let mut grouped_count = HashMap::<&str, f64>::from([
        ("JW XX XX", 0.0),
        ("XX JW XX", 0.0),
        ("XX XX JW", 0.0),
        ("JW JW XX", 0.0),
        ("JW XX JW", 0.0),
        ("XX JW JW", 0.0),
        ("AB AB AB", 0.0),
        ("1J 1J 1J", 0.0),
        ("5J 5J 5J", 0.0),
        ("7J 7J 7J", 0.0),
        ("JW JW JW", 0.0),
    ]);
    draws
        .iter()
        .map(|(draw, count)| {
            (
                paytable
                    .get_combo(create_combo(
                        draw.iter()
                            .map(|v| {
                                symbol_mapping
                                    .iter()
                                    .find(|(_, value)| **value == (*v as u64))
                                    .map_or(0, |(_, value)| *value)
                            })
                            .collect(),
                    ))
                    .map_or("?? ?? ??", |c| match c {
                        x if x == c1 => "JW XX XX",
                        x if x == c2 => "XX JW XX",
                        x if x == c3 => "XX XX JW",
                        x if x == c4 => "JW JW XX",
                        x if x == c5 => "JW XX JW",
                        x if x == c6 => "XX JW JW",
                        x if x == c7 => "AB AB AB",
                        x if x == c8 => "1J 1J 1J",
                        x if x == c9 => "5J 5J 5J",
                        x if x == c10 => "7J 7J 7J",
                        x if x == c11 => "JW JW JW",
                        _ => "?? ?? ??",
                    }),
                *count as f64 / N_SIMULATIONS as f64,
            )
        })
        .for_each(|(s, c)| {
            grouped_count.entry(s).and_modify(|v| *v += c);
        });

    let expected_prob: HashMap<&str, f64> = HashMap::from([
        ("JW XX XX", 0.025665283203125),
        ("XX JW XX", 0.025054931640625),
        ("XX XX JW", 0.024200439453125),
        ("JW JW XX", 0.000640869140625),
        ("JW XX JW", 0.000579833984375),
        ("XX JW JW", 0.000518798828125),
        ("AB AB AB", 0.045135498046875),
        ("1J 1J 1J", 0.017059326171875),
        ("5J 5J 5J", 0.003021240234375),
        ("7J 7J 7J", 0.000213623046875),
        ("JW JW JW", 0.000030517578125),
    ]);
    grouped_count.iter().for_each(|(s, v)| {
        println!(
            "{:<9} {:<10.10} {:<10.10} {:+<10.10}",
            s,
            v,
            expected_prob[s],
            v - expected_prob[s]
        )
    });

    let sums: Vec<f64> = [expected_prob, grouped_count]
        .iter()
        .map(|h| h.values().sum())
        .collect();

    println!("{:-<51}", "");
    println!("{:<9} {:<10.10} {:<10.10}", "Sum", sums[0], sums[1]);

    println!("\n{}", paytable);

    println!("\nEnter any key to start the game...\n");
    get_user_input();
    clear_screen();

    loop {
        let rng_iter = rand::thread_rng().sample_iter(rand::distributions::Uniform::from(
            0..=virtual_reels.len() - 1, // Account for indexes, start at 0
        ));
        let rng_result: Vec<usize> = rng_iter.take(3).collect(); // In real-life applications the numbers are being constantly re-generated and picked just on input
        let spin_result: Vec<usize> = rng_result
            .iter()
            .enumerate()
            .map(|(reel, rng)| expanded_reels[(reel, *rng)])
            .collect();
        let win = paytable
            .calculate_win(create_combo(
                spin_result.iter().map(|x| *x as u64).collect(),
            ))
            .unwrap_or(0);

        if balance > 0 {
            println!("Your balance is : {:?} credits", balance);
            println!("Enter any input to start a spin! (1 credit)");

            get_user_input();
            clear_screen();
        } else {
            println!("Balance is empty :(");
            break;
        }

        println!(
            "{} | {} | {}",
            virtual_reels[rng_result[0]][0],
            virtual_reels[rng_result[1]][1],
            virtual_reels[rng_result[2]][2]
        );
        println!("Win: {}\n", win);

        balance = balance - 1 + win;
    }
}
