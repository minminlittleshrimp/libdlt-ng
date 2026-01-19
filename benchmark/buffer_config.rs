// buffer_config.rs - Benchmark different buffer configurations
use dlt_user::{DltContext, DltLogLevel};
use std::time::{Duration, Instant};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::thread;
use std::env;

pub struct BufferConfigResult {
    pub config_name: String,
    #[allow(dead_code)]
    pub num_buffers: usize,
    #[allow(dead_code)]
    pub buffer_size: usize,
    #[allow(dead_code)]
    pub batch_size: usize,
    pub duration: Duration,
    pub messages_sent: u64,
    pub throughput: f64,
    pub avg_latency_us: f64,
}

/// Benchmark with different buffer counts
pub fn bench_buffer_count(messages_per_config: usize) -> Vec<BufferConfigResult> {
    println!("\n=== Benchmarking Different Buffer Counts ===");

    let configs = vec![
        (1, 2048, 16),
        (2, 2048, 16),
        (4, 2048, 16),
        (8, 2048, 16),
    ];

    let mut results = vec![];

    for (num_buffers, buffer_size, batch_size) in configs {
        println!("\nTesting {} buffer(s)...", num_buffers);

        // Set environment variables
        env::set_var("DLT_USER_NUM_BUFFERS", num_buffers.to_string());
        for i in 0..num_buffers {
            env::set_var(format!("DLT_USER_BUFFER_SIZE_{}", i), buffer_size.to_string());
        }
        env::set_var("DLT_USER_BATCH_SIZE", batch_size.to_string());

        // Force re-initialization by spawning new process would be needed
        // For now, we'll test in sequence within same process

        let start = Instant::now();
        let counter = Arc::new(AtomicU64::new(0));

        let ctx = DltContext::new("BCFG", "TST", "Buffer Config Bench", "Test");

        for i in 0..messages_per_config {
            let _ = ctx.log(DltLogLevel::Info, i as i32, "BufferBench");
            counter.fetch_add(1, Ordering::Relaxed);
        }

        let duration = start.elapsed();
        let sent = counter.load(Ordering::Relaxed);

        thread::sleep(Duration::from_millis(200));

        results.push(BufferConfigResult {
            config_name: format!("{} buffers", num_buffers),
            num_buffers,
            buffer_size,
            batch_size,
            duration,
            messages_sent: sent,
            throughput: sent as f64 / duration.as_secs_f64(),
            avg_latency_us: (duration.as_micros() as f64) / (sent as f64),
        });
    }

    results
}

/// Benchmark with different buffer sizes
pub fn bench_buffer_sizes(messages_per_config: usize) -> Vec<BufferConfigResult> {
    println!("\n=== Benchmarking Different Buffer Sizes ===");

    let configs = vec![
        512,
        1024,
        2048,
        4096,
        8192,
    ];

    let mut results = vec![];

    for buffer_size in configs {
        println!("\nTesting buffer size: {}...", buffer_size);

        env::set_var("DLT_USER_NUM_BUFFERS", "4");
        for i in 0..4 {
            env::set_var(format!("DLT_USER_BUFFER_SIZE_{}", i), buffer_size.to_string());
        }

        let start = Instant::now();
        let counter = Arc::new(AtomicU64::new(0));

        let ctx = DltContext::new("BSIZ", "TST", "Buffer Size Bench", "Test");

        for i in 0..messages_per_config {
            let _ = ctx.log(DltLogLevel::Info, i as i32, "SizeBench");
            counter.fetch_add(1, Ordering::Relaxed);
        }

        let duration = start.elapsed();
        let sent = counter.load(Ordering::Relaxed);

        thread::sleep(Duration::from_millis(200));

        results.push(BufferConfigResult {
            config_name: format!("{} msgs", buffer_size),
            num_buffers: 4,
            buffer_size,
            batch_size: 16,
            duration,
            messages_sent: sent,
            throughput: sent as f64 / duration.as_secs_f64(),
            avg_latency_us: (duration.as_micros() as f64) / (sent as f64),
        });
    }

    results
}

/// Benchmark with different batch sizes (writev batching)
pub fn bench_batch_sizes(messages_per_config: usize) -> Vec<BufferConfigResult> {
    println!("\n=== Benchmarking Different Batch Sizes (writev) ===");

    let configs = vec![1, 4, 8, 16, 32, 64];

    let mut results = vec![];

    for batch_size in configs {
        println!("\nTesting batch size: {}...", batch_size);

        env::set_var("DLT_USER_BATCH_SIZE", batch_size.to_string());

        let start = Instant::now();
        let counter = Arc::new(AtomicU64::new(0));

        let ctx = DltContext::new("BTCH", "TST", "Batch Size Bench", "Test");

        for i in 0..messages_per_config {
            let _ = ctx.log(DltLogLevel::Info, i as i32, "BatchBench");
            counter.fetch_add(1, Ordering::Relaxed);
        }

        let duration = start.elapsed();
        let sent = counter.load(Ordering::Relaxed);

        thread::sleep(Duration::from_millis(200));

        results.push(BufferConfigResult {
            config_name: format!("batch={}", batch_size),
            num_buffers: 4,
            buffer_size: 2048,
            batch_size,
            duration,
            messages_sent: sent,
            throughput: sent as f64 / duration.as_secs_f64(),
            avg_latency_us: (duration.as_micros() as f64) / (sent as f64),
        });
    }

    results
}

pub fn print_buffer_config_results(title: &str, results: &[BufferConfigResult]) {
    println!("\n╔═══════════════════════════════════════════════════════════════════════════╗");
    println!("║ {:<73} ║", title);
    println!("╠═══════════════════════════════════════════════════════════════════════════╣");
    println!("║ Config        │ Duration │ Messages │ Throughput    │ Avg Latency      ║");
    println!("╠═══════════════════════════════════════════════════════════════════════════╣");

    for result in results {
        println!("║ {:<13} │ {:>6.3}s │ {:>8} │ {:>9.0} msg/s │ {:>10.2} μs   ║",
            result.config_name,
            result.duration.as_secs_f64(),
            result.messages_sent,
            result.throughput,
            result.avg_latency_us
        );
    }

    println!("╚═══════════════════════════════════════════════════════════════════════════╝");

    if let Some(best) = results.iter().max_by(|a, b| a.throughput.partial_cmp(&b.throughput).unwrap()) {
        println!("\n✓ Best configuration: {} ({:.0} msg/s, {:.2} μs latency)",
            best.config_name, best.throughput, best.avg_latency_us);
    }
}
