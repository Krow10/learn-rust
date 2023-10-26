use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display},
};

use crate::utils::format_binary;

type Symbol = u64;
type Combo = Vec<Symbol>;

pub struct ParTable {
    pub symbol_num_mapping: HashMap<Symbol, String>,
    pub symbol_str_mapping: HashMap<String, Symbol>,
    pub combo_symbols: HashMap<Symbol, Symbol>,
    pub paytable: HashMap<Combo, Vec<u64>>,
    pub reels: Vec<Combo>,
}

impl ParTable {
    pub fn default() -> ParTable {
        ParTable {
            symbol_num_mapping: HashMap::<Symbol, String>::new(),
            symbol_str_mapping: HashMap::<String, Symbol>::new(),
            combo_symbols: HashMap::<Symbol, Symbol>::new(),
            paytable: HashMap::<Combo, Vec<u64>>::new(),
            reels: vec![],
        }
    }

    fn combo_from_string(&self, s: String, delimiter: char) -> Result<Combo, Box<dyn Error>> {
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

    fn parse_symbols(&mut self, file: &str) -> Result<(), Box<dyn Error>> {
        const MAX_SYMBOLS: u32 = 256;
        let mut rdr = csv::Reader::from_path(file)?;
        let mut numeral_symbols = (0..=(MAX_SYMBOLS / 8) - 1).map(|x| 2u64.pow(x));

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

    fn parse_paytable(&mut self, file: &str) -> Result<(), Box<dyn Error>> {
        let mut rdr = csv::Reader::from_path(file)?;

        // Assume `combo_symbols` is filled
        for result in rdr.deserialize() {
            let (combo, pays): (String, Vec<u64>) = result?;
            self.paytable
                .insert(self.combo_from_string(combo, ' ').unwrap(), pays);
        }

        Ok(())
    }

    fn parse_reels(&mut self, file: &str) -> Result<(), Box<dyn Error>> {
        let mut rdr = csv::Reader::from_path(file)?;

        // Assume `combo_symbols` is filled
        for result in rdr.deserialize() {
            let row: Vec<String> = result?;

            self.reels
                .push(self.combo_from_string(row.join(" "), ' ').unwrap());
        }

        Ok(())
    }

    pub fn parse_from_csv(&mut self, files: [&str; 3]) -> Result<(), Box<dyn Error>> {
        self.parse_symbols(files[0])?;
        self.parse_paytable(files[1])?;
        self.parse_reels(files[2])
    }

    pub fn calculate_win(&self, spin: Combo, coins: usize) -> Option<(Combo, u64)> {
        let mut sorted_combos: Vec<&Combo> = self.paytable.keys().collect();
        sorted_combos.sort_by_key(|c| self.paytable.get(*c).unwrap());

        if let Some(win_combo) = sorted_combos.iter().rev().find(|c| {
            c.iter()
                .enumerate()
                .all(|(i, x)| self.combo_symbols.get(x).unwrap() & spin[i] == spin[i])
        }) {
            Some((
                win_combo.to_vec(),
                *self.paytable.get(&**win_combo).unwrap().get(coins).unwrap(),
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

        writeln!(f, "{:=<47}", "").unwrap();

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

        writeln!(f, "{:=<47}", "").unwrap();

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

        Ok(())
    }
}

//struct TooMuchSymbolsError {}
//struct SymbolNotFoundError {}

#[derive(Debug)]
enum ParTableParseError {
    TooMuchSymbolsError,
    SymbolNotFoundError,
}

impl Error for ParTableParseError {}

impl fmt::Display for ParTableParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParTableParseError::TooMuchSymbolsError => {
                write!(f, "Too much symbols in table (max 256)")
            }
            ParTableParseError::SymbolNotFoundError => write!(f, "Symbol not found for pattern"),
        }
    }
}