# stream-reactor (Rust)

Generic byte-stream reactor built on top of `transport-platform`.

## What is this?

This crate is the reusable stream runtime that sits between:

- `transport-platform`, which knows how to bind listeners, accept streams,
  read, write, and poll
- higher runtimes such as `tcp-runtime`, Redis, IRC, and future WebSocket
  layers

The reactor owns:

- listener acceptance
- per-connection application state
- per-stream readable and writable progression
- queued writes
- a thread-safe outbound mailbox for delayed writes
- close-after-flush handling
- connection caps
- queued-write budget caps
- close callbacks with final connection state

It deliberately does not parse protocols or know about RESP, IRC, HTTP, or
WebSocket framing.

## Current Scope

Phase one supports:

- one listener
- many concurrent streams
- stateful or stateless handlers
- neutral handler results in terms of bytes plus close intent
- delayed write and close submissions through `StreamMailbox`
- read pausing and resume-all commands for upstream backpressure
- deferred-read replay so a handler can retain already-read bytes until the
  mailbox resumes reads
- cooperative stop via a stop flag
- interrupted-poll retry so native signals can complete cooperative shutdown
- macOS/BSD convenience binding through `transport_platform::bsd::KqueueTransportPlatform`

## Development

```bash
cargo test -p stream-reactor
cargo check -p stream-reactor --tests --target x86_64-unknown-linux-gnu
cargo check -p stream-reactor --tests --target x86_64-pc-windows-msvc
```


## Bounded spacing and transport reconciliation

Pending fan-out adoption has a separate max_connections quota. Reservations follow commands through drained vectors, errors and callback unwinding. Shrinking the setter preserves admitted reservations and refuses new admission until below the new limit. Stop, serve exit/error/unwind and reactor Drop close admission; shutdown closes it before application callbacks. Existing optional wake handles are used; Windows uses its configured finite poll timeout. Stop cannot preempt a permanently blocked callback.

Validation is source-attributed: Windows applicable packages and standalone
checks are recorded separately from Unix-only/hosted and whole-PR gates.
Final-source Miri is **DEFERRED/PENDING**, never passed.
