# tcp-runtime

## Overview

`tcp-runtime` is the first TCP-specific runtime layer above `stream-reactor`.

It exists to turn the generic byte-stream substrate into a concrete transport
surface that application servers can depend on without needing to understand:

- `transport-platform` listener and stream configuration details
- raw `stream-reactor` handler wiring
- TCP listener defaults such as backlog, `TCP_NODELAY`, and keepalive policy

Phase one is intentionally not the final transport engine described in
`tcp-runtime-roadmap.md`. It is the first honest slice that makes the layering
real:

```text
Redis / IRC / future TCP protocols
    ↓
tcp-runtime
    ↓
stream-reactor
    ↓
transport-platform
```

## Why This Exists

The repository now has:

- raw native backends
- `transport-platform`
- `stream-reactor`
- older proof crates such as `tcp-reactor`

What application-facing TCP work still lacks is a concrete runtime surface that
answers practical questions like:

- how do I bind a TCP listener with the repository's default policy?
- how do I configure backlog, `TCP_NODELAY`, keepalive, and buffer sizing?
- how do I receive TCP connection metadata instead of generic stream metadata?
- how do I run an end-to-end TCP server without touching platform details?

`tcp-runtime` is that missing layer.

## Phase-One Scope

Phase one of `tcp-runtime` supports:

- one TCP listener
- many accepted TCP connections
- TCP-specific listener and stream configuration through `TcpRuntimeOptions`
- TCP-specific connection metadata for handlers
- per-connection application state above the TCP transport
- queued-write and connection-admission limits inherited from `stream-reactor`
- cooperative shutdown through a stop handle
- host-OS convenience constructors:
  - macOS / BSD: `bind_kqueue`
  - Linux: `bind_epoll`
  - Windows: `bind_windows`

Phase one intentionally does not yet require:

- listener groups
- pause / resume of accepts
- explicit drain mode
- idle or write deadlines
- typed connection-close reasons above `stream-reactor`
- multi-core reactor sharding
- a C ABI

## Core Concepts

### `TcpConnectionInfo`

Handlers should receive TCP-flavored metadata:

- repository-owned connection identity
- peer socket address
- local listener address

The handler should not need to reconstruct the local address from the platform
or listener every time it processes bytes.

### `TcpHandlerResult`

The handler result remains intentionally simple:

- bytes to queue for writing
- whether the connection should close after queued bytes flush

This keeps the runtime useful for:

- echo-style servers
- request / response protocols such as Redis
- line-oriented protocols such as IRC

without coupling the runtime to framing rules.

### Connection State

Real TCP servers such as Redis and IRC keep connection-local protocol state that
must survive across many reads:

- partially buffered request bytes
- selected logical database or channel/session context
- authentication or negotiation state

`tcp-runtime` should support that pattern directly instead of forcing each
application to bolt on an external map keyed by connection id.

Phase one should therefore expose stateful bind variants that:

- initialize per-connection state from `TcpConnectionInfo`
- pass mutable state into each read callback
- optionally observe final state during close teardown

### `TcpRuntimeOptions`

`tcp-runtime` owns the TCP policy surface that applications care about:

- listener options:
  - backlog
  - `reuse_address`
  - `reuse_port`
  - default `TCP_NODELAY`
  - default keepalive policy
- accepted-stream options:
  - `TCP_NODELAY`
  - keepalive
  - receive and send buffer sizing
- runtime options:
  - read buffer size
  - maximum concurrent connections
  - maximum queued bytes per connection
  - poll timeout

This is the first place where TCP-specific policy becomes explicit instead of
being hidden in generic defaults.

## Public API

Phase-one public API:

- `TcpConnectionInfo`
- `TcpHandlerResult`
- `TcpRuntimeOptions`
- `StopHandle`
- `TcpRuntime::bind(platform, address, options, handler)`
- `TcpRuntime::bind_with_state(platform, address, options, init, handler, on_close)`
- `TcpRuntime::serve()`
- `TcpRuntime::local_addr()`
- `TcpRuntime::stop_handle()`
- `TcpRuntime::set_max_connections(max)`
- `TcpRuntime::set_max_pending_write_bytes(max)`
- convenience constructors:
  - `TcpRuntime::bind_kqueue(addr, options, handler)`
  - `TcpRuntime::bind_kqueue_with_state(addr, options, init, handler, on_close)`
  - `TcpRuntime::bind_epoll(addr, options, handler)`
  - `TcpRuntime::bind_epoll_with_state(addr, options, init, handler, on_close)`
  - `TcpRuntime::bind_windows(addr, options, handler)`
  - `TcpRuntime::bind_windows_with_state(addr, options, init, handler, on_close)`

## Relationship To `stream-reactor`

`stream-reactor` remains the generic byte-stream engine.

`tcp-runtime` should:

- own TCP listener and socket policy
- translate generic stream metadata into TCP runtime metadata
- present the TCP-specific API that Redis, IRC, and future bindings can target

`tcp-runtime` should not:

- duplicate the stream progression logic already solved by `stream-reactor`
- reintroduce direct dependencies on raw platform details

That means the implementation should delegate event progression and queued-write
handling to `stream-reactor` rather than forking another reactor loop.

## Error Handling

Phase one should preserve the `stream-reactor` behavior:

- listener-level platform failures terminate `serve()`
- per-connection platform failures close only that connection
- close-on-overflow remains the defense when queued outbound bytes exceed the
  configured budget

The runtime should keep using `transport-platform::PlatformError` for now.
Future phases can wrap or translate those into a richer TCP error taxonomy.

## Test Strategy

Phase one should include:

- macOS / BSD concurrent echo test through the `TcpRuntime` surface
- connection-cap test through the `TcpRuntime` surface
- queued-write-budget overflow test through the `TcpRuntime` surface
- local-address visibility test for handler metadata
- per-connection state persistence test through the `TcpRuntime` surface
- close-callback test through the `TcpRuntime` surface
- Linux and Windows target compile coverage through `cargo check`

## Future Work

Later `tcp-runtime` phases should add:

- listener groups and sharding
- drain mode and graceful listener shutdown
- idle and write deadlines
- statistics and tracing hooks
- typed lifecycle events
- FFI-facing runtime handles

This phase should stay focused on making the `stream-reactor` layer usable as a
real TCP runtime without claiming that the entire roadmap is already complete.
