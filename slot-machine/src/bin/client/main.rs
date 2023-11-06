use std::fs;

use std::os::unix::net::UnixStream;
use std::path::Path;

use std::time::Instant;

use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use slot_machine::{utils::get_user_input, GAMES_FOLDER, SOCKET_PATH};

use crate::app::App;

use crate::app::EVENT_POLL_INTERVAL_MS;
use crate::app::FRAMES_PER_SECONDS;
use crate::handlers::Event;
use crate::handlers::EventHandler;
use crate::handlers::Stream;
use crate::handlers::StreamHandler;

use crate::updates::update_info;
use crate::updates::{update_keys, update_spin};
use anyhow::Result;

mod app;
mod handlers;
mod ui;
mod updates;

pub const JSON_SYMBOLS_FILE: &str = "./data/display_symbols.json";

fn main() -> Result<()> {
    let paths: Vec<String> = fs::read_dir(GAMES_FOLDER)
        .unwrap()
        .filter(|p| p.as_ref().unwrap().metadata().unwrap().is_dir())
        .map(|p| p.unwrap().file_name().into_string().unwrap())
        .collect();

    println!("[x] Select a game:");
    paths.iter().enumerate().for_each(|(i, p)| {
        println!("{}. {}", i + 1, p);
    });

    let game_choice = &paths[1/*get_user_input().unwrap().parse::<usize>().unwrap() - 1*/];
    println!("[+] Playing \"{}\" !", game_choice);

    println!("Enter bet amount:");
    let bet = 2; // get_user_input().unwrap().parse::<usize>().unwrap();

    let stream = UnixStream::connect(SOCKET_PATH).unwrap();

    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(EVENT_POLL_INTERVAL_MS);
    let client = StreamHandler::new(stream);
    let mut app = App::new(terminal, game_choice.to_string(), events, client);

    app.state.bet = bet;
    app.client.send_balance_message();
    app.load_symbols_mapping(
        Path::new(GAMES_FOLDER)
            .join(game_choice)
            .join("display.csv"),
    );

    app.load_reels(Path::new(GAMES_FOLDER).join(game_choice).join("reels.csv"));

    app.enter()?;

    let mut duration = Instant::now();
    let dt = 1000.0 / FRAMES_PER_SECONDS as f64;
    let mut accumulator = 0.0;

    while !app.should_quit {
        app.draw()?;

        // From https://blog.sklambert.com/using-time-based-animation-implement/
        accumulator += duration.elapsed().as_millis() as f64;
        duration = Instant::now();

        while accumulator >= dt {
            app.render_tick();
            accumulator -= dt;
        }

        match app.events.next()? {
            Event::Noop => {}
            Event::Tick => {}
            Event::Key(key_event) => update_keys(&mut app, key_event),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => app.autoresize()?,
        };

        match app.client.next()? {
            Stream::Noop => {}
            Stream::Balance(balance) => {
                app.state.next_balance = balance;
                update_info(&mut app)
            }
            Stream::SpinResult(spin, win, balance) => update_spin(&mut app, spin, win, balance),
            Stream::ServerError(_err) => {}
        }
    }

    app.exit()?;
    Ok(())
}
