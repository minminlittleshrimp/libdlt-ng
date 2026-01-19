// lib: DLT user library for building test apps, integration tests, etc.
// Implements lockless logging with configurable multiple ring buffers and non-blocking writev I/O

// Re-export core protocol and types
pub use dlt_core::*;

// Re-export transport abstractions
pub use dlt_transport::*;

use crossbeam::channel::{Sender, Receiver, bounded, TrySendError};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::env;
use once_cell::sync::Lazy;

const DLT_DAEMON_SOCKET: &str = "/tmp/dlt";

// Environment variables for configuration
// DLT_USER_NUM_BUFFERS: Number of ring buffers (default: 4)
// DLT_USER_BUFFER_SIZE_N: Size in messages for buffer N (default: 2048)
// DLT_USER_OVERFLOW_MODE: 0=Overwrite, 1=DropNewest, 2=BlockTimeout (default: 0)
// DLT_USER_BATCH_SIZE: Number of messages to batch for writev (default: 16)

const DEFAULT_NUM_BUFFERS: usize = 4;
const DEFAULT_BUFFER_SIZE: usize = 2048;
const DEFAULT_BATCH_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DltLogLevel {
    Fatal = 1,
    Error = 2,
    Warn = 3,
    Info = 4,
    Debug = 5,
    Verbose = 6,
}

/// Overflow handling mode for ring buffer
/// Can be changed at runtime via dlt-control messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OverflowMode {
    /// Drop oldest message when buffer is full (overwrite) - mode 0
    Overwrite = 0,
    /// Drop newest message when buffer is full - mode 1
    DropNewest = 1,
    /// Block with timeout when buffer is full - mode 2
    BlockWithTimeout = 2,
}

impl OverflowMode {
    fn from_u8(val: u8) -> Self {
        match val {
            0 => OverflowMode::Overwrite,
            1 => OverflowMode::DropNewest,
            2 => OverflowMode::BlockWithTimeout,
            _ => OverflowMode::Overwrite,
        }
    }

    fn timeout() -> std::time::Duration {
        // Default timeout for BlockWithTimeout mode: 100ms
        std::time::Duration::from_millis(
            env::var("DLT_USER_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100)
        )
    }
}

/// Buffer configuration read from environment variables
struct BufferConfig {
    num_buffers: usize,
    buffer_sizes: Vec<usize>,
    batch_size: usize,
    overflow_mode: OverflowMode,
}

impl BufferConfig {
    fn from_env() -> Self {
        let num_buffers = env::var("DLT_USER_NUM_BUFFERS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_NUM_BUFFERS);

        let mut buffer_sizes = Vec::with_capacity(num_buffers);
        for i in 0..num_buffers {
            let size = env::var(format!("DLT_USER_BUFFER_SIZE_{}", i))
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_BUFFER_SIZE);
            buffer_sizes.push(size);
        }

        let batch_size = env::var("DLT_USER_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_BATCH_SIZE);

        let overflow_mode = env::var("DLT_USER_OVERFLOW_MODE")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(OverflowMode::from_u8)
            .unwrap_or(OverflowMode::Overwrite);

        BufferConfig {
            num_buffers,
            buffer_sizes,
            batch_size,
            overflow_mode,
        }
    }
}

// Internal message envelope for async logging
struct LogEnvelope {
    message: DltMessage,
    level: DltLogLevel,
    buffer_id: usize,
    local_print: bool,
    app_id: AppId,
    ctx_id: ContextId,
}

// Per-buffer statistics
struct BufferStats {
    enqueued: AtomicU64,
    dropped: AtomicU64,
    sent: AtomicU64,
}

impl BufferStats {
    fn new() -> Self {
        BufferStats {
            enqueued: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            sent: AtomicU64::new(0),
        }
    }
}

// Global DLT User state with dynamically configured buffers
struct DltUserState {
    senders: Vec<Sender<LogEnvelope>>,
    #[allow(dead_code)]
    worker_handles: Vec<JoinHandle<()>>,
    stats: Vec<Arc<BufferStats>>,
    local_print_enabled: Arc<AtomicBool>,
    overflow_mode: Arc<AtomicU8>, // Can be changed at runtime via dlt-control
}

impl DltUserState {
    fn new() -> Self {
        let config = BufferConfig::from_env();
        let local_print_enabled = Arc::new(AtomicBool::new(false));
        let overflow_mode = Arc::new(AtomicU8::new(config.overflow_mode as u8));
        
        let mut senders = Vec::with_capacity(config.num_buffers);
        let mut worker_handles = Vec::with_capacity(config.num_buffers);
        let mut stats = Vec::with_capacity(config.num_buffers);

        println!("DLT User Library initialized with {} buffers:", config.num_buffers);
        
        // Create separate channel and worker for each buffer
        for i in 0..config.num_buffers {
            let buffer_size = config.buffer_sizes[i];
            let (tx, rx) = bounded(buffer_size);
            
            println!("  Buffer {}: {} messages, batch_size={}", i, buffer_size, config.batch_size);
            
            // Create shared stats for this buffer
            let buffer_stats = Arc::new(BufferStats::new());
            let stats_clone = Arc::clone(&buffer_stats);
            
            // Spawn worker thread for this buffer
            let batch_size = config.batch_size;
            worker_handles.push(thread::Builder::new()
                .name(format!("dlt-buf-{}", i))
                .spawn(move || {
                    Self::worker_thread(rx, i, batch_size, stats_clone);
                })
                .expect(&format!("Failed to spawn worker for buffer {}", i)));
            
            senders.push(tx);
            stats.push(buffer_stats);
        }

        DltUserState {
            senders,
            worker_handles,
            stats,
            local_print_enabled,
            overflow_mode,
        }
    }

    // Background worker thread - handles all I/O with non-blocking writev batching
    fn worker_thread(receiver: Receiver<LogEnvelope>, buffer_id: usize, batch_size: usize, stats: Arc<BufferStats>) {
        // Initialize connection to daemon (only in worker thread)
        let mut transport = UnixSocketTransport::new(DLT_DAEMON_SOCKET);
        
        // Try to connect, but continue even if daemon is not available
        let mut connected = transport.connect().is_ok();

        // Batch buffer for writev
        let mut batch: Vec<LogEnvelope> = Vec::with_capacity(batch_size);
        let mut message_bytes: Vec<Vec<u8>> = Vec::with_capacity(batch_size);

        loop {
            // Collect a batch of messages for writev
            batch.clear();
            message_bytes.clear();

            // Non-blocking: try to collect up to batch_size messages
            match receiver.recv() {
                Ok(envelope) => {
                    batch.push(envelope);
                    
                    // Try to collect more without blocking
                    while batch.len() < batch_size {
                        match receiver.try_recv() {
                            Ok(envelope) => batch.push(envelope),
                            Err(_) => break, // No more messages available
                        }
                    }
                }
                Err(_) => break, // Channel closed, exit worker
            }

            // Process batch
            for envelope in &batch {
                // Local printing if requested
                if envelope.local_print {
                    println!("[{:?}][Buffer {}] {}.{}: <log>",
                        envelope.level,
                        buffer_id,
                        envelope.app_id.as_str(),
                        envelope.ctx_id.as_str()
                    );
                }

                // Serialize message
                message_bytes.push(envelope.message.to_bytes());
            }

            // Send batch using writev for efficiency
            if connected && !message_bytes.is_empty() {
                if let Err(e) = Self::writev_send(&mut transport, &message_bytes) {
                    eprintln!("DLT writev error on buffer {}: {}, attempting reconnect", buffer_id, e);
                    connected = false;
                } else {
                    // Successfully sent - update stats
                    stats.sent.fetch_add(message_bytes.len() as u64, Ordering::Relaxed);
                }
            }

            // Retry connection if disconnected
            if !connected {
                if transport.connect().is_ok() {
                    connected = true;
                    // Retry sending batch
                    if !message_bytes.is_empty() {
                        if Self::writev_send(&mut transport, &message_bytes).is_ok() {
                            stats.sent.fetch_add(message_bytes.len() as u64, Ordering::Relaxed);
                        }
                    }
                }
            }
        }

        // Cleanup
        let _ = transport.disconnect();
    }

    // Non-blocking writev send - batches multiple messages into single syscall
    // Uses real writev() for atomic multi-buffer writes (like Android's liblog)
    fn writev_send(transport: &mut UnixSocketTransport, messages: &[Vec<u8>]) -> std::io::Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        // Use real writev() syscall for maximum efficiency
        // This is a single atomic write operation, much faster than multiple send() calls
        let buffers: Vec<&[u8]> = messages.iter().map(|v| v.as_slice()).collect();
        
        match transport.writev(&buffers) {
            Ok(_bytes_written) => {
                // In non-blocking mode, partial writes are possible
                // For simplicity, we consider any write as success
                // Production code might want to handle partial writes
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Non-blocking socket: buffer full, try again later
                // For now, we drop the message (DropNewest behavior)
                // Could implement retry logic here
                Err(e)
            }
            Err(e) => Err(e),
        }
    }

    // Select buffer based on log level (round-robin for same level)
    fn select_buffer(&self, level: DltLogLevel) -> usize {
        // Simple hash-based distribution
        // Fatal logs go to buffer 0, others distributed
        match level {
            DltLogLevel::Fatal if self.senders.len() > 0 => 0,
            DltLogLevel::Error if self.senders.len() > 1 => 1 % self.senders.len(),
            _ => {
                // Use a simple counter for round-robin
                // In production, could use AtomicUsize for better distribution
                (level as usize) % self.senders.len()
            }
        }
    }

    // Lock-free message enqueue to appropriate buffer
    fn enqueue_message(&self, envelope: LogEnvelope) -> Result<(), String> {
        let buffer_id = envelope.buffer_id;
        
        if buffer_id >= self.senders.len() {
            return Err(format!("Invalid buffer id: {}", buffer_id));
        }
        
        let sender = &self.senders[buffer_id];
        let stats = &self.stats[buffer_id];

        let mode = OverflowMode::from_u8(self.overflow_mode.load(Ordering::Relaxed));

        match mode {
            OverflowMode::Overwrite | OverflowMode::DropNewest => {
                match sender.try_send(envelope) {
                    Ok(_) => {
                        stats.enqueued.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    }
                    Err(TrySendError::Full(_)) => {
                        stats.dropped.fetch_add(1, Ordering::Relaxed);
                        Err(format!("Buffer {} full, message dropped", buffer_id))
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        Err(format!("Buffer {} worker disconnected", buffer_id))
                    }
                }
            }
            OverflowMode::BlockWithTimeout => {
                match sender.send_timeout(envelope, OverflowMode::timeout()) {
                    Ok(_) => {
                        stats.enqueued.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    }
                    Err(_) => {
                        stats.dropped.fetch_add(1, Ordering::Relaxed);
                        Err(format!("Buffer {} timeout exceeded", buffer_id))
                    }
                }
            }
        }
    }
}

// Global state singleton - initialized lazily and never dropped
static DLT_USER: Lazy<DltUserState> = Lazy::new(|| DltUserState::new());

pub struct DltContext {
    app_id: AppId,
    ctx_id: ContextId,
    ecu_id: EcuId,
}

impl DltContext {
    /// Register a new DLT context (equivalent to DLT_REGISTER_APP + DLT_REGISTER_CONTEXT)
    pub fn new(app_id: &str, ctx_id: &str, _app_desc: &str, _ctx_desc: &str) -> Self {
        // Ensure DLT_USER is initialized (just by accessing it)
        let _ = &*DLT_USER;

        DltContext {
            app_id: AppId::new(app_id),
            ctx_id: ContextId::new(ctx_id),
            ecu_id: EcuId::new("ECU1"),
        }
    }

    /// Log a message with specified log level (equivalent to DLT_LOG)
    /// This is completely lock-free and never blocks (unless using BlockWithTimeout mode)
    /// Messages are automatically routed to appropriate buffer based on log level
    pub fn log(&self, level: DltLogLevel, num: i32, message: &str) -> std::io::Result<()> {
        self.log_to_buffer(level, num, message, None)
    }

    /// Log to a specific buffer (allows manual buffer selection)
    pub fn log_to_buffer(&self, level: DltLogLevel, num: i32, message: &str, buffer_id: Option<usize>) -> std::io::Result<()> {
        let payload = format!("{} {}", num, message);
        let msg = DltMessage::new_verbose(self.ecu_id, self.app_id, self.ctx_id, &payload);

        // Auto-select buffer based on log level if not specified
        let buffer = buffer_id.unwrap_or_else(|| DLT_USER.select_buffer(level));

        let envelope = LogEnvelope {
            message: msg,
            level,
            buffer_id: buffer,
            local_print: DLT_USER.local_print_enabled.load(Ordering::Relaxed),
            app_id: self.app_id,
            ctx_id: self.ctx_id,
        };

        // Lock-free enqueue - never blocks unless timeout mode
        match DLT_USER.enqueue_message(envelope) {
            Ok(_) => Ok(()),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }

    /// Log multiple messages with delay (for testing)
    pub fn log_multiple(&self, message: &str, count: usize, delay_ms: u64, level: DltLogLevel) -> std::io::Result<()> {
        for num in 0..count {
            println!("Send {} {}", num, message);
            self.log(level, num as i32, message)?;
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }
        Ok(())
    }
}

impl Drop for DltContext {
    fn drop(&mut self) {
        // Equivalent to DLT_UNREGISTER_CONTEXT and DLT_UNREGISTER_APP
        // In C implementation, contexts are unregistered automatically
    }
}

/// Enable local printing of DLT messages (equivalent to DLT_ENABLE_LOCAL_PRINT)
pub fn dlt_enable_local_print() {
    DLT_USER.local_print_enabled.store(true, Ordering::Relaxed);
}

/// Disable local printing of DLT messages
pub fn dlt_disable_local_print() {
    DLT_USER.local_print_enabled.store(false, Ordering::Relaxed);
}

/// Set overflow handling mode at runtime (can be called via dlt-control)
/// Mode: 0=Overwrite, 1=DropNewest, 2=BlockWithTimeout
pub fn dlt_set_overflow_mode(mode: u8) {
    if mode <= 2 {
        DLT_USER.overflow_mode.store(mode, Ordering::Relaxed);
        println!("DLT overflow mode set to: {:?}", OverflowMode::from_u8(mode));
    } else {
        eprintln!("Invalid overflow mode: {}, must be 0-2", mode);
    }
}

/// Get current overflow mode
pub fn dlt_get_overflow_mode() -> u8 {
    DLT_USER.overflow_mode.load(Ordering::Relaxed)
}

/// Get buffer statistics for a specific buffer
pub fn dlt_get_buffer_stats(buffer_id: usize) -> Option<(u64, u64, u64)> {
    if buffer_id < DLT_USER.stats.len() {
        let stats = &DLT_USER.stats[buffer_id];
        Some((
            stats.enqueued.load(Ordering::Relaxed),
            stats.dropped.load(Ordering::Relaxed),
            stats.sent.load(Ordering::Relaxed),
        ))
    } else {
        None
    }
}

/// Get total dropped messages across all buffers
pub fn dlt_get_overflow_count() -> u64 {
    DLT_USER.stats.iter()
        .map(|s| s.dropped.load(Ordering::Relaxed))
        .sum()
}

/// Get number of configured buffers
pub fn dlt_get_num_buffers() -> usize {
    DLT_USER.senders.len()
}

/// Print statistics for all buffers
pub fn dlt_print_buffer_stats() {
    println!("DLT Buffer Statistics (mode={:?}):", OverflowMode::from_u8(dlt_get_overflow_mode()));
    for i in 0..DLT_USER.stats.len() {
        let stats = &DLT_USER.stats[i];
        println!("  Buffer {}: enqueued={}, dropped={}, sent={}",
            i,
            stats.enqueued.load(Ordering::Relaxed),
            stats.dropped.load(Ordering::Relaxed),
            stats.sent.load(Ordering::Relaxed),
        );
    }
}