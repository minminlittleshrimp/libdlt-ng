# Decoupled Modular Architecture for libdlt-ng

## Problem Analysis: Old C DLT Daemon Coupling Issues

### Current Problems:
1. **dlt_common.c** is shared between user, client, and daemon - creates circular dependencies
2. **dlt_user.c** and **dlt_client.c** both in `src/lib` - user API and client API are coupled
3. **Shared state** across multiple modules with global variables
4. **No clear separation** between protocol, transport, and application logic

## New Rust Architecture: Decoupled Modules

```
libdlt-ng/
├── core/           # Core protocol and types (NO dependencies on transport)
│   ├── protocol.rs      # DLT message format, serialization/deserialization
│   ├── types.rs         # Common types (AppId, ContextId, LogLevel, etc.)
│   └── mod.rs
│
├── transport/      # Transport layer (Unix sockets, TCP, etc.)
│   ├── unix.rs          # Unix socket transport
│   ├── tcp.rs           # TCP transport
│   └── mod.rs
│
├── buffer/         # Lockless buffer implementation
│   ├── lockless.rs      # Lock-free queue
│   └── mod.rs
│
├── user/           # User API (logging from applications)
│   ├── context.rs       # DLT context management
│   ├── api.rs           # Public user API
│   └── mod.rs
│   └── Cargo.toml       # Depends on: core, transport
│
├── client/         # Client API (receiving logs, control)
│   ├── receiver.rs      # Log receiver
│   ├── control.rs       # Control message API
│   └── mod.rs
│   └── Cargo.toml       # Depends on: core, transport
│
├── daemon/         # Daemon (router/forwarder)
│   ├── router.rs        # Routes logs from users to clients
│   ├── main.rs
│   └── Cargo.toml       # Depends on: core, transport, buffer
│
└── lib/            # Core library (protocol only)
    ├── mod.rs           # Re-exports core and transport
    └── Cargo.toml       # Base library with NO app logic

```

## Key Design Principles

### 1. **Separation of Concerns**
- **core**: Protocol definitions only (no I/O)
- **transport**: Communication mechanisms (no protocol logic)
- **user**: Application logging (depends on core + transport)
- **client**: Log consumption (depends on core + transport)
- **daemon**: Message routing (depends on core + transport + buffer)

### 2. **Dependency Flow** (Acyclic!)
```
user → core + transport
client → core + transport
daemon → core + transport + buffer

NO circular dependencies!
```

### 3. **Trait-Based Abstraction**
```rust
// Transport trait allows pluggable backends
trait Transport {
    fn send(&mut self, data: &[u8]) -> Result<()>;
    fn receive(&mut self) -> Result<Vec<u8>>;
}

// Different implementations
struct UnixSocketTransport { ... }
struct TcpTransport { ... }
struct VsockTransport { ... }  // For QNX/embedded
```

### 4. **No Shared State**
- Each module owns its state
- Communication via message passing (channels)
- Lockless buffers for high-performance

### 5. **Testability**
- Core protocol can be tested without I/O
- Transport can be mocked
- User/client/daemon can be integration tested

## Migration Strategy

### Phase 1: Create Core Module ✓ (Current)
- Extract protocol definitions from lib

### Phase 2: Create Transport Module
- Abstract Unix socket and TCP into traits

### Phase 3: Refactor User & Client
- Make them independent modules
- Both depend on core + transport

### Phase 4: Refactor Daemon
- Use composition instead of tight coupling
- Plugin architecture for transports

## Benefits

1. **Modularity**: Each component can be developed/tested independently
2. **Reusability**: Core protocol can be used in embedded systems
3. **Maintainability**: Clear boundaries, no circular deps
4. **Performance**: Lockless design from ground up
5. **Cross-platform**: Transport abstraction makes porting easy (Linux/QNX/etc.)
