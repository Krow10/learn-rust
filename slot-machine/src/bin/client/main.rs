/* TODO:
- Require `gameinfo.json` for each game describing the name, author, version, help message, game color scheme, etc.
- Logging facility for debug and error messages
- Add sound and more visual effects for better engagment
- Better handling of help screen (multiple key combinations, ESC / q capture (?))
- Handle buffered input (i.e. continuously pressed keys)
- Refactor animations and screens for easier customization
- Show balance status on game chooser (?)
*/

use std::os::unix::net::UnixStream;

use std::time::Instant;

use app::SERVER_PING_TIMEOUT;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use slot_machine::SOCKET_PATH;

use crate::app::App;

use crate::app::EVENT_POLL_INTERVAL_MS;
use crate::app::FRAMES_PER_SECONDS;
use crate::handlers::Event;
use crate::handlers::EventHandler;
use crate::handlers::Stream;
use crate::handlers::StreamHandler;

use crate::updates::{update_keys, update_spin};
use anyhow::Result;

mod app;
mod handlers;
mod ui;
mod updates;

pub const JSON_SYMBOLS_FILE: &str = "./data/display_symbols.json";

fn main() -> Result<()> {
    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;

    let events = EventHandler::new(EVENT_POLL_INTERVAL_MS);

    let stream = UnixStream::connect(SOCKET_PATH).unwrap();
    let client = StreamHandler::new(stream);

    let mut app = App::new(terminal, events, client);
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
            Stream::Init(balance, max_bet) => {
                app.state.balance = balance;
                app.state.next_balance = balance;
                app.state.max_bet = max_bet;
                app.state.bet = max_bet;
            }
            Stream::SpinResult(spin, win, balance) => update_spin(&mut app, spin, win, balance),
            Stream::ServerError(_err) => {}
            Stream::Status(status) => {
                app.state.daemon_status = status;
            }
        }
    }

    app.exit()?;
    Ok(())
}
