extern crate cfonts;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{Screen, State};

use super::{
    game::{self},
    menu,
};

fn render_help(state: &State, frame: &mut Frame) {
    // TODO: Load help message from game paytable / symbols
    // => Has to be defined by user, might be too cumbersome to generate it programmatically
    frame.render_widget(
        Paragraph::new("Sample help message").block(
            Block::new()
                .title(format!("Help for {}", state.current_game().unwrap()))
                .borders(Borders::ALL),
        ),
        frame.size(),
    )
}

pub fn render(state: &mut State, frame: &mut Frame) {
    match state.active_screen {
        Screen::MainMenu => {
            let menu_window_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(65),
                    Constraint::Percentage(5),
                ])
                .split(frame.size());

            menu::render_header(state, menu_window_layout.get(0).unwrap(), frame);
            menu::render_game_chooser(state, menu_window_layout.get(1).unwrap(), frame);
            menu::render_footer(state, menu_window_layout.get(2).unwrap(), frame);
        }
        Screen::Game => {
            let game_window_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(2),
                    Constraint::Percentage(70),
                    Constraint::Percentage(28),
                ])
                .split(frame.size());

            game::render_header(state, game_window_layout.get(0).unwrap(), frame);
            game::render_reels(state, game_window_layout.get(1).unwrap(), frame);
            game::render_footer(state, game_window_layout.get(2).unwrap(), frame);

            if state.win > 0 {
                game::render_win_overlay(state, game_window_layout.get(1).unwrap(), frame);
            }
        }
        Screen::Help => {
            render_help(state, frame);
        }
    }
}
