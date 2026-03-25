# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-25

### Added

- `ControlFlow` enum (`CONTINUE` / `EXIT`) for handler return values
- `EventSource[E]` Protocol (structural subtyping via `@runtime_checkable`) with `def poll(self) -> List[E]`
- `EventLoop[E]` generic class with three-phase loop (collect → dispatch → idle)
- `add_source()`, `on_event()`, `run()`, `stop()` methods
- `threading.Event`-based stop for thread-safe exit signalling
- `time.sleep(0)` idle yield to avoid busy-spinning
- 12 unit tests; 100% statement coverage
