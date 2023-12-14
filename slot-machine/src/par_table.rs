//! Par table abstraction describing a slot game (reels, paytable, combos, etc.).
//!
//! For real slot games, a par table is what describes (almost) all the aspects of the slot game.
//! It includes a description a all the symbols to be used, the winning combos and their respective
//! payouts, as well as statistical evidence of the game return to player (RTP).
//!
//! ![Example of a par table](https://www-knowyourslots-com.exactdn.com/wp-content/uploads/2019/06/par-sheet-example-385x1024.jpg)
//!
//! *Example of a par table taken from [Know Your Slots](https://www.knowyourslots.com/the-par-sheet-a-look-under-the-hood-of-a-slot-machine-game/)*

use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display},
};

use crate::utils::format_binary;
use anyhow::Result;

type Symbol = u64;
type Combo = Vec<Symbol>;

/// Utilitary structure for referencing the files needed to load a par table's data.
pub struct ParTableFiles {
    reels_file: String,
    paytable_file: String,
    symbols_file: String,
}

impl TryFrom<Vec<String>> for ParTableFiles {
    type Error = ();
    fn try_from(_a: Vec<String>) -> Result<ParTableFiles, ()> {
        let mut ptf = ParTableFiles::default();
        let mut n_loaded = 0;

        for path in _a {
            if path.contains("reels") {
                ptf.reels_file = path.to_string();
                n_loaded += 1;
            } else if path.contains("paytable") {
                ptf.paytable_file = path.to_string();
                n_loaded += 1;
            } else if path.contains("symbols") {
                ptf.symbols_file = path.to_string();
                n_loaded += 1;
            }
        }

        if n_loaded < 3 {
            Err(())
        } else {
            Ok(ptf)
        }
    }
}

impl ParTableFiles {
    fn default() -> ParTableFiles {
        ParTableFiles {
            reels_file: "".to_string(),
            paytable_file: "".to_string(),
            symbols_file: "".to_string(),
        }
    }
}

/// Holds a game's information and can calculate the winnings given a set of random reel indexes.
///
/// It uses a bitmask representation of symbols in order to generate the required reference symbols and
/// check for combos using bitwise operations.
pub struct ParTable {
    /// Mapping of a symbol bitmask to its identifier.
    pub symbol_num_mapping: HashMap<Symbol, String>,
    /// Mapping of a symbol identifier to its bitmask.
    pub symbol_str_mapping: HashMap<String, Symbol>,
    /// Mapping of a symbol bitmask to its reference bitmask.
    pub combo_symbols: HashMap<Symbol, Symbol>,
    /// Mapping of a combo to its payouts.
    pub paytable: HashMap<Combo, Vec<u64>>,
    /// Reels of the game stored by rows. The number of rows of the game is given by the size of the
    /// elements of the vector.
    pub reels: Vec<Combo>,
    /// Maximum number of payouts for a single combo in the game.
    pub max_bet: u64,
}

impl ParTable {
    /// Initializes the par table with empty fields.
    pub fn default() -> ParTable {
        ParTable {
            symbol_num_mapping: HashMap::<Symbol, String>::new(),
            symbol_str_mapping: HashMap::<String, Symbol>::new(),
            combo_symbols: HashMap::<Symbol, Symbol>::new(),
            paytable: HashMap::<Combo, Vec<u64>>::new(),
            reels: vec![],
            max_bet: 1,
        }
    }

    fn combo_from_string(&self, s: String, delimiter: char) -> Result<Combo> {
        Ok(s.split(delimiter)
            .map(|k| {
                *self
                    .symbol_str_mapping
                    .get(k)
                    .ok_or(ParTableParseError::SymbolNotFoundError)
                    .unwrap()
            })
            .collect())
    }

    fn parse_symbols(&mut self, file: &str) -> Result<()> {
        const MAX_SYMBOLS: u32 = 64;
        let mut rdr = csv::Reader::from_path(file)?;
        let mut numeral_symbols = (0..=MAX_SYMBOLS - 1).map(|x| 2u64.pow(x));

        // Assume "display" symbol are described first in .csv followed by "mock" symbols for combos to parse everything in one loop
        for result in rdr.deserialize() {
            let (symbol, reference): (String, String) = result?;
            let key = numeral_symbols
                .next()
                .ok_or(ParTableParseError::TooMuchSymbolsError)?;

            self.symbol_num_mapping.insert(key, symbol.clone());
            self.symbol_str_mapping.insert(symbol.clone(), key);
            if reference.contains('|') {
                self.combo_symbols.insert(
                    key,
                    reference.split('|').fold(0u64, |acc, k| {
                        acc | *self
                            .symbol_str_mapping
                            .get(k)
                            .ok_or(ParTableParseError::SymbolNotFoundError)
                            .unwrap()
                    }),
                );
            } else if reference.contains('!') {
                self.combo_symbols.insert(
                    key,
                    self.symbol_str_mapping
                        .iter()
                        .filter(|(k, _)| symbol.cmp(k).is_ne())
                        .fold(0u64, |acc, (_, v)| acc | v),
                );
            } else {
                self.combo_symbols.insert(key, key);
            }
        }

        Ok(())
    }

    fn parse_paytable(&mut self, file: &str) -> Result<()> {
        let mut rdr = csv::Reader::from_path(file)?;

        // Assume `combo_symbols` is filled
        for result in rdr.deserialize() {
            let (combo, pays): (String, Vec<u64>) = result?;

            if pays.len() as u64 > self.max_bet {
                self.max_bet = pays.len() as u64;
            }

            self.paytable
                .insert(self.combo_from_string(combo, ' ').unwrap(), pays);
        }

        Ok(())
    }

    fn parse_reels(&mut self, file: &str) -> Result<()> {
        let mut rdr = csv::Reader::from_path(file)?;

        // Assume `combo_symbols` is filled
        for result in rdr.deserialize() {
            let row: Vec<String> = result?;

            self.reels
                .push(self.combo_from_string(row.join(" "), ' ').unwrap());
        }

        Ok(())
    }

    /// Loads a game from the required CSV files.
    pub fn parse_from_csv(&mut self, files: ParTableFiles) -> Result<()> {
        self.parse_symbols(files.symbols_file.as_str())?;
        self.parse_paytable(files.paytable_file.as_str())?;
        self.parse_reels(files.reels_file.as_str())
    }

    /// Tries to match the given spin result with a winning combo from the pay table and returns
    /// the corresponding payout amount (depending on the size of the bet). If it doesn't match,
    /// the spin is a loss.
    pub fn calculate_win(&self, spin: Combo, bet: usize) -> Option<(Combo, u64)> {
        let mut sorted_combos: Vec<&Combo> = self.paytable.keys().collect();
        sorted_combos.sort_by_key(|c| self.paytable.get(*c).unwrap());

        if let Some(win_combo) = sorted_combos.iter().rev().find(|c| {
            c.iter()
                .enumerate()
                .all(|(i, x)| self.combo_symbols.get(x).unwrap() & spin[i] == spin[i])
        }) {
            Some((
                win_combo.to_vec(),
                *self.paytable.get(&**win_combo).unwrap().get(bet).unwrap(),
            ))
        } else {
            None
        }
    }
}

impl Display for ParTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:<10} {:<3} {:^36}", "Identifier", "Code", "Symbol")?;
        writeln!(f, "{:-<10} {:-<3} {:-<36}", "", "", "")?;

        let mut sorted_symbols: Vec<(&Symbol, &Symbol)> = self.combo_symbols.iter().collect();
        sorted_symbols.sort();
        sorted_symbols.iter().for_each(|(symbol, combo)| {
            writeln!(
                f,
                "{:<10} {:<3} {:0>36}",
                symbol,
                self.symbol_num_mapping.get(symbol).unwrap(),
                format_binary(**combo)
            )
            .expect("Cannot format ParTable");
        });

        writeln!(f, "{:=<51}", "").unwrap();

        let mut sorted_paytable: Vec<(&Combo, &Vec<u64>)> = self.paytable.iter().collect();
        writeln!(f, "{:<18} {:<3}", "Combo", "Pays")?;
        write!(f, "{:-<18} {:-<3}", "", "")?;

        (1..=sorted_paytable[0].1.len()).for_each(|_| write!(f, "{:-<12}", "").unwrap());
        writeln!(f, "").unwrap();

        sorted_paytable.sort_by(|a, b| a.1.cmp(b.1).then(a.0.cmp(b.0)));
        sorted_paytable.iter().for_each(|(combo, pay)| {
            write!(
                f,
                "{:<18?}",
                combo
                    .iter()
                    .map(|c| self.symbol_num_mapping.get(c).unwrap().clone())
                    .collect::<Vec<String>>()
            )
            .expect("Cannot format ParTable");
            pay.iter()
                .for_each(|p| write!(f, " {:<12}", p).expect("Cannot format ParTable"));
            writeln!(f, "").unwrap();
        });

        writeln!(f, "{:=<51}", "").unwrap();

        (1..=self.reels[0].len()).for_each(|x| write!(f, "Reel {:<5} ", x).unwrap());
        writeln!(f, "").unwrap();
        (1..=self.reels[0].len()).for_each(|_| write!(f, "{:-<6}{:<5}", "", "").unwrap());
        writeln!(f, "").unwrap();

        self.reels.iter().for_each(|r| {
            r.iter().for_each(|n| {
                write!(f, "{:^6}{:<5}", self.symbol_num_mapping.get(n).unwrap(), "").unwrap();
            });
            writeln!(f, "").unwrap();
        });

        writeln!(f, "{:=<51}", "").unwrap();

        write!(f, "Symbol ").unwrap();
        (1..=self.reels[0].len()).for_each(|x| write!(f, "Count(Reel {:1}) ", x).unwrap());
        writeln!(f, "").unwrap();
        write!(f, "{:-<6} ", "").unwrap();
        (1..=self.reels[0].len()).for_each(|_| write!(f, "{:-<13}{:<1}", "", "").unwrap());
        writeln!(f, "").unwrap();

        let counts: HashMap<u64, Vec<u64>> =
            HashMap::from_iter(sorted_symbols.iter().map(|(symbol, _)| {
                let mut c = vec![0; self.reels[0].len()];

                self.reels[0].iter().enumerate().for_each(|(i, _)| {
                    c[i] = self
                        .reels
                        .iter()
                        .map(|r| r[i])
                        .filter(|x| *x == **symbol)
                        .fold(0u64, |acc, _| acc + 1);
                });

                (**symbol, c)
            }));

        sorted_symbols.iter().for_each(|(s, _)| {
            write!(f, "{:<7}", self.symbol_num_mapping.get(s).unwrap()).unwrap();
            self.reels[0].iter().enumerate().for_each(|(i, _)| {
                write!(f, "{:^13}{:<1}", counts.get(s).unwrap()[i], "").unwrap();
            });
            writeln!(f, "").unwrap();
        });

        Ok(())
    }
}

/// Parsing errors raised when loading the CSV files.
#[derive(Debug)]
pub enum ParTableParseError {
    /// Raised when the number of unique symbols in the `symbols.csv` is greater than 64.
    ///
    /// Storing the symbols as a bitmask requires than each bit of the `u64` represents a different
    /// symbol in order to be able to differentiate them when applying bitwise operations.
    TooMuchSymbolsError,
    /// Raised when a given identifier is not corresponding to any symbol in the `symbols_str_mapping`.
    SymbolNotFoundError,
}

impl Error for ParTableParseError {}

impl fmt::Display for ParTableParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParTableParseError::TooMuchSymbolsError => {
                write!(f, "Too much symbols in table (max 64)")
            }
            ParTableParseError::SymbolNotFoundError => write!(f, "Symbol not found for pattern"),
        }
    }
}
