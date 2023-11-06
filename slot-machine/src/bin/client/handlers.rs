use std::{
    io::{BufRead, Read},
    os::unix::net::UnixStream,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use slot_machine::{
    protocol::{ClientCommand, ServerResponse},
    utils::send_socket_message,
    MAX_BYTES_READ,
};

#[derive(Clone, Debug)]
pub enum Event {
    Noop,
    Tick,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

#[derive(Debug)]
pub struct EventHandler {
    receiver: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(tick_rate);

                if event::poll(timeout).expect("no events available") {
                    match event::read().expect("unable to read event") {
                        CrosstermEvent::Key(e) => {
                            if e.kind == event::KeyEventKind::Press {
                                sender.send(Event::Key(e))
                            } else {
                                Ok(()) // ignore KeyEventKind::Release on windows
                            }
                        }
                        CrosstermEvent::Mouse(e) => sender.send(Event::Mouse(e)),
                        CrosstermEvent::Resize(w, h) => sender.send(Event::Resize(w, h)),
                        _ => unimplemented!(),
                    }
                    .expect("failed to send terminal event")
                }

                if last_tick.elapsed() >= tick_rate {
                    sender.send(Event::Tick).expect("failed to send tick event");
                    last_tick = Instant::now();
                }
            }
        });
        Self { receiver }
    }

    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.try_recv().unwrap_or(Event::Noop))
    }
}

// Analog to `ServerResponse` with the addition of `Noop` and type conversions
#[derive(Clone, Debug)]
pub enum Stream {
    Noop,
    Balance(u64),
    SpinResult(Vec<isize>, u64, u64),
    ServerError(String),
}

#[derive(Debug)]
pub struct StreamHandler {
    receiver: mpsc::Receiver<Stream>,
    stream: UnixStream,
}

impl StreamHandler {
    pub fn new(stream: UnixStream) -> Self {
        let (sender, receiver) = mpsc::channel();
        let _stream = stream.try_clone().expect("Could not clone client socket");
        thread::spawn(move || {
            let reader = _stream.try_clone().unwrap();
            let mut reader = std::io::BufReader::new(reader).take(MAX_BYTES_READ);

            loop {
                let mut response = String::new();
                let bytes_read = reader
                    .read_line(&mut response)
                    .expect("Could not read from buffer");

                response = response.trim_end().to_string();

                if bytes_read == 0 {
                    // TODO: Think about how to deal with client socket errors (e.g. restart thread, server ping)
                    break;
                } else if let Ok(server_command) = serde_json::from_str::<ServerResponse>(&response)
                {
                    match server_command {
                        ServerResponse::Balance { balance } => {
                            let _ = sender.send(Stream::Balance(balance));
                        }
                        ServerResponse::Spin {
                            win,
                            balance,
                            result,
                        } => {
                            let _ = sender.send(Stream::SpinResult(
                                result.iter().map(|r| *r as isize).collect(),
                                win,
                                balance,
                            ));
                        }
                        ServerResponse::Error { code, message } => {
                            eprintln!("ERR{}: {}", code, message);
                            let _ = sender.send(Stream::ServerError(message));
                        }
                    }
                }
            }

            _stream
                .shutdown(std::net::Shutdown::Both)
                .expect("Could not shutdown client stream");
        });
        Self { receiver, stream }
    }

    pub fn next(&self) -> Result<Stream> {
        Ok(self.receiver.try_recv().unwrap_or(Stream::Noop))
    }

    pub fn send_spin_message(&mut self, game: String, bet: u64) {
        send_socket_message(
            &mut self.stream,
            serde_json::to_string(&ClientCommand::Play {
                game,
                bet: bet.saturating_sub(1) as usize,
            })
            .unwrap(),
        );
    }

    pub fn send_balance_message(&mut self) {
        send_socket_message(
            &mut self.stream,
            serde_json::to_string(&ClientCommand::Balance).unwrap(),
        );
    }
}
