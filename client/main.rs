// dlt-receive: example binary using client library
use dlt_client::{DltClient, parse_message_text};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut host = "127.0.0.1".to_string();

    // Parse arguments: -a <address>
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-a" => {
                if i + 1 < args.len() {
                    host = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    println!("Connecting to DLT daemon at {}:3490", host);

    let mut client = DltClient::connect(&host, 3490)
        .expect("Failed to connect to daemon");

    println!("Connected. Receiving logs...\n");

    loop {
        match client.receive_messages() {
            Ok(messages) => {
                if messages.is_empty() {
                    println!("Connection closed");
                    break;
                }
                for msg in messages {
                    let output = parse_message_text(&msg);
                    println!("{}", output);
                }
            }
            Err(e) => {
                eprintln!("Error reading: {}", e);
                break;
            }
        }
    }
}
