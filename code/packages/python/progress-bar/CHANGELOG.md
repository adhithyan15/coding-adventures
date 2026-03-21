# Changelog

## 0.1.0 — 2026-03-21

### Added

- Initial release of the progress bar package.
- `Tracker` class with `start()`, `send()`, `stop()` lifecycle.
- `Event` dataclass with `STARTED`, `FINISHED`, `SKIPPED` event types.
- Queue-based concurrency model — safe to call `send()` from any thread.
- Flat mode: single progress bar with item names and elapsed time.
- Hierarchical mode: `child()` / `finish()` for nested progress.
- `NullTracker` for no-op usage when progress tracking is disabled.
- Unicode block characters for the bar (`█` / `░`).
- Name truncation: shows up to 3 in-flight names, then "+N more".
