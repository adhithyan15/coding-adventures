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
