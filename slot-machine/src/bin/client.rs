use std::collections::HashMap;
use std::fs;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;

use slot_machine::utils::{clear_screen, get_user_input};

fn main() {
    let paths: Vec<String> = fs::read_dir("./data")
        .unwrap()
        .map(|p| p.unwrap().file_name().into_string().unwrap())
        .collect();

    println!("[x] Select a game:");
    paths.iter().enumerate().for_each(|(i, p)| {
        println!("{}. {}", i + 1, p);
    });

    let game_choice = &paths[get_user_input().unwrap().parse::<usize>().unwrap() - 1];
    println!("[+] Playing \"{}\" !", game_choice);

    println!("[*] Loading game symbols...");
    let mut rdr = csv::Reader::from_path(format!("./data/{}/display.csv", game_choice)).unwrap();
    let symbols_mapping: HashMap<String, String> = HashMap::from_iter(rdr.deserialize().map(|r| {
        let (symbol, display): (String, String) = r.unwrap();
        (symbol, display)
    }));

    println!("[+] Game symbols loaded !");

    println!("Enter bet amount:");
    let bet = get_user_input().unwrap().parse::<usize>().unwrap();

    let mut stream = UnixStream::connect("/tmp/slot_machine.sock").unwrap();
    let reader = stream.try_clone().unwrap();
    let mut reader = std::io::BufReader::new(reader).take(4096);

    loop {
        println!("Enter any input to start a spin!");
        get_user_input();
        clear_screen();

        writeln!(stream, "PLAY {} {}", game_choice, bet).expect("Could not send message to server");
        stream.flush().expect("Could not flush");

        let mut response = String::new();
        let bytes_read = reader
            .read_line(&mut response)
            .expect("Could not read from buffer");

        response = response.trim_end().to_string();

        println!("Server response: {:?}", response);

        /*
            SPIN {WIN} {BALANCE} {R1} {R2} ... {RN}
            ERRX {MSG}
        */

        if bytes_read == 0 {
            break;
        } else {
            match &response[0..4] {
                "SPIN" => {
                    let v = response[5..].split(' ').collect::<Vec<_>>();
                    let (win, balance) = (
                        v.get(0).expect("Win not present"),
                        v.get(1).expect("Balance not present"),
                    );

                    let spin_result = v.get(2..).expect("Reel not present");

                    println!("{:=<14}", "");
                    spin_result
                        .iter()
                        .for_each(|s| print!("{} | ", symbols_mapping.get(*s).unwrap()));
                    println!("\n{:=<14}", "");
                    println!("Win: {:+} | Balance: {}\n", win, balance);
                }
                "ERR1" => {
                    println!("{}", response[5..].to_string());
                    break;
                }
                _ => (),
            }
        }
    }

    stream
        .shutdown(std::net::Shutdown::Both)
        .expect("Could not shutdown client stream");
}
