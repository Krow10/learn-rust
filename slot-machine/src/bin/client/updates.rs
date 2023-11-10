use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{
    AnimationState, App, Screen, State, ANIMATION_SKIP_TIMEOUT, ANIMATION_WAIT_TIME,
    REEL_SPEED_FACTOR, SPIN_BASE_SPEED, SYMBOLS_DISTANCE_RATIO,
};

pub fn update_animations(app: &mut App) {
    match app.state.animation_state {
        AnimationState::Idle => match app.state.active_screen {
            // Simulate "shiness effect" for title screen text by bolding the lines one at a time at a fixed interval
            Screen::MainMenu => {
                if app
                    .state
                    .animation_duration
                    .elapsed()
                    .cmp(&ANIMATION_WAIT_TIME.mul_f64(0.5))
                    .is_ge()
                {
                    app.state.title_text_bold_line = app.state.title_text_bold_line.wrapping_add(1);
                    app.state.animation_duration = Instant::now();
                }
            }
            Screen::Game => {}
            Screen::Help => {}
        },
        AnimationState::Spin => {
            if app.state.spin_targets.iter().all(|(_, stopped)| *stopped) {
                app.state.animation_state = AnimationState::Balance;
                app.state.animation_duration = Instant::now();
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

                                app.state.spin_indexes[i] = (app.state.spin_indexes[i] + 1)
                                    % app.state.reels_symbols[i].len();

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
        AnimationState::Balance => {
            if app.state.balance != app.state.next_balance {
                if app
                    .state
                    .animation_duration
                    .elapsed()
                    .cmp(&ANIMATION_WAIT_TIME.mul_f64(0.6))
                    .is_ge()
                {
                    if app.state.win != app.state.next_win {
                        app.state.win = app.state.next_win;
                    }

                    if app.state.balance < app.state.next_balance {
                        app.state.balance += 1;
                    } else if app.state.balance > app.state.next_balance {
                        app.state.balance -= 1;
                    }

                    app.state.animation_duration = Instant::now();
                }
            } else {
                app.state.animation_state = AnimationState::Idle;
            }
        }
    }
}

fn next_game_animation(app: &mut App) {
    /* TODO: Make it more generic based on the flow below
        From current animation state :
            1. Check if current animation state is skippable
                Yes -> Setup next animation state
                No  -> Do nothing
    */
    match app.state.animation_state {
        AnimationState::Idle => {
            app.state.animation_state = AnimationState::Spin;
            app.state.animation_duration = Instant::now();
            app.state
                .spin_targets
                .iter_mut()
                .for_each(|(target, stopped)| {
                    *target = -1;
                    *stopped = false;
                });

            app.state.next_win = 0;
            app.state.win = app.state.next_win;

            app.state.next_balance = app.state.next_balance.saturating_sub(app.state.bet);
            app.state.balance = app.state.next_balance;

            app.client
                .send_spin_message(app.state.current_game().unwrap(), app.state.bet);
        }
        AnimationState::Spin => {
            if app
                .state
                .animation_duration
                .elapsed()
                .cmp(&ANIMATION_WAIT_TIME)
                .is_ge()
            {
                app.state
                    .spin_targets
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, (target, stopped))| {
                        *app.state.spin_indexes.get_mut(i).unwrap() = *target as usize;
                        *app.state.scroll_positions.get_mut(i).unwrap() = (0.0, 0.0);
                        *stopped = true;
                    });
                app.state.animation_state = AnimationState::Balance;
                app.state.animation_duration = Instant::now();
            }
        }
        AnimationState::Balance => {
            app.state.balance = app.state.next_balance;
            app.state.animation_state = AnimationState::Idle
        }
    }
}

pub fn update_keys(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc | KeyCode::Char('q') => match app.state.active_screen {
            Screen::MainMenu => app.quit(),
            Screen::Game => {
                let previous_game = app.state.selected_game.selected();
                app.state = State::default();
                app.init_menu();
                app.state.selected_game.select(previous_game);
            }
            Screen::Help => app.state.active_screen = Screen::Game,
        },
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit()
            }
        }
        KeyCode::F(1) => match app.state.active_screen {
            Screen::MainMenu => {}
            Screen::Game => app.state.active_screen = Screen::Help,
            Screen::Help => app.state.active_screen = Screen::Game,
        },
        KeyCode::Enter | KeyCode::Char(' ') => match app.state.active_screen {
            Screen::MainMenu => {
                app.init_game(app.state.current_game().unwrap());
                app.state.title_text_bold_line = -1;
                app.state.active_screen = Screen::Game;
            }
            Screen::Game => {
                if app.state.animation_state == AnimationState::Idle
                    && app.state.balance.overflowing_sub(app.state.bet).1
                {
                    // TODO: Show insufficent balance message
                } else if app
                    .state
                    .animation_skip_timeout
                    .elapsed()
                    .cmp(&ANIMATION_SKIP_TIMEOUT)
                    .is_ge()
                {
                    next_game_animation(app);
                    app.state.animation_skip_timeout = Instant::now();
                }
            }
            Screen::Help => {}
        },
        KeyCode::Up | KeyCode::Char('+') | KeyCode::Char('j') => match app.state.active_screen {
            Screen::MainMenu => {
                if let Some(selected) = app.state.selected_game.selected() {
                    app.state
                        .selected_game
                        .select(Some(if selected.overflowing_sub(1).1 {
                            app.state.available_games.len() - 1
                        } else {
                            selected - 1
                        }));
                }
            }
            Screen::Game => {
                if app.state.animation_state == AnimationState::Idle
                    && app.state.bet < app.state.max_bet
                {
                    app.state.bet += 1;
                }
            }
            Screen::Help => {}
        },
        KeyCode::Down | KeyCode::Char('-') | KeyCode::Char('k') => match app.state.active_screen {
            Screen::MainMenu => {
                if let Some(selected) = app.state.selected_game.selected() {
                    app.state
                        .selected_game
                        .select(Some((selected + 1) % app.state.available_games.len()));
                }
            }
            Screen::Game => {
                if app.state.animation_state == AnimationState::Idle && app.state.bet > 1 {
                    app.state.bet -= 1;
                }
            }
            Screen::Help => {}
        },
        _ => {}
    };
}

pub fn update_spin(app: &mut App, spin: Vec<isize>, win: u64, balance: u64) {
    app.state.next_balance = balance;
    app.state.next_win = win;
    app.state
        .spin_targets
        .iter_mut()
        .enumerate()
        .for_each(|(i, (target, _))| {
            *target = app.state.reels_symbols[i].len() as isize - 1 - spin[i]
        });
}
