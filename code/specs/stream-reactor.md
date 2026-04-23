# stream-reactor

## Overview

`stream-reactor` is the first generic byte-stream runtime above
`transport-platform`.

It exists to separate:

- stream progression
- listener acceptance
- queued writes
- close and half-close handling
- backpressure budgets

from higher protocol logic such as RESP, IRC line framing, HTTP, or WebSocket
frames.

That makes it the missing middle layer between:

- `transport-platform`, which knows about listeners, streams, timers, wakeups,
  and platform events
- `tcp-runtime`, which should eventually know about TCP server policy and
  runtime composition for real applications

## Layer Position

```text
Redis / IRC / WebSocket / future protocols
    ↓
tcp-runtime
    ↓
stream-reactor
    ↓
transport-platform
    ↓
native socket + event providers
```

## Why This Exists

The repository already has:

- raw native backends such as `epoll`, `kqueue`, and `iocp`
- `native-event-core`
- `transport-platform`
- `tcp-reactor` as the earlier proof-of-life transport loop

What is still missing is the generic stream runtime that proves:

- we can progress many byte streams above `transport-platform`
- listener acceptance and per-stream interest management live above the raw
  provider seam
- protocols can depend on byte-stream semantics without inheriting
  platform-specific details

`stream-reactor` should therefore become the reusable substrate for:

- RESP/Redis request loops
- IRC line-oriented sessions
- WebSocket upgrade targets after the handshake layer exists
- future language bindings that want a stream-oriented runtime without directly
  binding to TCP policy objects

## Scope

Phase one of `stream-reactor` supports:

- one listener
- many accepted byte streams
- readable and writable event progression
- per-connection application state owned by the reactor
- per-stream queued outbound bytes
- thread-safe outbound mailbox submissions keyed by `ConnectionId`
- configurable connection caps
- configurable queued-write budget caps
- neutral handler output in terms of bytes and close intent
- optional close callbacks for connection teardown
- cooperative stop via a stop flag

Phase two adds a mailbox for worker threads and other off-reactor producers to
send bytes back to streams without blocking the readable callback.

Phase one and two intentionally do not yet require:

- timer-driven idle deadlines
- listener adoption instead of bind-at-construction
- UDP or Unix-domain sockets
- protocol-specific framing
- multi-listener fan-in

## Core Concepts

### `ConnectionId`

`stream-reactor` owns its own connection identity instead of exposing
`transport-platform::StreamId` to higher layers.

That keeps the upper runtime boundary repository-owned and stable even if the
provider representation changes later.

### `StreamConnectionInfo`

The handler sees:

- `ConnectionId`
- peer address

Phase one does not expose the lower-layer `StreamId`.

### `StreamHandlerResult`

The per-read handler result is intentionally neutral:

- bytes to queue for writing
- whether the stream should close after queued bytes flush

This keeps the runtime generic for protocols that:

- echo bytes
- transform requests into responses
- emit one last frame and then close

without forcing protocol knowledge into the reactor.

### `StreamMailbox`

Longer-running application work should not keep the reactor thread waiting for
a response. `StreamMailbox` is the return address for that work.

The handle is cloneable and thread-safe. Producers can submit:

- bytes to write to a connection
- bytes to write followed by close-after-flush
- a close request with no bytes

The reactor drains mailbox commands on its event-loop thread and applies the
same queued-write budget rules as inline handler responses. Mail for already
closed connections is discarded.

Until `transport-platform` exposes a thread-safe wake handle, mailbox delivery
is cooperative: the reactor notices queued commands on the next poll tick.

### Connection State

Higher layers such as Redis do not just need bytes-in / bytes-out callbacks.
They also need per-connection state that lives for the full lifetime of the
stream, for example:

- partially buffered protocol frames
- selected logical database
- authentication/session flags
- protocol parser state

`stream-reactor` should therefore own connection-local application state rather
than forcing callers to keep side maps keyed by connection id.

Phase one should support:

- state initialization when a connection is admitted
- mutable access to that state on each readable callback
- a close callback that receives the final state value when the stream leaves
  the reactor

### Backpressure

`stream-reactor` owns queued-write budgets and connection caps.

If a handler would cause a stream's queued bytes to exceed the configured cap,
the reactor closes that stream instead of buffering unbounded data.

That makes the runtime safe to embed under higher-level protocols that may
temporarily outproduce the socket.

## Public API

Phase-one public API:

- `ConnectionId`
- `StreamConnectionInfo`
- `StreamHandlerResult`
- `StreamReactorOptions`
- `StopHandle`
- `StreamReactor::bind(platform, address, options, handler)`
- `StreamReactor::bind_with_state(platform, address, options, init, handler, on_close)`
- `StreamReactor::serve()`
- `StreamReactor::local_addr()`
- `StreamReactor::stop_handle()`
- `StreamReactor::mailbox()`
- `StreamReactor::set_max_connections(max)`
- `StreamReactor::set_max_pending_write_bytes(max)`
- `StreamMailbox::send(connection_id, bytes)`
- `StreamMailbox::send_and_close(connection_id, bytes)`
- `StreamMailbox::close(connection_id)`
- macOS/BSD convenience: `StreamReactor::bind_kqueue(addr, handler)`

## Event Progression

Listener side:

1. bind the listener through `transport-platform`
2. enable listener readability interest
3. on `ListenerAcceptReady`, drain accepts until `WouldBlock`
4. configure each accepted stream through the provider
5. initialize the application state for each admitted stream
6. register readable interest for each admitted stream

Readable side:

1. read until `WouldBlock`, close, or error
2. pass each read chunk to the handler together with mutable connection-local
   state
3. append returned bytes to the pending-write queue
4. if the queue is non-empty, enable writable interest
5. if the peer closed and no bytes remain to flush, close the stream

Writable side:

1. write until `WouldBlock`, queue empty, or error
2. if bytes remain queued, keep writable interest
3. if queue drains fully and the peer already closed or the handler requested
   close-after-flush, close the stream
4. otherwise fall back to readable-only interest

Mailbox side:

1. drain queued mailbox commands on the reactor thread
2. find the active stream for each `ConnectionId`
3. append write bytes to that stream's pending-write queue
4. mark close-after-flush when requested
5. enable writable interest for streams that now have pending bytes
6. close streams that exceed the queued-write budget

## Error Handling

Phase-one rules:

- listener-level provider errors terminate `serve()`
- per-stream provider errors close that stream
- `InvalidResource` or `ResourceClosed` during close teardown should be treated
  as already-closed cleanup, not as a fatal reactor error
- when a stream leaves the reactor, the close callback should receive the final
  connection-local state exactly once

## Current Limitation

`transport-platform` currently exposes wakeups as methods on the mutable
provider object, not as separate thread-safe wake handles.

Because of that, phase-one `StopHandle` is cooperative:

- it sets a stop flag
- `serve()` polls with a bounded timeout and notices the flag quickly

This is acceptable for phase one, but a future `transport-platform` revision
should make wakeups usable across threads so `stream-reactor` can stop
immediately without relying on short poll timeouts.

## Test Strategy

Phase one should include:

- macOS/BSD end-to-end test with many concurrent echo clients
- connection-cap test
- queued-write-budget overflow test
- stop-handle shutdown test
- per-connection state persistence test across multiple reads
- close-callback test proving state teardown runs once per connection
- Linux and Windows target compile coverage through `cargo check`

## Relationship To `tcp-reactor`

`tcp-reactor` was a proof reactor directly above `native-event-core`.

`stream-reactor` should now become the generic runtime that future TCP policy
layers depend on. Over time, `tcp-reactor` should either:

- be rewritten atop `stream-reactor`, or
- be retired in favor of `tcp-runtime`

The important point is that higher protocol and application work should now
move upward from `stream-reactor`, not back downward to raw backend loops.
