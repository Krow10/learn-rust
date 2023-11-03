use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, REEL_SPEED_FACTOR, SPIN_BASE_SPEED, SYMBOLS_DISTANCE_RATIO};

pub fn update_animations(app: &mut App) {
    if app.state.is_spinning {
        if app.state.spin_targets.iter().all(|(_, stopped)| *stopped) {
            app.state.is_spinning = false;
        } else {
            let term_h = app.get_term_size().unwrap().height as f64;
            app.state
                .scroll_positions
                .iter_mut()
                .enumerate()
                .for_each(|(i, (_, y))| {
                    let (target, stopped) = app.state.spin_targets.get_mut(i).unwrap();
                    if !*stopped {
                        if *y <= -term_h {
                            *y += term_h * SYMBOLS_DISTANCE_RATIO;

                            // Check if performance can be improved by just using an index to keep track of spin to display
                            app.state.spin_indexes[i] =
                                (app.state.spin_indexes[i] + 1) % app.state.reels_symbols[i].len();

                            if *target as usize == app.state.spin_indexes[i] {
                                *stopped = true;
                            }
                        } else {
                            *y -= SPIN_BASE_SPEED + 1.0 / (i + 1) as f64 * REEL_SPEED_FACTOR;
                        }
                    }
                });
        }
    }
}

pub fn update_keys(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc | KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit()
            }
        }
        KeyCode::Char(' ') => {
            if !app.state.is_spinning {
                app.state.is_spinning = true;
                app.state
                    .spin_targets
                    .iter_mut()
                    .for_each(|(target, stopped)| {
                        *target = -1;
                        *stopped = false;
                    });
                app.client
                    .send_spin_message(app.game.to_string(), app.state.bet);
            }
        }
        _ => {}
    };
}

pub fn update_spin(app: &mut App, spin: Vec<isize>, win: u64, balance: u64) {
    app.state
        .spin_targets
        .iter_mut()
        .enumerate()
        .for_each(|(i, (target, _))| {
            *target = app.state.reels_symbols[i].len() as isize - 1 - spin[i]
        });
}
