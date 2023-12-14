use cfonts::{Fonts, Options};
use ratatui::{
    prelude::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use slot_machine::{built_info, protocol::ServerStatus};

use crate::app::{State, CFONTS_TEXT_COLORS, PRIMARY_TEXT_COLOR, SECONDARY_TEXT_COLOR};

use super::widgets::{AlignCenter, CFontTextWidget};

pub fn render_header(state: &mut State, layout: &Rect, frame: &mut Frame) {
    let title_text_option = Options {
        font: Fonts::Font3d,
        colors: CFONTS_TEXT_COLORS.to_vec(),
        spaceless: true,
        ..Options::default()
    };

    let title_text_widget = CFontTextWidget::default()
        .options(Options {
            text: "SLOTS".to_string(),
            ..title_text_option
        })
        .align_center(AlignCenter::Both)
        .animated_bold_line(state.title_text_bold_line);

    frame.render_widget(title_text_widget, *layout);
}

pub fn render_game_chooser(state: &mut State, layout: &Rect, frame: &mut Frame) {
    frame.render_stateful_widget(
        List::new(
            state
                .available_games
                .iter()
                .map(|s| ListItem::new(s.to_string()))
                .collect::<Vec<_>>(),
        )
        .block(
            Block::default()
                .title("Available games")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(
                    SECONDARY_TEXT_COLOR[0],
                    SECONDARY_TEXT_COLOR[1],
                    SECONDARY_TEXT_COLOR[2],
                ))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> "),
        *layout,
        &mut state.selected_game,
    );
}

pub fn render_footer(state: &State, layout: &Rect, frame: &mut Frame) {
    let footer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(*layout);

    let default_style = Style::new().fg(Color::Rgb(
        PRIMARY_TEXT_COLOR[0],
        PRIMARY_TEXT_COLOR[1],
        PRIMARY_TEXT_COLOR[2],
    ));
    let daemon_status_style = Style::new().italic();
    let daemon_status_style = match state.daemon_status.server_status {
        ServerStatus::Stopped => daemon_status_style.red(),
        ServerStatus::Disconnected => daemon_status_style.yellow(),
        ServerStatus::Connected => daemon_status_style.green(),
    };

    let uptime_seconds = state.daemon_status.uptime.as_secs();
    let uptime_minutes = uptime_seconds / 60;
    let uptime_hours = uptime_minutes / 60;
    let mut uptime_format = format!(
        "{}s",
        uptime_seconds - uptime_minutes * 60 - uptime_hours * 3600
    );

    if uptime_minutes > 0 {
        uptime_format = format!(
            "{}m {}",
            uptime_minutes - uptime_hours * 3600,
            uptime_format
        );
    }

    if uptime_hours > 0 {
        uptime_format = format!("{}h {}", uptime_hours, uptime_format);
    }

    let status_text = Line::from(vec![
        Span::styled("status : ", default_style),
        Span::styled(
            format!("{:?} ", state.daemon_status.server_status),
            daemon_status_style,
        ),
        Span::styled(
            format!(
                "- uptime {} - ping {}.{:0>6}ms",
                uptime_format,
                state.daemon_status.latency.as_millis(),
                state.daemon_status.latency.as_nanos()
            ),
            default_style,
        ),
    ]);

    let centered_status_layout = *footer_layout.get(0).unwrap();
    frame.render_widget(
        Paragraph::new(status_text).alignment(Alignment::Center),
        // Cannot use `inner()` method as the current implementation subtracts double the margin from the width/height
        Rect {
            x: centered_status_layout.x,
            y: centered_status_layout
                .y
                .saturating_add(centered_status_layout.height.saturating_sub(1) / 2),
            width: centered_status_layout.width,
            height: centered_status_layout
                .height
                .saturating_sub(centered_status_layout.height / 2),
        },
    );

    let build_text_widget = CFontTextWidget::default()
        .options(Options {
            text: format!(
                "client: {} v{} (commit {}) on {}",
                built_info::TARGET,
                built_info::PKG_VERSION,
                built_info::GIT_VERSION.unwrap_or("<unknown>"),
                built_info::BUILT_TIME_UTC
            ),
            font: Fonts::FontConsole,
            colors: CFONTS_TEXT_COLORS.to_vec(),
            spaceless: true,
            ..Options::default()
        })
        .align_center(AlignCenter::Both);
    frame.render_widget(build_text_widget, *footer_layout.get(1).unwrap());
}
