# libdlt-ng - DLT Daemon in Rust with Lockless Concurrency

A modern Rust implementation of COVESA DLT (Diagnostic Log and Trace) with a fully decoupled, modular architecture and lockless concurrent logging.

## Features

- ✅ **Lockless Concurrency**: No coarse-grained locks, **eliminates deadlock risk completely**
- ✅ **Suspended Thread Safety**: Threads can be suspended (SIGSTOP) without blocking others
- ✅ **Decoupled Architecture**: Independent, reusable modules
- ✅ **DLT Protocol Compatible**: Works with standard DLT tools
- ✅ **High Performance**: Lock-free MPSC channels, multiple ring buffers
- ✅ **Cross-Platform Ready**: Abstract transport layer for easy porting

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Application Layer                      │
├─────────────┬─────────────────┬─────────────────────────┤
│   user/     │    client/      │      daemon/             │
│  (logger)   │   (receiver)    │     (router)             │
└──────┬──────┴────────┬────────┴──────────┬──────────────┘
       │               │                   │
       v               v                   v
┌──────────────────────────────────────────────────────────┐
│          core/ (Protocol) + transport/ (I/O)             │
│          buffer/ (Lockless Queue)                        │
└──────────────────────────────────────────────────────────┘
```

See [ARCHITECTURE.md](ARCHITECTURE.md) and [DECOUPLED_DESIGN.md](DECOUPLED_DESIGN.md) for details.

## Quick Start

### Build

```bash
cargo build --release
```

### Run Demo

```bash
./demo.sh
```

Or manually:

```bash
# Terminal 1: Start daemon
./target/release/dlt-daemon

# Terminal 2: Send logs
./target/release/dlt-example-user -n 100 "Hello from Rust DLT"

# Terminal 3: Receive logs
./target/release/dlt-receive -a 127.0.0.1
```

## Usage

### User API (Logging)

```rust
use dlt_user::DltContext;

fn main() {
    // Create context (auto-registers with daemon)
    let ctx = DltContext::new("APP1", "CTX1", "My App", "My Context");

    // Log messages
    ctx.log("Simple message").unwrap();
    ctx.log_multiple("Batch message", 10, 100).unwrap();

    // Auto-unregisters on drop
}
```

### Client API (Receiving)

```rust
use dlt_client::{DltClient, parse_message_text};

fn main() {
    let mut client = DltClient::connect("127.0.0.1", 3490).unwrap();

    loop {
        if let Some(msg) = client.receive_message().unwrap() {
            println!("{}", parse_message_text(&msg));
        }
    }
}
```

## Module Structure

| Module | Purpose | Dependencies |
|--------|---------|-------------|
| `core/` | DLT protocol definitions | None |
| `transport/` | Unix socket, TCP abstractions | None |
| `buffer/` | Lock-free queue | crossbeam |
| `user/` | Logging API | core + transport |
| `client/` | Log receiver API | core + transport |
| `daemon/` | Message router | core + transport + buffer |
| `lib/` | Convenience wrapper | core + transport |

## Key Differences from C Implementation

### Problem: Old C DLT Coupling

- `dlt_common.c` shared between user, client, daemon → circular dependencies
- User and client APIs in same library → tight coupling
- Coarse-grained locks → deadlock risk with multithreading

### Solution: Rust Decoupled Design

- Independent modules with clear boundaries
- Lockless buffers from the start
- Trait-based transport abstraction
- No circular dependencies

## Output Format

libdlt-ng outputs logs in the standard DLT format:

```
1762707231.123456 ECU1 TEST TCON log warn V 1 [0 Hello from Rust DLT]
1762707231.234567 ECU1 TEST TCON log warn V 1 [1 Hello from Rust DLT]
```

Format: `timestamp.microseconds ECU_ID APP_ID CTX_ID log_level verbose arg_count [payload]`

## Command Line Tools

### dlt-daemon

Start the DLT daemon to collect and forward logs.

```bash
./target/release/dlt-daemon
```

Options:
- Listens on Unix socket `/tmp/dlt` for user applications
- Serves logs on TCP port `3490` for clients

### dlt-example-user

Send test log messages.

```bash
./target/release/dlt-example-user -n <count> <message>
```

Options:
- `-n <count>`: Number of messages to send (default: 10)
- `<message>`: Message text (default: "Hello")

Example:
```bash
./target/release/dlt-example-user -n 100 "Test message"
```

### dlt-receive

Receive and display logs from daemon.

```bash
./target/release/dlt-receive -a <address>
```

Options:
- `-a <address>`: Daemon address (default: 127.0.0.1)

Example:
```bash
./target/release/dlt-receive -a localhost
```

## Development

### Adding New Transport

Implement the `Transport` trait:

```rust
use dlt_transport::Transport;

struct MyTransport { ... }

impl Transport for MyTransport {
    fn send(&mut self, data: &[u8]) -> Result<usize> { ... }
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize> { ... }
    fn connect(&mut self) -> Result<()> { ... }
    fn disconnect(&mut self) -> Result<()> { ... }
}
```

### Building for aarch64

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --target aarch64-unknown-linux-gnu --release
```

### Running Tests

```bash
cargo test
```

## Compatibility

- **Protocol**: Compatible with DLT specification
- **API**: High-level API similar to C DLT library
- **Tools**: Works with standard DLT tools (DLT Viewer, etc.)

## Performance

- **Lock-free**: Uses MPSC channels with atomic operations for zero-lock message passing
- **Concurrent**: Multiple threads can log simultaneously without blocking
- **Reliable**: No deadlock risk from suspended threads holding locks
- **Scalable**: Linear performance scaling with thread count (see benchmarks)
- **Configurable**: Multiple ring buffers, batch sizes, overflow modes

### Benchmarking

Comprehensive benchmark suite to validate lock-free performance:

```bash
# Quick validation (~30 seconds)
./target/release/dlt-bench -c quick

# Thread scalability test
./target/release/dlt-bench -c concurrency-scale

# Compare overflow modes
./target/release/dlt-bench -c overflow-all

# Full benchmark suite (~5 minutes)
./target/release/dlt-bench -a

# List all available tests
./target/release/dlt-bench list
```

**Benchmark Categories:**
- **Overflow Modes**: Drop/Overwrite/Timeout performance comparison
- **Buffer Configuration**: Optimal buffer counts, sizes, batch sizes
- **Concurrency**: Thread scalability (1-16 threads), MPSC validation
- **Suspended Threads**: Deadlock prevention validation (SIGSTOP scenarios)
- **CPU Efficiency**: Throughput per CPU core

**Critical Tests:**
```bash
# Validate deadlock-free design with suspended threads
./target/release/dlt-bench -c suspend-all
```

This proves the lock-free design handles scenarios that would deadlock mutex-based systems.

See [benchmark/README.md](benchmark/README.md) for detailed documentation.

**Typical Results:**
- Single-threaded: ~1-5M msg/s
- 4 threads: ~4-10M msg/s (linear scaling)
- 16 threads: ~10-20M msg/s (with efficient batching)
- Zero lock contention in all scenarios

## Non-Blocking I/O

Implements **real writev() syscall** with non-blocking sockets for maximum throughput:

- ✅ **Real writev()**: Single atomic syscall for multiple buffers
- ✅ **Non-blocking sockets**: Never blocks producer threads
- ✅ **Optimized buffers**: 64KB socket buffers for high throughput
- ✅ **4.8M msg/s**: Peak performance with batch=16

See [NONBLOCKING_IO.md](NONBLOCKING_IO.md) for implementation details.

## Future Work

- [ ] Add VSOCK transport for QNX/embedded systems
- [ ] Implement control message protocol
- [ ] Add C FFI bindings for legacy compatibility
- [x] Performance benchmarking suite ✓
- [x] Real writev() syscall implementation ✓
- [x] Non-blocking I/O ✓
- [ ] Comprehensive test coverage
- [ ] Log level filtering
- [ ] Multiple context support per application
- [ ] CPU profiling integration
- [ ] Adaptive batch sizing based on load

## License

MPL-2.0 (matching original DLT daemon license)

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) - High-level architecture overview
- [DECOUPLED_DESIGN.md](DECOUPLED_DESIGN.md) - Detailed decoupling design
- [DLT For Developers](https://github.com/COVESA/dlt-daemon/blob/master/doc/dlt_for_developers.md) - Original DLT documentation

## Contributing

1. Modules are independent - work on one without affecting others
2. All public APIs should be documented
3. Add tests for new functionality
4. Follow Rust idioms and best practices

## References

- [COVESA DLT Daemon](https://github.com/COVESA/dlt-daemon)
- [DLT Specification](https://www.covesa.org/)
