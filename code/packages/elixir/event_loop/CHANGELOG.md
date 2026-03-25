# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-25

### Added

- `EventLoop.run/2` — main entry point accepting a list of sources and a list of handlers
- Source abstraction as `{poll_fn, state}` tuples; `poll_fn :: state -> {[events], new_state}`
- Handler convention: `fn event -> :continue | :exit`
- Tail-recursive `loop/2` with `Enum.map_reduce/3` for simultaneous source polling and state evolution
- `Process.sleep(0)` idle yield to avoid busy-spinning
- 8 ExUnit tests covering delivery, exit, stop, multiple handlers/sources, state evolution
