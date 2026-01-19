// overflow_modes.rs - Benchmark different overflow handling modes
use dlt_user::{DltContext, DltLogLevel, dlt_set_overflow_mode, dlt_get_overflow_count};
use std::time::{Duration, Instant};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::thread;

pub struct OverflowModeResult {
    pub mode_name: String,
    pub duration: Duration,
    pub messages_sent: u64,
    pub messages_dropped: u64,
    pub throughput: f64,
    #[allow(dead_code)]
    pub cpu_percent: f64,
}

/// Benchmark Drop mode (mode 1)
pub fn bench_drop_mode(num_messages: usize, num_threads: usize) -> OverflowModeResult {
    println!("\n=== Benchmarking Drop Mode (DropNewest) ===");

    // Set mode before creating context
    dlt_set_overflow_mode(1);

    let start = Instant::now();
    let counter = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(AtomicBool::new(false));

    // Spawn producer threads
    let mut handles = vec![];
    for _thread_id in 0..num_threads {
        let counter_clone = Arc::clone(&counter);
        let stop_clone = Arc::clone(&stop);

        handles.push(thread::spawn(move || {
            let ctx = DltContext::new("DRPB", "TST", "Drop Bench", "Test");
            let mut local_count = 0;

            while local_count < num_messages / num_threads && !stop_clone.load(Ordering::Relaxed) {
                let _ = ctx.log(DltLogLevel::Info, local_count as i32, "Benchmark");
                local_count += 1;
                counter_clone.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Wait for completion
    for handle in handles {
        let _ = handle.join();
    }

    let duration = start.elapsed();
    let sent = counter.load(Ordering::Relaxed);
    let dropped = dlt_get_overflow_count();

    // Give time for background workers to flush
    thread::sleep(Duration::from_millis(500));

    let throughput = sent as f64 / duration.as_secs_f64();

    OverflowModeResult {
        mode_name: "DropNewest".to_string(),
        duration,
        messages_sent: sent,
        messages_dropped: dropped,
        throughput,
        cpu_percent: 0.0, // Will be measured separately
    }
}

/// Benchmark Overwrite mode (mode 0)
pub fn bench_overwrite_mode(num_messages: usize, num_threads: usize) -> OverflowModeResult {
    println!("\n=== Benchmarking Overwrite Mode ===");

    dlt_set_overflow_mode(0);

    let start = Instant::now();
    let counter = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];
    for _thread_id in 0..num_threads {
        let counter_clone = Arc::clone(&counter);

        handles.push(thread::spawn(move || {
            let ctx = DltContext::new("OWRB", "TST", "Overwrite Bench", "Test");
            let mut local_count = 0;

            while local_count < num_messages / num_threads {
                let _ = ctx.log(DltLogLevel::Info, local_count as i32, "Benchmark");
                local_count += 1;
                counter_clone.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for handle in handles {
        let _ = handle.join();
    }

    let duration = start.elapsed();
    let sent = counter.load(Ordering::Relaxed);
    let dropped = dlt_get_overflow_count();

    thread::sleep(Duration::from_millis(500));

    let throughput = sent as f64 / duration.as_secs_f64();

    OverflowModeResult {
        mode_name: "Overwrite".to_string(),
        duration,
        messages_sent: sent,
        messages_dropped: dropped,
        throughput,
        cpu_percent: 0.0,
    }
}

/// Benchmark Timeout mode (mode 2)
pub fn bench_timeout_mode(num_messages: usize, num_threads: usize) -> OverflowModeResult {
    println!("\n=== Benchmarking Timeout Mode (BlockWithTimeout) ===");

    dlt_set_overflow_mode(2);

    let start = Instant::now();
    let counter = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];
    for _thread_id in 0..num_threads {
        let counter_clone = Arc::clone(&counter);

        handles.push(thread::spawn(move || {
            let ctx = DltContext::new("TIMB", "TST", "Timeout Bench", "Test");
            let mut local_count = 0;

            while local_count < num_messages / num_threads {
                let _ = ctx.log(DltLogLevel::Info, local_count as i32, "Benchmark");
                local_count += 1;
                counter_clone.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for handle in handles {
        let _ = handle.join();
    }

    let duration = start.elapsed();
    let sent = counter.load(Ordering::Relaxed);
    let dropped = dlt_get_overflow_count();

    thread::sleep(Duration::from_millis(500));

    let throughput = sent as f64 / duration.as_secs_f64();

    OverflowModeResult {
        mode_name: "BlockWithTimeout".to_string(),
        duration,
        messages_sent: sent,
        messages_dropped: dropped,
        throughput,
        cpu_percent: 0.0,
    }
}

pub fn print_overflow_results(results: &[OverflowModeResult]) {
    println!("\n╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║              OVERFLOW MODE BENCHMARK RESULTS                          ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║ Mode              │ Duration │ Sent      │ Dropped │ Throughput       ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");

    for result in results {
        println!("║ {:<17} │ {:>6.2}s │ {:>9} │ {:>7} │ {:>10.0} msg/s ║",
            result.mode_name,
            result.duration.as_secs_f64(),
            result.messages_sent,
            result.messages_dropped,
            result.throughput
        );
    }

    println!("╚═══════════════════════════════════════════════════════════════════════╝");

    // Find best performer
    if let Some(best) = results.iter().max_by(|a, b| a.throughput.partial_cmp(&b.throughput).unwrap()) {
        println!("\n✓ Best throughput: {} ({:.0} msg/s)", best.mode_name, best.throughput);
    }

    // Check for drops
    let total_dropped: u64 = results.iter().map(|r| r.messages_dropped).sum();
    if total_dropped > 0 {
        println!("Total messages dropped: {}", total_dropped);
    } else {
        println!("No messages dropped across all modes");
    }
}
