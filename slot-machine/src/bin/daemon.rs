use std::collections::HashMap;
use std::fs;

use std::io::{BufRead, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ctrlc;

use itertools::Itertools;
use rand::Rng;
use slot_machine::par_table::{ParTable, ParTableFiles};
use slot_machine::{MAX_BYTES_READ, SOCKET_PATH};

const RATE_LIMIT_MS: u64 = 300;
const START_BALANCE: i64 = 100;

fn handle_client(mut stream: UnixStream, par_tables: Arc<HashMap<String, ParTable>>) {
    println!("Accepted client: {:?}", stream);
    let mut balance = START_BALANCE;
    let reader = stream.try_clone().unwrap();
    let mut reader = std::io::BufReader::new(reader).take(MAX_BYTES_READ);

    loop {
        println!("Waiting for next message...");
        let mut buf = String::new();
        let bytes_read = reader.read_line(&mut buf).expect("Could not read line");
        buf = buf.trim_end().to_string();
        println!("{:-<20}", "");
        println!("Command: {:?}", buf);

        /*
            PLAY {NAME} {BET}
        */

        if bytes_read == 0 {
            break;
        } else {
            match &buf[..4] {
                "PLAY" => {
                    if let Some((slot_machine, bet)) = buf[5..].split_once(' ') {
                        let bet = bet.parse::<usize>().unwrap_or(1); // Pick lowest bet on parse error
                        if let Some(table) = par_tables.get(slot_machine) {
                            println!("Playing {} size bet on {}", bet, slot_machine);
                            let rng_iter =
                                rand::thread_rng().sample_iter(rand::distributions::Uniform::from(
                                    0..=table.reels.len() - 1, // Account for indexes, start at 0
                                ));
                            let rng_result: Vec<usize> = rng_iter.take(3).collect(); // In real-life applications the numbers are being constantly re-generated and picked just on input
                            let spin_result: Vec<u64> = rng_result
                                .iter()
                                .enumerate()
                                .map(|(reel, rng)| table.reels[*rng][reel])
                                .collect();
                            let (_, win) = table
                                .calculate_win(spin_result.clone(), bet - 1)
                                .unwrap_or((spin_result.clone(), 0));

                            balance += win as i64 - bet as i64;

                            if balance < 0 {
                                balance = 0;
                                writeln!(
                                    stream,
                                    "ERR1 Insufficent balance, thank you for playing !"
                                )
                                .expect("Could not write to client");
                                stream.flush().expect("Could not flush");
                            }

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

                            writeln!(
                                stream,
                                "SPIN {} {} {}",
                                win,
                                balance,
                                spin_result
                                    .iter()
                                    .map(|x| { table.symbol_num_mapping.get(x).unwrap() })
                                    .join(" ")
                            )
                            .expect("Could not write to client");
                            stream.flush().expect("Could not flush");
                        }
                    }
                }
                _ => (),
            }
        }

        thread::sleep(Duration::from_millis(RATE_LIMIT_MS)); // Rate limiting to prevent filling stream buffer to quickly (will still break after some time)
    }

    println!("Client connection terminated!");
}

fn main() {
    let paths = fs::read_dir("./data").unwrap();
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
