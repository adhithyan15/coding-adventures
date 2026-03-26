# Changelog

## 0.1.0 — 2026-03-21

### Added

- Initial release of the Elixir progress bar package.
- `CodingAdventures.ProgressBar.Tracker` GenServer with `start_link/1`, `send_event/4`, `stop/1` lifecycle.
- `Event` struct with `:started`, `:finished`, `:skipped` event types.
- GenServer/mailbox-based concurrency model — safe to call `send_event` from any process.
- Flat mode: single progress bar with item names and elapsed time.
- Hierarchical mode: `child/3` / `finish/1` for nested progress (e.g., build levels).
- Nil safety: all public functions are no-ops when given `nil` pid.
- Unicode block characters for the bar (`█` / `░`).
- Name truncation: shows up to 3 in-flight names, then "+N more".
- Knuth-style literate programming with detailed `@moduledoc` and `@doc` annotations.
