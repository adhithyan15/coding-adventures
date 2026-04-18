# tcp-reactor

## Overview

`tcp-reactor` is a small nonblocking TCP server built on top of
`native-event-core`.

It is intentionally narrow:

- one listener
- many concurrent connections
- single-threaded event loop
- pluggable byte handler

This package exists to prove that `native-event-core` is a usable substrate for
real transports. If this crate works, higher layers such as WebSockets can build
above it.

## Layer Position

```text
websocket-runtime / higher protocols
    ↓
tcp-reactor
    ↓
native-event-core
    ↓
kqueue / epoll / iocp
```

## Concepts

- The listener is registered as a readable source.
- Accepted streams are switched to nonblocking mode and registered with the backend.
- Read events drain input.
- Write events flush queued output when nonblocking writes would otherwise stall.
- Active connections are capped so a flood of idle sockets cannot grow the
  reactor state without bound.
- Queued outbound bytes are capped per connection. When a handler would exceed
  the configured budget, the reactor closes that connection instead of buffering
  unbounded response data.
- A stop flag allows clean shutdown without panicking under concurrent load.

## Public API

- `ConnectionId`
- `ConnectionInfo`
- `StopHandle`
- `TcpReactor::with_backend(listener, backend, handler)`
- `TcpReactor::serve()`
- `TcpReactor::stop_handle()`
- `TcpReactor::local_addr()`
- `TcpReactor::set_max_connections(max_connections)`
- `TcpReactor::set_max_pending_write_bytes(max_pending_write_bytes)`
- macOS/BSD convenience: `TcpReactor::bind_kqueue(addr, handler)`

## Data Flow

Input:

- incoming TCP connections
- readable and writable readiness events
- handler output bytes

Output:

- bytes written back to connected clients

## Test Strategy

- unit tests for lifecycle basics
- macOS/BSD end-to-end test with many concurrent echo clients
- macOS/BSD connection-cap test
- macOS/BSD pending-write-budget overflow test
- server shutdown test

## Future Extensions

- explicit accept/backlog tuning
- wakeup fd integration for immediate stop
- connection metadata callbacks
- protocol framing layers above raw byte handling
