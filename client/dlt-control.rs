// Example: dlt-control tool skeleton
// This demonstrates how to build control tools using the dlt-client library

use dlt_client::DltClient;

fn main() {
    println!("DLT Control Tool (skeleton)");

    let mut client = match DltClient::connect("127.0.0.1", 3490) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            return;
        }
    };

    println!("Connected to DLT daemon");

    // Example: Send control message
    let control_msg = b"CONTROL_MESSAGE_EXAMPLE";
    match client.send_control_message(control_msg) {
        Ok(_) => println!("Control message sent"),
        Err(e) => eprintln!("Failed to send control message: {}", e),
    }

    // Can also receive responses
    match client.receive_message() {
        Ok(Some(msg)) => {
            println!("Received response: {:?}", msg.payload);
        }
        Ok(None) => println!("Connection closed"),
        Err(e) => eprintln!("Error: {}", e),
    }
}
