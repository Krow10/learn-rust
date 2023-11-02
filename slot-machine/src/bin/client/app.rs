use std::{collections::HashMap, fs, io, panic, path::PathBuf, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::Rect, style::Color};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

pub type CrosstermTerminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stderr>>;

pub const EVENT_POLL_INTERVAL_MS: u64 = 250;
pub const ANIMATION_FRAMES_PER_SECONDS: u64 = 60;
pub const ANIMATION_WAIT_TIME: Duration = Duration::from_secs(4);

pub const SPIN_BASE_SPEED: f64 = 1.2;
pub const REEL_SPEED_FACTOR: f64 = 1.08;

pub const SYMBOLS_DISPLAY_RATIO: f64 = 0.8;
pub const SYMBOLS_DISTANCE_RATIO: f64 = 1.0;

use crate::{
    handlers::ClientHandler,
    handlers::EventHandler,
    ui::{self, load_spin_symbol},
    updates::update_animations,
    JSON_SYMBOLS_FILE,
};

#[derive(Debug, Clone)]
pub struct SpinSymbol {
    pub points: Vec<(f64, f64)>,
    pub size: (f64, f64),
    pub color: Color,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub path: String,
    pub luma_threshold: u8,
    #[serde(default)]
    #[serde_as(as = "DisplayFromStr")]
    pub color: Color,
}

#[derive(Debug)]
pub struct State {
    pub symbols_mapping: HashMap<String, SpinSymbol>,
    pub reels_symbols: Vec<Vec<String>>,
    pub scroll_positions: Vec<(f64, f64)>,
    pub is_spinning: bool,
    pub n_reels: u64,
    pub spin_indexes: Vec<usize>,
    pub spin_targets: Vec<(isize, bool)>,
    pub bet: u64,
}

#[derive(Debug)]
pub struct App {
    terminal: CrosstermTerminal,
    pub events: EventHandler,
    pub client: ClientHandler,
    pub should_quit: bool,
    pub game: String,
    pub state: State,
}

impl App {
    pub fn tick(&mut self) {
        update_animations(self);
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn new(
        terminal: CrosstermTerminal,
        game: String,
        events: EventHandler,
        client: ClientHandler,
    ) -> Self {
        Self {
            terminal,
            events,
            client,
            should_quit: false,
            game,
            state: State {
                symbols_mapping: HashMap::new(),
                reels_symbols: vec![],
                scroll_positions: vec![],
                is_spinning: false,
                n_reels: 3,
                spin_indexes: vec![],
                spin_targets: vec![],
                bet: 1,
            },
        }
    }

    pub fn load_symbols_mapping(&mut self, path: PathBuf) {
        println!("[*] Loading game symbols...");
        let mut rdr = csv::Reader::from_path(path).unwrap();

        let f = fs::read_to_string(JSON_SYMBOLS_FILE).expect("Unable to read file");
        let symbols: Vec<Symbol> = serde_json::from_str(&f).unwrap();

        self.state.symbols_mapping = HashMap::from_iter(rdr.deserialize().map(|r| {
            let (symbol, display): (String, String) = r.unwrap();
            (
                symbol,
                load_spin_symbol(symbols.iter().find(|s| s.name == display).unwrap()).unwrap(),
            )
        }));

        println!(
            "[*] Symbols mapping: {:?}",
            self.state.symbols_mapping.keys()
        );
        println!("[+] Game symbols loaded !");
    }

    pub fn load_reels(&mut self, path: PathBuf) {
        println!("[*] Loading game reels...");
        let rdr = csv::Reader::from_path(path).unwrap();
        let records = rdr.into_deserialize::<Vec<String>>().collect::<Vec<_>>();

        self.state.n_reels = records.first().unwrap().as_ref().unwrap().len() as u64;

        for _ in 1..=self.state.n_reels {
            self.state.reels_symbols.push(vec![]);
            self.state.scroll_positions.push((0.0, 0.0));
            self.state.spin_indexes.push(0);
            self.state.spin_targets.push((0, false));
        }

        for result in records {
            let row: Vec<String> = result.unwrap();

            row.iter().enumerate().for_each(|(i, s)| {
                self.state.reels_symbols[i].push(s.to_string());
            });
        }

        println!("[*] Spin display images: {:?}", self.state.reels_symbols);
        println!("[+] Game reels loaded !");
    }

    pub fn enter(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        let panic_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset the terminal");
            panic_hook(panic);
        }));

        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    pub fn autoresize(&mut self) -> Result<()> {
        Ok(self.terminal.autoresize()?)
    }

    pub fn get_term_size(&self) -> Result<Rect> {
        Ok(self.terminal.size()?)
    }

    pub fn draw(&mut self) -> Result<()> {
        self.terminal.draw(|frame| ui::render(&self.state, frame))?;
        Ok(())
    }

    fn reset() -> Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}