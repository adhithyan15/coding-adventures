# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Drain actual terminal cancellation packets before releasing Windows operation storage; retain outstanding allocations/port on fatal wait errors. Correct AcceptEx BOOL handling. Add real pending accepts/read and unadopted-socket controls, preserving final-bytes/EOF controls.
- Final-source Miri **DEFERRED/PENDING**; native/hosted and whole-PR gates remain separately attributed.

- Windows `close_stream` now shuts the socket down for sending before
  dropping it. Winsock turns `closesocket()` into an RST while an overlapped
  `WSARecv` is pending (this provider nearly always has one), and the peer
  then discards bytes it had not read yet: `websocket-runtime`'s Close echo
  arrived as `ConnectionReset`. New `close_after_write_delivers_data_then_eof`
  test on Linux and Windows.
- The Windows provider implements `adopt_stream`, so a socket accepted by a
  plain `std` listener on another thread can join its completion port (the
  accept fan-out `tcp-runtime` now uses on Windows). The adoption test now
  runs on Windows too.
- The IPv6-only listener test now reserves `127.0.0.1:P` itself before
  binding `[::]:P`, instead of hoping no other process listens on the IPv4
  side of a random port. On main's busy sharded runner an unrelated IPv4
  listener on the same port number made the old test fail although
  `set_only_v6(true)` was in effect.
- Keep the Windows backend clean under Rust 1.97 by documenting the
  platform-specific wake helper and using the standard backlog clamp.

## [0.1.0] - 2026-04-18

### Added

- `TransportPlatform` trait defining the runtime-facing listener, stream,
  timer, wakeup, and polling contract
- opaque resource identifiers and normalized `PlatformEvent` values
- macOS/BSD `KqueueTransportPlatform` provider
- Linux `EpollTransportPlatform` provider backed by `epoll`, `timerfd`, and
  `eventfd`
- Windows `WindowsTransportPlatform` provider backed by nonblocking sockets,
  `WSAPoll`, loopback wakeup sockets, and user-space timers
- integration tests covering accept/read/write flow, timers, and wakeups
- Linux- and Windows-targeted test modules so CI exercises the provider seam on
  those runners
- `required_capabilities.json` declaring that the crate itself does not claim
  extra repository capability requirements

### Fixed

- Linux and Windows listeners now force IPv6 sockets into IPv6-only mode so an
  IPv6 bind does not silently widen into dual-stack exposure
- Windows listeners now use exclusive address ownership instead of mapping the
  cross-platform `reuse_address` flag onto Winsock's unsafe TCP listener
  `SO_REUSEADDR` semantics

## [0.1.1] - 2026-06-14

### Added

- `WakeHandle` — a `Send + Sync`, cloneable, thread-safe trigger for a platform's
  wakeup, and a `TransportPlatform::wake_handle(wakeup)` method that returns one.
  Unlike `wake(&mut self)` (which lives on the reactor-owned platform), a
  `WakeHandle` owns a thread-safe clone of the underlying OS primitive, so an
  off-reactor producer can interrupt the reactor's `poll` the instant it has work.
  - kqueue: duplicates the kqueue fd and re-issues `EVFILT_USER` + `NOTE_TRIGGER`.
  - epoll: duplicates the wakeup `eventfd` and `write`s the counter.
  - Default impl returns `Unsupported`; Windows/IOCP uses it for now (callers fall
    back to the poll timeout), so no platform is broken — they just wake later.

## [0.1.2] - 2026-06-14

### Added

- `TransportPlatform::adopt_stream(fd)` — adopt an already-connected socket
  (owned `AdoptableFd`: `OwnedFd` on Unix, `OwnedSocket` on Windows) as a managed
  stream, returning its `StreamId`. This is the receiving half of an **accept
  fan-out**: a single acceptor accepts on one listener (needed where
  `SO_REUSEPORT` doesn't load-balance, e.g. macOS/BSD) and hands each accepted
  socket to a worker platform on another thread, which adopts it and then drives
  it like any other stream (`configure_stream` + `set_stream_interest`, exactly as
  after `accept`). Implemented for the kqueue and epoll backends; the default
  returns `Unsupported` (Windows/IOCP), so the trait stays cross-platform.
- Test `adopts_externally_accepted_stream` (kqueue + epoll): a separate std
  listener accepts a real loopback connection, its socket is adopted into the
  platform, and the adopted stream reads/writes correctly.
