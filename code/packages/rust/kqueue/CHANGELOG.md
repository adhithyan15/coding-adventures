# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- `Filter`, `EventFlags`, `KqueueChange`, and `KqueueEvent`
- `Kqueue` wrapper with `new`, `apply`, `apply_all`, and `wait`
- macOS/BSD readiness tests and unsupported fallback

### Changed

- widened the wrapper to expose timer and user-event filters for higher
  transport layers such as `transport-platform`
