// concurrency.rs - Benchmark concurrent logging patterns
use dlt_user::{DltContext, DltLogLevel, dlt_get_overflow_count};
use std::time::{Duration, Instant};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::thread;

pub struct ConcurrencyResult {
    pub test_name: String,
    pub num_threads: usize,
    #[allow(dead_code)]
    pub messages_per_thread: usize,
    pub duration: Duration,
    pub total_messages: u64,
    pub throughput: f64,
    pub messages_dropped: u64,
    #[allow(dead_code)]
    pub cpu_efficiency: f64, // throughput per thread
}

/// Benchmark single-threaded logging
pub fn bench_single_thread(num_messages: usize) -> ConcurrencyResult {
    println!("\n=== Benchmarking Single Thread ===");

    let start = Instant::now();
    let ctx = DltContext::new("SNGL", "TST", "Single Thread Bench", "Test");

    for i in 0..num_messages {
        let _ = ctx.log(DltLogLevel::Info, i as i32, "SingleThread");
    }

    let duration = start.elapsed();
    thread::sleep(Duration::from_millis(200));

    let dropped = dlt_get_overflow_count();

    ConcurrencyResult {
        test_name: "Single Thread".to_string(),
        num_threads: 1,
        messages_per_thread: num_messages,
        duration,
        total_messages: num_messages as u64,
        throughput: num_messages as f64 / duration.as_secs_f64(),
        messages_dropped: dropped,
        cpu_efficiency: num_messages as f64 / duration.as_secs_f64(),
    }
}

/// Benchmark multi-threaded logging with MPSC (lock-free)
pub fn bench_multi_thread_mpsc(num_threads: usize, messages_per_thread: usize) -> ConcurrencyResult {
    println!("\n=== Benchmarking {} Threads (MPSC Lock-Free) ===", num_threads);

    let start = Instant::now();
    let counter = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];
    for thread_id in 0..num_threads {
        let counter_clone = Arc::clone(&counter);
        
        handles.push(thread::spawn(move || {
            let ctx = DltContext::new("MPSC", "TST", "MPSC Bench", "Test");
            
            for i in 0..messages_per_thread {
                let _ = ctx.log(DltLogLevel::Info, i as i32, &format!("Thread{}", thread_id));
                counter_clone.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }    for handle in handles {
        let _ = handle.join();
    }

    let duration = start.elapsed();
    let total = counter.load(Ordering::Relaxed);

    thread::sleep(Duration::from_millis(500));

    let dropped = dlt_get_overflow_count();
    let throughput = total as f64 / duration.as_secs_f64();

    ConcurrencyResult {
        test_name: format!("{} Threads (Lock-Free)", num_threads),
        num_threads,
        messages_per_thread,
        duration,
        total_messages: total,
        throughput,
        messages_dropped: dropped,
        cpu_efficiency: throughput / num_threads as f64,
    }
}

/// Benchmark scalability: 1, 2, 4, 8, 16 threads
pub fn bench_thread_scalability(messages_per_thread: usize) -> Vec<ConcurrencyResult> {
    println!("\n=== Benchmarking Thread Scalability ===");

    let thread_counts = vec![1, 2, 4, 8, 16];
    let mut results = vec![];

    for num_threads in thread_counts {
        let result = bench_multi_thread_mpsc(num_threads, messages_per_thread);
        results.push(result);

        // Small delay between tests
        thread::sleep(Duration::from_millis(500));
    }

    results
}

/// Benchmark burst vs sustained load
pub fn bench_burst_vs_sustained() -> Vec<ConcurrencyResult> {
    println!("\n=== Benchmarking Burst vs Sustained Load ===");

    let mut results = vec![];

    // Burst: 10K messages as fast as possible
    println!("\nBurst load: 10K messages (no delay)...");
    let start = Instant::now();
    let ctx = DltContext::new("BRST", "TST", "Burst Bench", "Test");

    for i in 0..10000 {
        let _ = ctx.log(DltLogLevel::Info, i as i32, "Burst");
    }

    let duration = start.elapsed();
    thread::sleep(Duration::from_millis(200));

    results.push(ConcurrencyResult {
        test_name: "Burst (10K msgs)".to_string(),
        num_threads: 1,
        messages_per_thread: 10000,
        duration,
        total_messages: 10000,
        throughput: 10000.0 / duration.as_secs_f64(),
        messages_dropped: dlt_get_overflow_count(),
        cpu_efficiency: 10000.0 / duration.as_secs_f64(),
    });

    // Sustained: 10K messages with 1ms delay between each
    println!("\nSustained load: 10K messages (1ms delay)...");
    let start = Instant::now();
    let ctx = DltContext::new("SUST", "TST", "Sustained Bench", "Test");

    for i in 0..10000 {
        let _ = ctx.log(DltLogLevel::Info, i as i32, "Sustained");
        thread::sleep(Duration::from_micros(1000));
    }

    let duration = start.elapsed();
    thread::sleep(Duration::from_millis(200));

    results.push(ConcurrencyResult {
        test_name: "Sustained (10K msgs, 1ms delay)".to_string(),
        num_threads: 1,
        messages_per_thread: 10000,
        duration,
        total_messages: 10000,
        throughput: 10000.0 / duration.as_secs_f64(),
        messages_dropped: dlt_get_overflow_count(),
        cpu_efficiency: 10000.0 / duration.as_secs_f64(),
    });

    results
}

pub fn print_concurrency_results(results: &[ConcurrencyResult]) {
    println!("\n╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    CONCURRENCY BENCHMARK RESULTS                              ║");
    println!("╠═══════════════════════════════════════════════════════════════════════════════╣");
    println!("║ Test Name          │ Threads │ Duration │ Total Msgs │ Throughput │ Dropped ║");
    println!("╠═══════════════════════════════════════════════════════════════════════════════╣");

    for result in results {
        println!("║ {:<18} │ {:>7} │ {:>6.2}s │ {:>10} │ {:>8.0} /s │ {:>7} ║",
            result.test_name,
            result.num_threads,
            result.duration.as_secs_f64(),
            result.total_messages,
            result.throughput,
            result.messages_dropped
        );
    }

    println!("╚═══════════════════════════════════════════════════════════════════════════════╝");

    // Calculate scalability efficiency
    if results.len() > 1 {
        println!("\n=== Scalability Analysis ===");
        if let Some(baseline) = results.first() {
            for result in results.iter().skip(1) {
                let speedup = result.throughput / baseline.throughput;
                let efficiency = (speedup / result.num_threads as f64) * 100.0;
                println!("  {} threads: {:.2}x speedup, {:.1}% efficiency",
                    result.num_threads, speedup, efficiency);
            }
        }
    }

    // Show best performer
    if let Some(best) = results.iter().max_by(|a, b| a.throughput.partial_cmp(&b.throughput).unwrap()) {
        println!("\nHighest throughput: {} ({:.0} msg/s)",
            best.test_name, best.throughput);
    }
}
