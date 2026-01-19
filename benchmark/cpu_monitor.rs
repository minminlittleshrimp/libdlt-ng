// cpu_monitor.rs - CPU usage monitoring during benchmarks
use sysinfo::{System, RefreshKind, CpuRefreshKind, ProcessRefreshKind};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

pub struct CpuStats {
    pub avg_cpu_percent: f32,
    pub peak_cpu_percent: f32,
    pub samples: usize,
}

/// Monitor CPU usage in background thread
#[allow(dead_code)]
pub fn monitor_cpu_usage(duration: Duration) -> CpuStats {
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything())
    );

    let mut samples = vec![];
    let start = std::time::Instant::now();

    // Initial refresh
    sys.refresh_cpu();
    thread::sleep(Duration::from_millis(100));

    while start.elapsed() < duration {
        sys.refresh_cpu();

        // Get process CPU usage
        let pid = sysinfo::get_current_pid().unwrap();
        if let Some(process) = sys.process(pid) {
            let cpu_usage = process.cpu_usage();
            samples.push(cpu_usage);
        }

        thread::sleep(Duration::from_millis(100));
    }

    let avg = if !samples.is_empty() {
        samples.iter().sum::<f32>() / samples.len() as f32
    } else {
        0.0
    };

    let peak = samples.iter().copied().fold(0.0f32, f32::max);

    CpuStats {
        avg_cpu_percent: avg,
        peak_cpu_percent: peak,
        samples: samples.len(),
    }
}

/// Run a benchmark with CPU monitoring
#[allow(dead_code)]
pub fn run_with_cpu_monitor<F, R>(name: &str, benchmark_fn: F) -> (R, CpuStats)
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    println!("\n=== Running {} with CPU monitoring ===", name);

    let (tx, rx) = std::sync::mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    // Start CPU monitor thread
    let monitor_handle = thread::spawn(move || {
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_processes(ProcessRefreshKind::everything())
        );

        let mut samples = vec![];
        sys.refresh_cpu();
        thread::sleep(Duration::from_millis(100));

        while !stop_clone.load(Ordering::Relaxed) {
            sys.refresh_cpu();

            let pid = sysinfo::get_current_pid().unwrap();
            if let Some(process) = sys.process(pid) {
                samples.push(process.cpu_usage());
            }

            thread::sleep(Duration::from_millis(50));
        }

        let avg = if !samples.is_empty() {
            samples.iter().sum::<f32>() / samples.len() as f32
        } else {
            0.0
        };

        let peak = samples.iter().copied().fold(0.0f32, f32::max);

        CpuStats {
            avg_cpu_percent: avg,
            peak_cpu_percent: peak,
            samples: samples.len(),
        }
    });

    // Run benchmark
    let bench_handle = thread::spawn(move || {
        let result = benchmark_fn();
        tx.send(result).unwrap();
    });

    // Wait for benchmark to complete
    let result = rx.recv().unwrap();
    bench_handle.join().unwrap();

    // Stop CPU monitor
    stop.store(true, Ordering::Relaxed);
    let cpu_stats = monitor_handle.join().unwrap();

    println!("CPU Stats: avg={:.1}%, peak={:.1}%, samples={}",
        cpu_stats.avg_cpu_percent,
        cpu_stats.peak_cpu_percent,
        cpu_stats.samples
    );

    (result, cpu_stats)
}
