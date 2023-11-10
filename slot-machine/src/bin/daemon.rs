use std::collections::HashMap;
use std::fs;

use std::io::{BufRead, Read};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ctrlc;

use itertools::Itertools;
use rand::Rng;
use slot_machine::par_table::{ParTable, ParTableFiles};
use slot_machine::protocol::{ClientCommand, ServerResponse, ServerStatus, Status};
use slot_machine::utils::send_socket_message;
use slot_machine::{GAMES_FOLDER, MAX_BYTES_READ, SOCKET_PATH};

const RATE_LIMIT_MS: u64 = 300;
const START_BALANCE: i64 = 100;

fn handle_client(mut stream: UnixStream, par_tables: Arc<HashMap<String, ParTable>>) {
    let client_uptime = Instant::now(); // TODO: Get server uptime rather than per-client
    println!("Accepted client: {:?}", stream);
    let mut balance = START_BALANCE;
    let reader = stream.try_clone().unwrap();
    let mut reader = std::io::BufReader::new(reader).take(MAX_BYTES_READ);
    let mut average_latency = 0.0;
    let mut status_query_count = 1;

    // TODO: First loop to wait for session init from client (e.g. set the game, verify identity to get balance (?))

    loop {
        println!("Waiting for next message...");
        let mut buf = String::new();
        let bytes_read = reader.read_line(&mut buf).expect("Could not read line");
        buf = buf.trim_end().to_string();
        println!("{:-<20}", "");
        println!("Buf: {:?}", buf);

        if bytes_read == 0 {
            break;
        } else if let Ok(client_command) = serde_json::from_str::<ClientCommand>(&buf) {
            println!("Parsed command: {:?}", client_command);
            match client_command {
                ClientCommand::Init { game } => {
                    send_socket_message(
                        &mut stream,
                        serde_json::to_string(&ServerResponse::Init {
                            balance: balance as u64,
                            max_bet: par_tables.get(&game).unwrap().max_bet,
                        })
                        .unwrap(),
                    );
                }
                ClientCommand::Play { game, bet } => {
                    if balance <= 0 {
                        balance = 0;
                        send_socket_message(
                            &mut stream,
                            serde_json::to_string(&ServerResponse::Error {
                                code: 1,
                                message: "Insufficent balance, thank you for playing !".to_string(),
                            })
                            .unwrap(),
                        );
                    } else if let Some(table) = par_tables.get(&game) {
                        // TODO: Validate game / bet input => Return corresponding errors to client
                        println!("Playing {} size bet on {}", bet, game);
                        let rng_iter =
                            rand::thread_rng().sample_iter(rand::distributions::Uniform::from(
                                0..=table.reels.len() - 1, // Account for indexes, start at 0
                            ));
                        let rng_result: Vec<usize> = rng_iter.take(3).collect();
                        let spin_result: Vec<u64> = rng_result
                            .iter()
                            .enumerate()
                            .map(|(reel, rng)| table.reels[*rng][reel])
                            .collect();
                        let (_, win) = table
                            .calculate_win(spin_result.clone(), bet)
                            .unwrap_or((spin_result.clone(), 0));

                        balance += win as i64 - (bet + 1) as i64;

                        println!(
                            "Spin: {:?}",
                            spin_result
                                .iter()
                                .map(|x| table.symbol_num_mapping.get(x).unwrap())
                                .join(" ")
                        );
                        println!("Win: {:+}", win);
                        println!("\nBalance: {:+}", balance);
                        println!("{:-<20}", "");

                        println!("Sending {:?}", rng_result);

                        send_socket_message(
                            &mut stream,
                            serde_json::to_string(&ServerResponse::Spin {
                                win,
                                balance: balance as u64,
                                result: rng_result,
                            })
                            .unwrap(),
                        );
                    }
                }
                ClientCommand::Status { clock } => {
                    average_latency += (clock.elapsed().unwrap().as_secs_f64() - average_latency)
                        / status_query_count as f64;
                    status_query_count += 1;
                    send_socket_message(
                        &mut stream,
                        serde_json::to_string(&ServerResponse::Status(Status {
                            server_status: ServerStatus::Connected,
                            uptime: client_uptime.elapsed(),
                            latency: Duration::from_secs_f64(average_latency),
                        }))
                        .unwrap(),
                    );
                }
            }
        } else {
            println!("Unrecognized client command: {:?}", buf);
        }

        // Rate limiting to prevent filling stream buffer to quickly (will still break after some time)
        thread::sleep(Duration::from_millis(RATE_LIMIT_MS));
    }

    println!("Client connection terminated!");
}

fn main() {
    let paths = fs::read_dir(GAMES_FOLDER).unwrap();
    let mut par_tables: HashMap<String, ParTable> = HashMap::new();

    for path in paths {
        let path = path.unwrap();
        println!("[x] Loading CSV files for {:?}...", path.path());
        let csv_files: Vec<String> = fs::read_dir(path.path())
            .unwrap()
            .map(|p| p.unwrap().path().display().to_string())
            .collect();

        let slot_machine = path.file_name().into_string().unwrap();
        let table = par_tables
            .entry(slot_machine.clone())
            .or_insert(ParTable::default());
        table
            .parse_from_csv(ParTableFiles::try_from(csv_files).unwrap())
            .expect("Failed to parse from csv");

        println!("[*] Loaded \"{}\"", slot_machine);
        println!("{}", table);
    }

    println!(
        "[+] Loaded {} tables for games: {:?}",
        par_tables.len(),
        par_tables.keys()
    );

    let _ = std::fs::remove_file(SOCKET_PATH);
    println!("Starting new listening socket on \"{}\"...", SOCKET_PATH);

    let listener = UnixListener::bind(SOCKET_PATH).unwrap();
    let clients: Vec<UnixStream> = vec![];

    let clients_handle = Arc::new(Mutex::new(clients));
    let clients_main_handle = clients_handle.clone();

    let run = Arc::new(Mutex::new(true));
    let run_handle = run.clone();

    let par_tables_arc = Arc::new(par_tables);

    ctrlc::set_handler(move || {
        clients_handle.lock().unwrap().iter().for_each(|client| {
            client
                .shutdown(std::net::Shutdown::Both)
                .expect("Could not shutdown client");
        });

        *run_handle.lock().unwrap() = false;
    })
    .expect("Error setting Ctrl-C handler");

    listener
        .set_nonblocking(true)
        .expect("Could not set non-blocking mode");
    while *run.lock().unwrap() {
        match listener.accept() {
            Ok((stream, _)) => {
                clients_main_handle
                    .lock()
                    .unwrap()
                    .push(stream.try_clone().expect("Could not clone client stream"));
                let par_tables_handle = par_tables_arc.clone();
                thread::spawn(move || handle_client(stream, par_tables_handle));
            }

            Err(err) => {
                if err.kind() != std::io::ErrorKind::WouldBlock {
                    ()
                }
            }
        }
    }
}
