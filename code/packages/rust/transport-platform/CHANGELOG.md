# Changelog

All notable changes to this package will be documented in this file.

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
