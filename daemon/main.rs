// daemon: DLT daemon with lockless buffer (decoupled design)
use std::os::unix::net::UnixListener;
use std::net::TcpListener;
use std::io::{Read, Write};
use std::thread;
use dlt_buffer::LocklessBuffer;

const DLT_DAEMON_SOCKET: &str = "/tmp/dlt";
const DLT_DAEMON_PORT: &str = "127.0.0.1:3490";

fn main() {
    // Remove old socket if exists
    let _ = std::fs::remove_file(DLT_DAEMON_SOCKET);

    // Create lockless buffer for logs
    let log_buffer: LocklessBuffer<Vec<u8>> = LocklessBuffer::new(1024);

    // Spawn Unix socket listener (receives logs from users)
    let buffer_for_unix = log_buffer.clone();
    thread::spawn(move || {
        let listener = UnixListener::bind(DLT_DAEMON_SOCKET).unwrap();
        println!("DLT daemon listening on {}", DLT_DAEMON_SOCKET);

        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };

            let buffer = buffer_for_unix.clone();
            thread::spawn(move || {
                let mut buf = vec![0u8; 4096];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break, // Connection closed
                        Ok(n) => {
                            let msg = buf[..n].to_vec();
                            let _ = buffer.push(msg);
                        }
                        Err(_) => break,
                    }
                }
            });
        }
    });

    // TCP listener for clients (dlt-receive)
    let listener = TcpListener::bind(DLT_DAEMON_PORT).unwrap();
    println!("DLT daemon serving on {}", DLT_DAEMON_PORT);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let buffer = log_buffer.clone();
        thread::spawn(move || {
            loop {
                if let Some(log_msg) = buffer.pop() {
                    if stream.write_all(&log_msg).is_err() {
                        break;
                    }
                } else {
                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });
    }
}
