# Changelog

## 0.1.0 — 2026-03-21

### Added

- Initial release of the progress bar package.
- `Tracker` type with `Start()`, `Send()`, `Stop()` lifecycle.
- `Event` type with `Started`, `Finished`, `Skipped` event types.
- Channel-based concurrency model — safe to call `Send()` from any goroutine.
- Flat mode: single progress bar with item names and elapsed time.
- Hierarchical mode: `Child()` / `Finish()` for nested progress (e.g., build levels).
- Nil safety: all methods are no-ops on a nil `*Tracker`.
- Unicode block characters for the bar (`█` / `░`).
- Name truncation: shows up to 3 in-flight names, then "+N more".
