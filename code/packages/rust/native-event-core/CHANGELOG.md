# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- generic `Token`, `Interest`, `SourceKind`, `NativeEvent`, and `PollTimeout`
- `EventBackend` trait and `NativeEventLoop` wrapper
- Linux, BSD/macOS, and Windows backend modules
- fake-backend unit tests and macOS/BSD kqueue integration tests

### Fixed

- backend event translation now uses direct token metadata lookups instead of
  scanning all registered sources per event
