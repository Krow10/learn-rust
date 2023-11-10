use cfonts::{Colors, Rgb};
use image::{io::Reader as ImageReader, GenericImageView};
use slot_machine::{protocol::Status, GAMES_FOLDER};
use std::{
    collections::HashMap,
    fs, io, panic,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::Rect, style::Color, widgets::ListState};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

pub type CrosstermTerminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stderr>>;

pub const EVENT_POLL_INTERVAL_MS: u64 = 250;
pub const FRAMES_PER_SECONDS: u64 = 60;
pub const ANIMATION_WAIT_TIME: Duration = Duration::from_millis(400);
pub const ANIMATION_SKIP_TIMEOUT: Duration = Duration::from_millis(200);
pub const SERVER_PING_TIMEOUT: Duration = Duration::from_secs(5);

pub const SPIN_BASE_SPEED: f64 = 3.0;
pub const REEL_SPEED_FACTOR: f64 = 1.08;

pub const SYMBOLS_DISPLAY_RATIO: f64 = 0.8;
pub const SYMBOLS_DISTANCE_RATIO: f64 = 1.0;

// Can be extended to any number of colors (at least one)
// https://coolors.co/palette/ef476f-ffd166-06d6a0-118ab2-073b4c
pub const GAME_IDLE_COLOR: [u8; 3] = [0xef, 0x47, 0x6f];
pub const GAME_WIN_COLOR: [u8; 3] = [0x06, 0xd6, 0xa0];
pub const PRIMARY_TEXT_COLOR: [u8; 3] = [0xff, 0xd1, 0x66];
pub const SECONDARY_TEXT_COLOR: [u8; 3] = [0x11, 0x8a, 0xb2];

pub const CFONTS_TEXT_COLORS: [Colors; 2] = [
    Colors::Rgb(Rgb::Val(
        PRIMARY_TEXT_COLOR[0],
        PRIMARY_TEXT_COLOR[1],
        PRIMARY_TEXT_COLOR[2],
    )),
    Colors::Rgb(Rgb::Val(
        SECONDARY_TEXT_COLOR[0],
        SECONDARY_TEXT_COLOR[1],
        SECONDARY_TEXT_COLOR[2],
    )),
];

pub const CFONTS_WIN_COLORS: [Colors; 2] = [
    Colors::Rgb(Rgb::Val(
        GAME_WIN_COLOR[0],
        GAME_WIN_COLOR[1],
        GAME_WIN_COLOR[2],
    )),
    Colors::Rgb(Rgb::Val(
        SECONDARY_TEXT_COLOR[0],
        SECONDARY_TEXT_COLOR[1],
        SECONDARY_TEXT_COLOR[2],
    )),
];

pub const CFONTS_IDLE_COLORS: [Colors; 2] = [
    Colors::Rgb(Rgb::Val(
        GAME_IDLE_COLOR[0],
        GAME_IDLE_COLOR[1],
        GAME_IDLE_COLOR[2],
    )),
    Colors::Rgb(Rgb::Val(
        SECONDARY_TEXT_COLOR[0],
        SECONDARY_TEXT_COLOR[1],
        SECONDARY_TEXT_COLOR[2],
    )),
];

use crate::{
    handlers::EventHandler, handlers::StreamHandler, ui::render::render,
    updates::update_animations, JSON_SYMBOLS_FILE,
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnimationState {
    Idle,
    Spin,
    Balance,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    Game,
    Help,
}

#[derive(Debug)]
pub struct State {
    pub symbols_mapping: HashMap<String, SpinSymbol>,
    pub reels_symbols: Vec<Vec<String>>,
    pub scroll_positions: Vec<(f64, f64)>,
    pub animation_state: AnimationState,
    pub animation_duration: Instant,
    pub animation_skip_timeout: Instant,
    pub n_reels: u64,
    pub spin_indexes: Vec<usize>,
    pub spin_targets: Vec<(isize, bool)>,
    pub bet: u64,
    pub max_bet: u64,
    pub win: u64,
    pub next_win: u64,
    pub balance: u64,
    pub next_balance: u64,
    pub available_games: Vec<String>,
    pub selected_game: ListState,
    pub active_screen: Screen,
    pub daemon_status: Status,
    pub title_text_bold_line: isize,
}

impl Default for State {
    fn default() -> Self {
        State {
            symbols_mapping: HashMap::new(),
            reels_symbols: vec![],
            scroll_positions: vec![],
            animation_state: AnimationState::Idle,
            animation_duration: Instant::now(),
            animation_skip_timeout: Instant::now(),
            n_reels: 3,
            spin_indexes: vec![],
            spin_targets: vec![],
            bet: 1,
            max_bet: 1,
            win: 0,
            next_win: 0,
            balance: 0,
            next_balance: 0,
            available_games: vec![],
            selected_game: ListState::default(),
            active_screen: Screen::MainMenu,
            daemon_status: Status::default(),
            title_text_bold_line: -1,
        }
    }
}

impl State {
    pub fn current_game(&self) -> Option<String> {
        let selected_game = self.selected_game.selected()?;
        self.available_games.get(selected_game).cloned()
    }
}

#[derive(Debug)]
pub struct App {
    terminal: CrosstermTerminal,
    pub events: EventHandler,
    pub client: StreamHandler,
    pub should_quit: bool,
    pub state: State,
}

impl App {
    pub fn render_tick(&mut self) {
        // TODO: Can compute FPS here
        update_animations(self);
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn new(terminal: CrosstermTerminal, events: EventHandler, client: StreamHandler) -> Self {
        Self {
            terminal,
            events,
            client,
            should_quit: false,
            state: State { ..State::default() },
        }
    }

    pub fn init_menu(&mut self) {
        // TODO: Find clean way to periodically query server status and act accordingly on lost connection
        self.client.send_status_message();
        self.load_games();
    }

    pub fn init_game(&mut self, game: String) {
        self.client.send_init_message(game.to_string());
        self.load_symbols_mapping(
            Path::new(GAMES_FOLDER)
                .join(game.to_string())
                .join("display.csv"),
        );
        self.load_reels(Path::new(GAMES_FOLDER).join(game).join("reels.csv"));
    }

    fn load_spin_symbol(&self, symbol: &Symbol) -> Result<SpinSymbol> {
        let img = ImageReader::open(symbol.path.clone())?
            .decode()
            .expect("Could not decode image");

        // TODO: Investigate if downsampling image can help performance
        // img.resize(50, 50, image::imageops::FilterType::Gaussian);
        Ok(SpinSymbol {
            points: img
                .to_luma8()
                .enumerate_pixels()
                .filter(|(_, _, luma)| luma.0[0] < symbol.luma_threshold)
                .map(|(x, y, _)| (x as f64, y as f64))
                .collect(),
            size: (img.dimensions().0 as f64, img.dimensions().1 as f64),
            color: symbol.color,
        })
    }

    pub fn load_symbols_mapping(&mut self, path: PathBuf) {
        let mut rdr = csv::Reader::from_path(path).unwrap();

        let f = fs::read_to_string(JSON_SYMBOLS_FILE).expect("Unable to read file");
        let symbols: Vec<Symbol> = serde_json::from_str(&f).unwrap();

        self.state.symbols_mapping = HashMap::from_iter(rdr.deserialize().map(|r| {
            let (symbol, display): (String, String) = r.unwrap();
            (
                symbol,
                self.load_spin_symbol(symbols.iter().find(|s| s.name == display).unwrap())
                    .unwrap(),
            )
        }));
    }

    pub fn load_reels(&mut self, path: PathBuf) {
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
    }

    pub fn load_games(&mut self) {
        // TODO: Check for errors / empty directory / etc.
        self.state.available_games = fs::read_dir(GAMES_FOLDER)
            .unwrap()
            .filter(|p| p.as_ref().unwrap().metadata().unwrap().is_dir())
            .map(|p| p.unwrap().file_name().into_string().unwrap())
            .collect();
        //app.state.available_games.extend_from_slice(&["test1".to_string(), "test2".to_string(), "test3".to_string()]);
        self.state.selected_game.select(Some(0));
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

        self.init_menu();
        Ok(())
    }

    pub fn autoresize(&mut self) -> Result<()> {
        Ok(self.terminal.autoresize()?)
    }

    pub fn get_term_size(&self) -> Result<Rect> {
        Ok(self.terminal.size()?)
    }

    pub fn draw(&mut self) -> Result<()> {
        self.terminal.draw(|frame| render(&mut self.state, frame))?;
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
