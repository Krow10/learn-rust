extern crate cfonts;

use ansi_to_tui::IntoText;
use cfonts::{render::RenderedString, Fonts, Options};

use image::{io::Reader as ImageReader, GenericImageView};
use ratatui::{
    prelude::*,
    widgets::{
        canvas::{Canvas, Points},
        Block, Borders,
    },
    Frame,
};

use std::io::Result;

use crate::app::{SpinSymbol, State, Symbol, SYMBOLS_DISPLAY_RATIO, SYMBOLS_DISTANCE_RATIO};

pub fn render(state: &State, frame: &mut Frame) {
    let area = frame.size();
    let slot_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (1..=state.n_reels)
                .map(|_| Constraint::Percentage(100 / state.n_reels as u16))
                .collect::<Vec<_>>(),
        )
        .split(area);

    let _styled_text_widget_factory = |x: f64, y: f64, text: String| -> Canvas<'_, _> {
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
                    if align_center {
                        (1.0 - display_ratio) * area.width as f64 / 2.0
                    } else {
                        0.0
                    },
                    if align_center {
                        -(1.0 - display_ratio) * area.height as f64 / 2.0
                    } else {
                        0.0
                    },
                );

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

        for (j, symbol) in vec![
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
        ]
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

pub fn stylize_text(text: String) -> RenderedString {
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

pub fn load_spin_symbol(symbol: &Symbol) -> Result<SpinSymbol> {
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
