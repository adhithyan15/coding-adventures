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

## [0.1.1] - 2026-06-14

### Added

- `Kqueue::try_clone` — duplicates the queue descriptor, yielding a second handle
  to the same kernel queue. Because `apply` takes `&self` and `kevent` is
  thread-safe, the clone can trigger an `EVFILT_USER` wakeup from another thread
  on a queue a different thread is blocked in `wait` on (cross-thread reactor wake).
