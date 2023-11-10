use cfonts::{Fonts, Options};
use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier},
    symbols,
    widgets::{
        canvas::{Canvas, Points},
        Block, Borders,
    },
    Frame,
};

use crate::app::{
    State, CFONTS_IDLE_COLORS, CFONTS_TEXT_COLORS, CFONTS_WIN_COLORS, SYMBOLS_DISPLAY_RATIO,
    SYMBOLS_DISTANCE_RATIO,
};

use super::widgets::{AlignCenter, CFontTextWidget};

pub fn render_header(state: &State, layout: &Rect, frame: &mut Frame) {
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
        colors: CFONTS_TEXT_COLORS.to_vec(),
        spaceless: true,
        ..Options::default()
    };

    for (k, text) in vec![
        format!("Show help: [{}]", "F1"),
        format!("{} created by {}", state.current_game().unwrap(), "Unknown"),
        format!("Version: {}", "0.0.1"),
    ]
    .iter()
    .enumerate()
    {
        let w = CFontTextWidget::default()
            .options(Options {
                text: text.to_string(),
                ..header_text_option.clone()
            })
            .align_center(AlignCenter::Both);
        frame.render_widget(w, *header_layout.get(k).unwrap());
    }
}

pub fn render_reels(state: &State, layout: &Rect, frame: &mut Frame) {
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

pub fn render_win_overlay(state: &State, layout: &Rect, frame: &mut Frame) {
    let info_text_option = Options {
        font: Fonts::FontHuge,
        colors: CFONTS_WIN_COLORS.to_vec(),
        spaceless: true,
        ..Options::default()
    };

    let mut win_text_widget = CFontTextWidget::default()
        .options(Options {
            text: format!("Win : +{}", state.win),
            ..info_text_option
        })
        .align_center(AlignCenter::Both);

    if state.next_balance != state.balance {
        win_text_widget = win_text_widget.text_style_modifiers(vec![Modifier::RAPID_BLINK]);
    }

    frame.render_widget(win_text_widget, *layout);
}

pub fn render_footer(state: &State, layout: &Rect, frame: &mut Frame) {
    let footer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(*layout);

    let info_text_option = Options {
        font: Fonts::FontPallet,
        colors: CFONTS_IDLE_COLORS.to_vec(),
        spaceless: true,
        ..Options::default()
    };

    for (w_text, w_value, w_layout) in vec![
        (
            "Balance".to_string(),
            state.balance.to_string(),
            *footer_layout.get(0).unwrap(),
        ),
        (
            "Bet".to_string(),
            state.bet.to_string(),
            *footer_layout.get(2).unwrap(),
        ),
    ] {
        let w_info_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(w_layout);

        let w_text_widget = CFontTextWidget::default()
            .options(Options {
                text: w_text,
                ..info_text_option.clone()
            })
            .align_center(AlignCenter::Both);

        let w_value_widget = CFontTextWidget::default()
            .options(Options {
                text: w_value,
                ..info_text_option.clone()
            })
            .align_center(AlignCenter::Both);

        frame.render_widget(w_text_widget, *w_info_layout.get(0).unwrap());
        frame.render_widget(w_value_widget, *w_info_layout.get(1).unwrap());
    }
}
