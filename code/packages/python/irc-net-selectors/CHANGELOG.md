# Changelog — irc-net-selectors

## 0.1.0 — 2026-04-23

### Added

- **`SelectorsEventLoop`** — single-threaded event loop using `selectors.DefaultSelector`
  (epoll on Linux, kqueue on macOS).  Replaces one-thread-per-connection with the reactor
  pattern.  Handles thousands of connections with a single OS thread.

- **`SelectorsConnection`** — non-blocking TCP connection with per-connection write buffer.
  Exposes `read_available()` (drain recv buffer), `enqueue()` (buffer outgoing bytes), and
  `flush()` (push buffered bytes when `EVENT_WRITE` fires).

- **`SelectorsListener`** — non-blocking server socket.  Registered with the selector;
  `accept_connection()` is called when `EVENT_READ` fires.

- **`create_listener(host, port)`** — factory function with identical signature to
  `irc_net_stdlib.create_listener()`.  Drop-in replacement.

- **Stable interface types** re-exported: `ConnId`, `Connection`, `Listener`, `Handler`,
  `EventLoop` — identical protocols to `irc-net-stdlib`.  The `ircd` program changes one
  import to switch implementations.

- **95 %+ test coverage** across 13 test classes covering: echo round-trip, multiple clients,
  `on_connect`/`on_disconnect` events, `send_to` safety, `stop()` behaviour, single-thread
  assertion, write-buffer deregistration, and rapid connect/disconnect stress.

### Design decisions

- **No framing in the transport layer**: raw bytes are passed to `on_data`, keeping
  `irc-net-selectors` protocol-agnostic.  Framing and parsing remain in `ircd`'s
  `DriverHandler` (unchanged from `irc-net-stdlib`).

- **Level-triggered notifications**: `selectors.DefaultSelector` uses level-triggered mode
  by default, matching `select()`/`poll()` semantics.  Edge-triggered mode (with its
  requirement to drain all data in one shot) is introduced at Level 3 (`irc-net-epoll`).

- **Write buffer drain deregistration**: after `flush()` empties the write buffer, `EVENT_WRITE`
  is removed from the selector registration.  Leaving it registered when there is nothing to
  write causes a busy-loop (the kernel's send buffer is almost always writable).
