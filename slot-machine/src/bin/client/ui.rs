extern crate cfonts;

use ansi_to_tui::IntoText;
use cfonts::{render::RenderedString, Fonts, Options};

use ratatui::{
    prelude::*,
    widgets::{
        canvas::{Canvas, Points},
        Block, Borders, Widget,
    },
    Frame,
};

use crate::app::{State, SYMBOLS_DISPLAY_RATIO, SYMBOLS_DISTANCE_RATIO};

fn render_game_info(state: &State, layout: &Rect, frame: &mut Frame) {
    let info_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(*layout);

    let info_text_option = Options {
        font: Fonts::FontBlock,
        // https://coolors.co/gradients
        gradient: ["#82f4b1", "#30c67c"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        transition_gradient: true,
        spaceless: true,
        ..Options::default()
    };

    for (k, text) in vec![
        format!(
            "Win : {}{:?}",
            if state.win > 0 { "+" } else { "" },
            state.win
        ),
        format!("Balance : {:?}", state.balance),
    ]
    .iter()
    .enumerate()
    {
        frame.render_widget(
            CFontTextWidget {
                options: Options {
                    text: text.to_string(),
                    ..info_text_option.clone()
                },
                enable_text_scaling: true,
                align_center: true,
            },
            *info_layout.get(k).unwrap(),
        );
    }
}

fn render_reels(state: &State, layout: &Rect, frame: &mut Frame) {
    let area = frame.size();
    let slot_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (1..=state.n_reels)
                .map(|_| Constraint::Percentage(100 / state.n_reels as u16))
                .collect::<Vec<_>>(),
        )
        .split(*layout);

    // Hard to port over to a proper custom widget (separate struct) as the `Buffer` object required in the `render` method
    // doesn't support marker drawing (e.g. Braille, etc.) out-of-the-box like Canvas does. It would require setting each of
    // the `Cell` individually with the appropriate `symbol`.
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
                let center_shift = if align_center {
                    (
                        (1.0 - display_ratio) * area.width as f64 / 2.0,
                        -(1.0 - display_ratio) * area.height as f64 / 2.0,
                    )
                } else {
                    (0.0, 0.0)
                };

                let const_x = x + center_shift.0;
                let const_y = y + area.height as f64 + center_shift.1;

                ctx.draw(&Points {
                    coords: &points
                        .iter()
                        .map(|(_x, _y)| {
                            (
                                const_x + *_x * aspect_ratio.0,
                                const_y - *_y * aspect_ratio.1,
                            )
                        })
                        .collect::<Vec<_>>(),
                    color,
                });
            })
    };

    for (i, l) in slot_layout.iter().enumerate() {
        let (x, y) = state.scroll_positions[i];
        let distance = area.height as f64 * SYMBOLS_DISTANCE_RATIO;
        let spin_index = state.reels_symbols[i].len() - state.spin_indexes[i] - 1;

        // Display 3 symbols at a time on the reels
        let (previous_symbol, current_symbol, next_symbol) = (
            state.reels_symbols[i][if spin_index == 0 {
                state.reels_symbols[i].len() - 1
            } else {
                spin_index - 1
            }]
            .clone(),
            state.reels_symbols[i][spin_index].clone(),
            state.reels_symbols[i][if spin_index + 1 == state.reels_symbols[i].len() {
                0
            } else {
                spin_index + 1
            }]
            .clone(),
        );

        for (j, symbol) in vec![previous_symbol, current_symbol, next_symbol]
            .iter()
            .map(|s| state.symbols_mapping.get(s).unwrap())
            .enumerate()
        {
            frame.render_widget(
                _image_draw_widget_factory(
                    x,
                    y + distance * (1 - j as isize) as f64,
                    symbol.points.clone(),
                    symbol.size,
                    SYMBOLS_DISPLAY_RATIO,
                    symbol.color,
                    true,
                ),
                *l,
            )
        }
    }
}

fn render_header(state: &State, layout: &Rect, frame: &mut Frame) {
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(*layout);

    let header_text_option = Options {
        font: Fonts::FontConsole,
        // https://coolors.co/gradients
        gradient: ["#0061ff", "#60efff"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        transition_gradient: true,
        spaceless: true,
        ..Options::default()
    };

    for (k, text) in vec![
        format!("Show help: [{}]", "F1"),
        format!("{} created by {}", state.game, "Unknown"),
        format!("Version: {}", "0.0.1"),
    ]
    .iter()
    .enumerate()
    {
        frame.render_widget(
            CFontTextWidget {
                options: Options {
                    text: text.to_string(),
                    ..header_text_option.clone()
                },
                enable_text_scaling: false,
                align_center: true,
            },
            *header_layout.get(k).unwrap(),
        );
    }
}

pub fn render(state: &State, frame: &mut Frame) {
    let main_window_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(5),
            Constraint::Percentage(75),
            Constraint::Percentage(20),
        ])
        .split(frame.size());

    render_header(state, main_window_layout.get(0).unwrap(), frame);
    render_reels(state, main_window_layout.get(1).unwrap(), frame);
    render_game_info(state, main_window_layout.get(2).unwrap(), frame);
}

#[derive(Debug, Clone)]
pub struct CFontTextWidget {
    options: Options,
    enable_text_scaling: bool,
    // TODO: Support all align enum values from CFonts
    align_center: bool,
}

impl CFontTextWidget {
    fn stylized_text(&self, options_override: Option<Options>) -> RenderedString {
        if options_override.is_some() {
            cfonts::render(options_override.unwrap())
        } else {
            cfonts::render(self.options.clone())
        }
    }
}

impl Widget for CFontTextWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut stylized_text = self.stylized_text(None).vec;
        let mut text_height = stylized_text.len() as u16;

        // Check that the current stylized text can fit on the screen (height check only)
        // If not, use a smaller font if text scaling is enabled
        if self.enable_text_scaling
            && stylized_text.len() as u16
                + (area.height.saturating_sub(1).saturating_sub(text_height)) / 2
                >= area.height
        {
            let options = self.options.clone();
            stylized_text = self
                .stylized_text(Some(Options {
                    // Progressive font scaling from (any) => (tiny) => (console) to fit on the screen
                    font: if options.font == Fonts::FontTiny {
                        Fonts::FontConsole
                    } else {
                        Fonts::FontTiny
                    },
                    ..options
                }))
                .vec;
            text_height = stylized_text.len() as u16;
        }

        stylized_text.iter().enumerate().for_each(|(j, s)| {
            let lines = s.replace("\n", "").as_bytes().into_text().unwrap().lines;
            for l in lines {
                let center_shift = if self.align_center {
                    (
                        (area.width.saturating_sub(l.spans.len() as u16)) / 2,
                        (area.height.saturating_sub(text_height) / 2).wrapping_add(0),
                    )
                } else {
                    (0, 0)
                };

                buf.set_line(
                    area.left() + center_shift.0,
                    area.top() + j as u16 + center_shift.1,
                    &l,
                    area.width,
                );
            }
        });
    }
}
