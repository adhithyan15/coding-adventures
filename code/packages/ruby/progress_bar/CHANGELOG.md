# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the `coding_adventures_progress_bar` gem.
- `Tracker` class: thread-safe progress bar using `Thread::Queue` and a background rendering thread.
- `NullTracker` class: no-op stand-in with the same interface for disabling progress display.
- `Event` data class with three event types: `STARTED`, `FINISHED`, `SKIPPED`.
- Flat mode: single-level progress bar with optional label prefix.
- Hierarchical mode: parent/child trackers where `child.finish` advances the parent.
- Unicode block characters for bar rendering (20-character width).
- In-flight name display: up to 3 names sorted alphabetically, with "+N more" overflow.
- Elapsed time display in seconds.
- Carriage return (`\r`) line overwriting for terminal rendering.
- Comprehensive minitest test suite with 95%+ coverage target.
