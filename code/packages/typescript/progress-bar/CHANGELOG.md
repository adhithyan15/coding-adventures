# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the TypeScript progress bar package.
- `Tracker` class with flat and hierarchical progress tracking.
- `NullTracker` class implementing the Null Object pattern for disabled progress.
- `EventType` enum with Started, Finished, and Skipped states.
- `Event` interface for communicating state changes to the tracker.
- `Writable` interface for dependency-injected output (testability).
- `formatActivity` helper for building in-flight name strings.
- Unicode block character rendering (20-char wide bar with filled/empty segments).
- Carriage return (`\r`) line overwriting for smooth terminal updates.
- Parent/child hierarchical tracking via `child()` and `finish()`.
- Comprehensive test suite with mock writer for deterministic assertions.
