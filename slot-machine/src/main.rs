extern crate cfonts;
use std::{
    fs,
    time::{Duration, Instant},
};

use ansi_to_tui::IntoText;
use cfonts::{render::RenderedString, Fonts, Options};
use rand::Rng;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use image::{io::Reader as ImageReader, GenericImageView};
use ratatui::{
    prelude::*,
    widgets::{
        canvas::{Canvas, Points},
        Block, Borders,
    },
    Frame,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::io::{stdout, Result};

const EVENT_POLL_INTERVAL_MS: u64 = 16;
const MAX_FPS: u64 = 30;

const N_REELS: usize = 5;
const SPIN_BASE_SPEED: f64 = 7.0;
const JSON_SYMBOLS_FILE: &str = "./data/display_symbols.json";
const SYMBOLS_DISPLAY_RATIO: f64 = 0.9;

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
    spin_display_images: Vec<SpinSymbol>,
    scroll_positions: Vec<(f64, f64)>,
    animation_cycle: FrameCycle,
}

struct SpinSymbol {
    points: Vec<(f64, f64)>,
    size: (f64, f64),
    color: Color,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct Symbol {
    name: String,
    path: String,
    luma_threshold: u8,
    #[serde(default)]
    #[serde_as(as = "DisplayFromStr")]
    color: Color,
}

fn load_spin_symbol(symbol: &Symbol) -> Result<SpinSymbol> {
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

fn render_tui(frame: &mut Frame, area: Rect, state: &State) {
    let slot_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (1..=N_REELS)
                .map(|_| Constraint::Percentage(100 / N_REELS as u16))
                .collect::<Vec<_>>(),
        )
        .split(area);

    let _styled_text_widget_factory = |(x, y): (f64, f64), text: String| -> Canvas<'_, _> {
        Canvas::default()
            .block(Block::new().borders(Borders::ALL))
            .x_bounds([0.0, area.width as f64])
            .y_bounds([0.0, area.height as f64])
            .marker(symbols::Marker::Braille)
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

    let _image_draw_widget_factory = |x: f64,
                                      y: f64,
                                      points: Vec<(f64, f64)>,
                                      image_size: (f64, f64),
                                      display_ratio: f64,
                                      color: Color,
                                      align_center: bool|
     -> Canvas<'_, _> {
        Canvas::default()
            .block(Block::new().borders(Borders::ALL))
            .x_bounds([0.0, area.width as f64])
            .y_bounds([0.0, area.height as f64])
            .marker(symbols::Marker::Braille)
            .paint(move |ctx| {
                let (img_w, img_h) = image_size;
                let aspect_ratio = (
                    display_ratio * area.width as f64 / img_w,
                    display_ratio * area.height as f64 / img_h,
                );
                let center_shift = (
                    (1.0 - display_ratio) * area.width as f64 / 2.0,
                    -(1.0 - display_ratio) * area.height as f64 / 2.0,
                );

                ctx.draw(&Points {
                    coords: &points
                        .iter()
                        .map(|(_x, _y)| {
                            (
                                x + *_x * aspect_ratio.0
                                    + (if align_center { center_shift.0 } else { 0.0 }),
                                y + area.height as f64 - (*_y * aspect_ratio.1)
                                    + (if align_center { center_shift.1 } else { 0.0 }),
                            )
                        })
                        .collect::<Vec<_>>(),
                    color,
                });
            })
    };

    slot_layout.iter().enumerate().for_each(|(i, l)| {
        let (x, y) = state.scroll_positions[i];
        /*frame.render_widget(
            _styled_text_widget_factory((x, y).clone(), state.spin[i].clone()),
            *l,
        );*/

        frame.render_widget(
            _image_draw_widget_factory(
                x,
                y,
                state.spin_display_images[i].points.clone(),
                state.spin_display_images[i].size,
                SYMBOLS_DISPLAY_RATIO,
                state.spin_display_images[i].color,
                true,
            ),
            *l,
        )
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

fn get_random_spin() -> Vec<Symbol> {
    let f = fs::read_to_string(JSON_SYMBOLS_FILE).expect("Unable to read file");
    let symbols: Vec<Symbol> = serde_json::from_str(&f).unwrap();

    rand::thread_rng()
        .sample_iter(rand::distributions::Uniform::from(
            0..=symbols.len() - 1, // Account for indexes, start at 0
        ))
        .take(N_REELS)
        .map(|x| Symbol {
            name: symbols[x].name.clone(),
            path: symbols[x].path.clone(),
            luma_threshold: symbols[x].luma_threshold,
            color: symbols[x].color,
        })
        .collect()
}

fn init_state() -> State {
    State {
        spin_display_images: get_random_spin()
            .iter()
            .map(|s| load_spin_symbol(s).expect("Could not load symbol"))
            .collect(),
        scroll_positions: Vec::from_iter((1..=N_REELS).map(|_| (0.0, 0.0))),
        animation_cycle: FrameCycle {
            current: 0,
            max: MAX_FPS,
        },
    }
}

fn main() -> Result<()> {
    install_panic_hook();

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut duration = Instant::now();
    let mut state = init_state();
    let mut play_animation = false;

    loop {
        terminal.draw(|frame| {
            render_tui(frame, frame.size(), &state);
        })?;

        if event::poll(std::time::Duration::from_millis(EVENT_POLL_INTERVAL_MS))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        break;
                    } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Char(' ') {
                        play_animation = !play_animation;
                    } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('r') {
                        state = init_state();
                    }
                }
                Event::Resize(_width, _height) => {
                    terminal.autoresize()?;
                }
                _ => (),
            }
        }

        if duration.elapsed() >= Duration::from_millis(1000 / MAX_FPS) {
            if play_animation {
                state
                    .scroll_positions
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, (_, y))| {
                        let h = terminal.size().unwrap().height as f64;
                        if *y <= -h {
                            *y = h
                        } else {
                            *y -= SPIN_BASE_SPEED + i as f64 * 1.2
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
