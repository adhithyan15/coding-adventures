# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- `TransportPlatform` trait defining the runtime-facing listener, stream,
  timer, wakeup, and polling contract
- opaque resource identifiers and normalized `PlatformEvent` values
- macOS/BSD `KqueueTransportPlatform` provider
- integration tests covering accept/read/write flow, timers, and wakeups
