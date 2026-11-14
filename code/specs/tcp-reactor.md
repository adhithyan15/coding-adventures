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
    ├── Linux / macOS / BSD: readiness engine
    │       ↓ native-event-core
    │       epoll / kqueue
    └── Windows: completion engine
            ↓ iocp (completion port) + Winsock overlapped I/O
            AcceptEx / WSARecv / WSASend
```

The two engines expose the same public API and the same observable behaviour.
They are different inside because the OS models are different: epoll and kqueue
say "this socket is ready, go and read it"; Windows says "the read you started
has finished". See `event-loop-native-backends.md` ("Windows IOCP"): IOCP is
modelled as completion-based and is never dressed up as a readiness API.

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

### Windows completion engine

The Windows engine owns one I/O completion port (the `iocp` crate) and talks to
the native networking stack directly:

- **Accept.** A small fixed number of `AcceptEx` operations are kept posted on the
  listener. Each carries a pre-created overlapped accept socket. When one
  completes the socket gets `SO_UPDATE_ACCEPT_CONTEXT`, is wrapped in a
  `TcpStream` (which gives it normal `peer_addr`/`local_addr` and closes it on
  drop), associated with the port, and a replacement `AcceptEx` is posted.
  Connections past the cap are closed immediately, as on Unix.
- **Read.** Each connection has at most one `WSARecv` in flight. A completion of
  `n > 0` bytes calls the handler; `0` bytes is the peer's half-close.
- **Write.** Each connection has at most one `WSASend` in flight. Handler output
  is appended to a pending buffer; when no send is in flight the pending bytes
  move to the in-flight buffer and are sent. A partial completion resends the
  remainder; this is defensive, since an overlapped stream send normally completes
  whole, so no test can provoke it.
- **Backpressure and budget** match the Unix engine: the queued bytes (in flight
  plus pending) are capped per connection; a handler response that would exceed
  the cap closes the connection; while the queue is at or over the cap no new
  `WSARecv` is posted until a send completion brings it back under.
- **Half-close.** After the peer's half-close the connection is closed once its
  queued output has been flushed.
- **Buffer lifetime.** The kernel owns an operation's buffers and `OVERLAPPED`
  from the moment it is posted until its completion packet is dequeued, even if
  the socket is closed meanwhile (closing cancels the operation, which still
  completes, with an abort error). So a closing connection stays in the table,
  with no new operations posted, until all of its operations have completed;
  only then are its buffers freed. If a drain deadline passes with operations
  still outstanding their memory is deliberately leaked rather than freed under
  the kernel.
- Operation completion errors (reset, aborted, anything else) close only the
  affected connection; they never abort `serve`.
- Accept slots are re-armed best-effort: a failed or aborted re-post leaves the slot
  disarmed and `serve` retries it on its next loop turn (which doubles as the
  back-off), so exhausting sockets or buffers never ends the server. Aborts are not
  treated as "the listener is closing" because Windows also aborts operations whose
  issuing thread exited.
- One `WSASend` carries at most 1 GiB (`WSABUF.len` is a `u32`), and a send that
  completes with zero bytes while data remains closes the connection rather than
  being re-posted forever.
- If the handler panics, the connection is closed before the panic propagates out of
  `serve` (the same propagation as the readiness engine).
- Dropping the reactor cancels the pending `AcceptEx` operations with `CancelIoEx` on
  the listener and drains their completions.
- `serve` should stay on one thread for the life of the posted operations: Windows
  cancels overlapped operations when the thread that issued them exits.
- `serve` polls the port with the same 10 ms timeout as the Unix engine so the
  stop flag is observed promptly. Stopping closes every connection and drains
  their completions; the listener's `AcceptEx` operations stay posted so
  `serve` can be called again. Dropping the reactor cancels and drains them.

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
- Linux convenience: `TcpReactor::bind_epoll(addr, handler)`
- macOS/BSD convenience: `TcpReactor::bind_kqueue(addr, handler)`
- Windows: `TcpReactor::bind_iocp(addr, handler)` and
  `TcpReactor::with_listener(listener, handler)`. On Windows `TcpReactor` is not
  generic over a backend, because the completion engine is not an
  `EventBackend`.
- Every platform: the `NativeTcpReactor` alias and `bind_native(addr, handler)`,
  which pick the native engine, so portable callers and tests are written once.

## Data Flow

Input:

- incoming TCP connections
- readable and writable readiness events
- handler output bytes

Output:

- bytes written back to connected clients

## Test Strategy

The behaviour tests are written once against the public API (`bind_native`,
plain `std::net` clients) and run on Linux, macOS and Windows:

- many concurrent echo clients, each half-closing its write side
- connection cap: a client beyond the cap is closed without being served
- pending-write budget: a handler response over the budget closes the
  connection, and the client never receives it
- large response with a slow reader, exercising partial writes and the
  backpressure path
- stop: `serve` returns promptly after `StopHandle::stop`, and `serve` can be
  called again
- dropping an idle reactor, and dropping one with live connections, releases
  everything (no hang)

Windows-only unit tests cover the completion engine's bookkeeping that has no
Unix counterpart (closing a connection with operations in flight, drain
deadline).

## Future Extensions

- explicit accept/backlog tuning
- wakeup fd integration for immediate stop
- connection metadata callbacks
- protocol framing layers above raw byte handling
