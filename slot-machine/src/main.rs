extern crate cfonts;
use std::time::{Duration, Instant};

use ansi_to_tui::IntoText;
use cfonts::{render::RenderedString, Fonts, Options};
use rand::Rng;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{canvas::Canvas, Block, Borders},
    Frame,
};
use std::io::{stdout, Result};

const N_REELS: usize = 3;
const EVENT_POLL_INTERVAL: u64 = 16;
const MAX_FPS: u64 = 30;

struct FrameCycle {
    current: u64,
    max: u64,
}

impl FrameCycle {
    fn incr(&mut self) {
        self.current += 1 % self.max;
    }
}

// Override panic hook to safely restore terminal state
fn install_panic_hook() {
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        disable_raw_mode().expect("Could not disable raw mode");
        stdout()
            .execute(LeaveAlternateScreen)
            .expect("Could not leave alternate screen");
        prev_hook(info);
    }));
}

struct State {
    spin: Vec<String>,
    scroll_positions: Vec<(f64, f64)>,
    animation_cycle: FrameCycle,
}

fn render_tui(frame: &mut Frame, area: Rect, state: &State) {
    let slot_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (1..=N_REELS)
                .map(|_| Constraint::Percentage(100 / N_REELS as u16))
                .collect::<Vec<_>>(),
        )
        .split(area);

    let _styled_text_widget_factory = |text: String, (x, y): (f64, f64)| -> Canvas<'_, _> {
        Canvas::default()
            .block(Block::new().borders(Borders::ALL))
            .x_bounds([0.0, area.width as f64])
            .y_bounds([0.0, area.height as f64])
            .marker(symbols::Marker::HalfBlock)
            .paint(move |ctx| {
                stylize_text(text.clone())
                    .vec
                    .iter()
                    .enumerate()
                    .for_each(|(k, s)| {
                        let lines = s
                            .replace("\n", "")
                            .to_string()
                            .as_bytes()
                            .into_text()
                            .unwrap()
                            .lines;
                        for l in lines {
                            ctx.print(x, y - k as f64, l);
                        }
                    });
            })
    };

    slot_layout.iter().enumerate().for_each(|(i, l)| {
        let (x, y) = state.scroll_positions[i];
        frame.render_widget(
            _styled_text_widget_factory(state.spin[i].clone(), (x, y).clone()),
            *l,
        );
    });
}

fn stylize_text(text: String) -> RenderedString {
    cfonts::render(Options {
        text,
        font: Fonts::FontBlock,
        // From https://coolors.co/palettes/trending
        gradient: ["#ffe5ec", "#ffc2d1", "#ffb3c6", "#ff8fab", "#fb6f92"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        transition_gradient: true,
        spaceless: true,
        ..Options::default()
    })
}

fn get_random_spin() -> Vec<String> {
    let test_characters = ["BAR", "7", "Z"];
    rand::thread_rng()
        .sample_iter(rand::distributions::Uniform::from(
            0..=test_characters.len() - 1, // Account for indexes, start at 0
        ))
        .take(N_REELS)
        .map(|x| test_characters[x].to_string())
        .collect()
}

fn main() -> Result<()> {
    install_panic_hook();

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let term_size = terminal.size().expect("Could not get terminal size");
    let mut duration = Instant::now();
    let mut state = State {
        spin: get_random_spin(),
        scroll_positions: Vec::from_iter((1..=N_REELS).map(|_| {
            (
                term_size.width as f64 / 2.0 - 30.0,
                term_size.height as f64 - 3.0,
            )
        })),
        animation_cycle: FrameCycle {
            current: 0,
            max: MAX_FPS,
        },
    };
    let mut play_animation = false;

    loop {
        terminal.draw(|frame| {
            render_tui(frame, frame.size(), &state);
        })?;

        if event::poll(std::time::Duration::from_millis(EVENT_POLL_INTERVAL))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        break;
                    } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('p') {
                        play_animation = !play_animation;
                    }
                }
                Event::Resize(width, height) => {
                    // TODO: Resize symbols according to window
                    eprintln!("Resized: {:?} {:?}", width, height);
                }
                _ => (),
            }
        }

        if duration.elapsed() >= Duration::from_millis(1000 / MAX_FPS) {
            if play_animation {
                state.scroll_positions.iter_mut().for_each(|(_, y)| {
                    if *y <= 0.0 {
                        *y = term_size.height as f64 + 10.0 as f64
                    } else {
                        *y -= 1.0
                    }
                });
            }

            state.animation_cycle.incr();
            duration = Instant::now();
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
