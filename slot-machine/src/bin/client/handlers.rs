use std::{
    io::{BufRead, Read, Write},
    os::unix::net::UnixStream,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use slot_machine::MAX_BYTES_READ;

use crate::app::ANIMATION_WAIT_TIME;

#[derive(Clone, Debug)]
pub enum Event {
    Noop,
    Tick,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    SpinResult(Vec<isize>, u64, u64),
    ServerError(String),
}

#[derive(Debug)]
pub struct EventHandler {
    #[allow(dead_code)]
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,
    #[allow(dead_code)]
    handler: thread::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (sender, receiver) = mpsc::channel();
        let handler = {
            let sender = sender.clone();
            let _sender = sender.clone();
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
            })
        };
        Self {
            sender,
            receiver,
            handler,
        }
    }

    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.try_recv().unwrap_or(Event::Noop))
    }
}

#[derive(Debug)]
pub struct ClientHandler {
    #[allow(dead_code)]
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,
    #[allow(dead_code)]
    handler: thread::JoinHandle<()>,
    stream: UnixStream,
}

impl ClientHandler {
    pub fn new(stream: UnixStream) -> Self {
        let (sender, receiver) = mpsc::channel();
        let handler = {
            let sender = sender.clone();
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

                    /*
                    SPIN {WIN} {BALANCE} {R1} {R2} ... {RN}
                    ERRX {MSG}
                    */

                    if bytes_read == 0 {
                        // TODO: Think about how to deal with socket errors (retries ? where to handle ?)
                        break;
                    } else {
                        match &response[0..4] {
                            "SPIN" => {
                                let v = response[5..].split(' ').collect::<Vec<_>>();
                                let (win, balance) = (
                                    v.get(0).expect("Win not present"),
                                    v.get(1).expect("Balance not present"),
                                );

                                let spin_result = v
                                    .get(2..)
                                    .expect("Reel not present")
                                    .to_vec()
                                    .iter()
                                    .map(|s| str::parse::<isize>(s).unwrap())
                                    .collect::<Vec<_>>();

                                thread::sleep(ANIMATION_WAIT_TIME); // Make spin animation go for a certain time :)

                                let _ = sender.send(Event::SpinResult(
                                    spin_result,
                                    str::parse::<u64>(&win).unwrap(),
                                    str::parse::<u64>(&balance).unwrap(),
                                ));

                                // TODO: Pass down spin_result, win, balance to main thread
                            }
                            "ERR1" => {
                                let err = response[5..].to_string();
                                eprintln!("{}", err);
                                let _ = sender.send(Event::ServerError(err));
                            }
                            _ => (),
                        }
                    }
                }

                _stream
                    .shutdown(std::net::Shutdown::Both)
                    .expect("Could not shutdown client stream");
            })
        };
        Self {
            sender,
            receiver,
            handler,
            stream,
        }
    }

    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.try_recv().unwrap_or(Event::Noop))
    }

    pub fn send_spin_message(&mut self, game_choice: String, bet: u64) {
        writeln!(self.stream, "PLAY {} {}", game_choice, bet)
            .expect("Could not send message to server");
        self.stream.flush().expect("Could not flush");
    }
}
