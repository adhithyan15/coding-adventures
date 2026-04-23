# tcp-runtime (Rust)

TCP-specific runtime layer above `stream-reactor`.

## What is this?

This crate turns the generic byte-stream engine into a concrete TCP runtime.

It owns:

- TCP listener and stream option policy
- TCP-flavored connection metadata for handlers
- connection-local application state for protocol sessions
- a TCP-flavored outbound mailbox for delayed worker responses
- a runtime surface that Redis-, IRC-, and protocol-focused crates can bind to

It deliberately delegates byte-stream progression to `stream-reactor` instead
of reimplementing another reactor loop.

## Current Scope

Phase one supports:

- one listener
- many concurrent TCP connections
- configurable backlog, `TCP_NODELAY`, keepalive, and socket buffer defaults
- stateless and stateful handler variants
- mailbox handles that can write or close TCP connections from worker threads
- mailbox read pause/resume commands inherited from `stream-reactor`
- `TcpHandlerResult::defer_read()` for lossless backpressure when an adapter
  cannot accept the current read chunk yet
- queued-write and connection caps inherited from `stream-reactor`
- cooperative shutdown through a stop handle
- host-OS convenience constructors for macOS / BSD, Linux, and Windows

## Development

```bash
cargo test -p tcp-runtime -- --nocapture
cargo check -p tcp-runtime --tests --target x86_64-unknown-linux-gnu
cargo check -p tcp-runtime --tests --target x86_64-pc-windows-msvc
TCP_RUNTIME_STRESS_CLIENTS=10000 cargo test -p tcp-runtime bind_iocp_sustains_ten_thousand_concurrent_connections -- --ignored --nocapture
```
