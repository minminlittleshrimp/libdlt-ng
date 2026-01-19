// dlt-example-user: example binary using user library
// Rust equivalent of dlt-daemon/src/examples/dlt-example-user.c
use dlt_user::{DltContext, DltLogLevel, dlt_enable_local_print};
use std::env;

fn usage() {
    println!("Usage: dlt-example-user [options] message");
    println!("Generate DLT messages and send them to daemon.");
    println!("Options:");
    println!("  -d delay      Milliseconds to wait between sending messages (Default: 500)");
    println!("  -n count      Number of messages to be generated (Default: 10)");
    println!("  -a            Enable local printing of DLT messages (Default: disabled)");
    println!("  -l level      Set log level (1=Fatal, 2=Error, 3=Warn, 4=Info, 5=Debug, 6=Verbose) (Default: 3=Warn)");
    println!("  -A AppID      Set app ID for send message (Default: LOG)");
    println!("  -C ContextID  Set context ID for send message (Default: TEST)");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut num_messages = 10;
    let mut message = String::new();
    let mut delay = 500;
    let mut aflag = false;
    let mut level = DltLogLevel::Warn;
    let mut app_id = "LOG";
    let mut context_id = "TEST";

    // Parse arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-d" => {
                if i + 1 < args.len() {
                    delay = args[i + 1].parse().unwrap_or(500);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-n" => {
                if i + 1 < args.len() {
                    num_messages = args[i + 1].parse().unwrap_or(10);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-a" => {
                aflag = true;
                i += 1;
            }
            "-l" => {
                if i + 1 < args.len() {
                    let lval: i32 = args[i + 1].parse().unwrap_or(3);
                    level = match lval {
                        1 => DltLogLevel::Fatal,
                        2 => DltLogLevel::Error,
                        3 => DltLogLevel::Warn,
                        4 => DltLogLevel::Info,
                        5 => DltLogLevel::Debug,
                        6 => DltLogLevel::Verbose,
                        _ => DltLogLevel::Warn,
                    };
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-A" => {
                if i + 1 < args.len() {
                    app_id = Box::leak(args[i + 1].clone().into_boxed_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-C" => {
                if i + 1 < args.len() {
                    context_id = Box::leak(args[i + 1].clone().into_boxed_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "-h" | "--help" => {
                usage();
                return;
            }
            _ => {
                message = args[i].clone();
                i += 1;
            }
        }
    }

    if message.is_empty() {
        eprintln!("ERROR: No message selected");
        usage();
        std::process::exit(1);
    }

    if aflag {
        dlt_enable_local_print();
    }

    // Register app and context (equivalent to DLT_REGISTER_APP + DLT_REGISTER_CONTEXT)
    let ctx = DltContext::new(app_id, context_id, "Test Application for Logging", "Test Context for Logging");

    // Send log messages (equivalent to DLT_LOG loop in C version)
    if let Err(e) = ctx.log_multiple(&message, num_messages, delay as u64, level) {
        eprintln!("Failed to send logs: {}", e);
    }

    // Sleep before exit to ensure all messages are sent
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Print buffer statistics before exit
    println!("\n=== Final Buffer Statistics ===");
    dlt_user::dlt_print_buffer_stats();
    let dropped = dlt_user::dlt_get_overflow_count();
    if dropped > 0 {
        eprintln!("\nWARNING: {} messages were dropped due to buffer overflow!", dropped);
    }

    // Auto-unregisters on drop (equivalent to DLT_UNREGISTER_CONTEXT + DLT_UNREGISTER_APP)
}
