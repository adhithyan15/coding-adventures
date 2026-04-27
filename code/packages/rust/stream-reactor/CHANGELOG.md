# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- `StreamReactor` generic over `transport-platform`
- neutral `StreamHandlerResult` for bytes plus close intent
- connection caps and queued-write budget caps
- macOS/BSD `bind_kqueue` convenience constructor
- macOS/BSD end-to-end echo and budget/cap/shutdown tests

## [0.1.1] - 2026-04-18

### Added

- `bind_with_state` and `bind_kqueue_with_state` for connection-local
  application state
- close callbacks that receive the final connection state exactly once
- tests covering state persistence across reads and close callback teardown

### Fixed

- stabilized the stateful read/write test to tolerate delayed client-side
  readability on CI

## [0.1.2] - 2026-04-21

### Added

- Added a thread-safe outbound mailbox so off-reactor worker threads can queue
  writes or close requests by `ConnectionId`.
- Added tests for delayed mailbox writes, write-and-close delivery, and stale
  mailbox commands for already closed streams.

## [0.1.3] - 2026-04-22

### Added

- Added mailbox read pause/resume commands for backpressure-aware adapters.
- Added deferred-read replay so a handler can ask the reactor to retry an
  already-read byte chunk after reads are resumed.
- Added a regression test proving deferred bytes are not lost across pause and
  resume.
