# Decoupled Module Structure - Implementation Complete

## Architecture Overview

The libdlt-ng Rust implementation now uses a fully decoupled, modular architecture that eliminates the coupling issues present in the original C implementation.

## Module Dependency Graph

```
┌─────────────────────────────────────────────────────────┐
│                   Application Layer                      │
├─────────────┬─────────────────┬─────────────────────────┤
│             │                 │                          │
│   user/     │    client/      │      daemon/             │
│   (logger)  │   (receiver)    │     (router)             │
│             │                 │                          │
└──────┬──────┴────────┬────────┴──────────┬──────────────┘
       │               │                   │
       │               │                   │
       ├───────────────┼───────────────────┤
       │               │                   │
       v               v                   v
┌─────────────────────────────────────────────────────────┐
│                  lib/ (Convenience Layer)                │
│            Re-exports core + transport                   │
└──────────────────────┬─────────────────────────────  ───┘
                       │
        ┌──────────────┴──────────────┐
        │                             │
        v                             v
┌──────────────────┐         ┌──────────────────┐
│   core/          │         │  transport/      │
│  - protocol.rs   │         │  - traits.rs     │
│  - types.rs      │         │  - unix.rs       │
│                  │         │  - tcp.rs        │
│  (Pure protocol) │         │  (Pure I/O)      │
└──────────────────┘         └──────────────────┘

                       ┌──────────────────┐
                       │   buffer/        │
                       │  - lockless.rs   │
                       │                  │
                       │  (Lock-free)     │
                       └──────────────────┘
```

## Key Improvements

### 1. **No Circular Dependencies**
- Old C design: `dlt_common.c` shared between user, client, and daemon
- New Rust design: Clear unidirectional dependencies

### 2. **Separation of Concerns**

| Module | Responsibility | Dependencies |
|--------|---------------|--------------|
| `core/` | Protocol definitions, types | None |
| `transport/` | Communication mechanisms | None |
| `buffer/` | Lock-free data structures | crossbeam |
| `user/` | Application logging API | core + transport |
| `client/` | Log receiver/control API | core + transport |
| `daemon/` | Message routing | core + transport + buffer |
| `lib/` | Convenience wrapper | core + transport |

### 3. **Trait-Based Abstraction**

```rust
// Transport trait allows pluggable backends
pub trait Transport {
    fn send(&mut self, data: &[u8]) -> Result<usize>;
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn connect(&mut self) -> Result<()>;
    fn disconnect(&mut self) -> Result<()>;
}

// Implementations:
- UnixSocketTransport
- TcpTransport
- (Future: VsockTransport for QNX)
```

### 4. **Independent Testing**
- Core protocol can be tested without I/O
- Transport implementations can be mocked
- User/client/daemon have isolated unit tests

### 5. **Lockless from Ground Up**
- `buffer/lockless.rs` uses `crossbeam::ArrayQueue`
- No coarse-grained locks
- No deadlock risk from suspended threads

## Module Details

### core/ (Zero Dependencies)
Pure data structures and protocol logic. Can be compiled for embedded systems.

```rust
// Types
pub struct AppId([u8; 4]);
pub struct ContextId([u8; 4]);
pub enum LogLevel { Fatal, Error, Warn, Info, Debug, Verbose }

// Protocol
pub struct DltMessage {
    storage_header: DltStorageHeader,
    standard_header: DltStandardHeader,
    extended_header: Option<DltExtendedHeader>,
    payload: Vec<u8>,
}
```

### transport/ (I/O Abstraction)
Platform-specific communication, but no protocol knowledge.

```rust
pub trait Transport { ... }
impl Transport for UnixSocketTransport { ... }
impl Transport for TcpTransport { ... }
```

### user/ (Application Layer)
High-level logging API for applications.

```rust
pub struct DltContext {
    app_id: AppId,
    ctx_id: ContextId,
    ecu_id: EcuId,
}

impl DltContext {
    pub fn log(&self, message: &str) -> Result<()>;
    pub fn log_multiple(&self, message: &str, count: usize, delay_ms: u64) -> Result<()>;
}
```

### client/ (Application Layer)
High-level API for log consumers and control tools.

```rust
pub struct DltClient {
    transport: Box<dyn Transport>,
    buffer: Vec<u8>,
}

impl DltClient {
    pub fn receive_message(&mut self) -> Result<Option<DltMessage>>;
    pub fn send_control_message(&mut self, data: &[u8]) -> Result<()>;
}
```

### daemon/ (Infrastructure Layer)
Routes logs from users to clients using lockless buffers.

```rust
fn main() {
    let log_buffer: LocklessBuffer<Vec<u8>> = LocklessBuffer::new(1024);

    // Unix socket listener (user -> daemon)
    spawn_unix_listener(log_buffer.clone());

    // TCP listener (daemon -> client)
    spawn_tcp_forwarder(log_buffer);
}
```

## Comparison: Old vs New

### Old C Design (Coupled)
```
src/lib/
├── dlt_user.c       # User API + protocol + transport
├── dlt_client.c     # Client API + protocol + transport
└── dlt_common.c     # Shared by ALL (coupling!)

src/daemon/
├── dlt-daemon.c     # Uses dlt_common.c
└── ...
```

### New Rust Design (Decoupled)
```
core/                # Pure protocol
transport/           # Pure I/O
buffer/              # Pure data structure
user/                # = core + transport
client/              # = core + transport
daemon/              # = core + transport + buffer
lib/                 # = core + transport (convenience)
```

## Benefits for Development

1. **Parallel Development**: Teams can work on user/client/daemon independently
2. **Easy Testing**: Mock transports, test protocol logic in isolation
3. **Platform Porting**: Just implement Transport trait for new platform
4. **Performance**: Lockless design from the start
5. **Maintainability**: Clear boundaries, no hidden dependencies

## Next Steps

- Add more transport implementations (VSOCK for QNX)
- Implement control message protocol
- Add comprehensive tests for each module
- Create C FFI bindings for legacy compatibility
- Performance benchmarking of lockless buffer

## Build Verification

All modules build successfully with no circular dependencies:

```bash
$ cargo build
   Compiling dlt-core v0.1.0
   Compiling dlt-transport v0.1.0
   Compiling dlt-buffer v0.1.0
   Compiling dlt-client v0.1.0
   Compiling dlt-user v0.1.0
   Compiling libdlt-ng v0.1.0
   Compiling dlt-daemon v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

✅ **Decoupling Complete: No circular dependencies, clean module structure!**
