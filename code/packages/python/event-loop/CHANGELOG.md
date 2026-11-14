# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- Two tests that stop the loop from another thread now wait for the loop to start
  instead of sleeping 10 ms first. A stop requested before `Run()` starts is erased (`Run()` resets the stop flag on entry), so a test that stops the loop from another thread after a fixed 10 ms sleep hangs forever whenever `Run()` starts late. On a loaded CI runner that hung the F# shard for 150 minutes until the step timeout. The stopping thread now waits for a source that signals "the loop is polling" before it stops. Test-only change; the library is unchanged. Verified by injecting a 100 ms delay before `Run()`: the old tests hang, the new ones pass.

## [0.1.0] - 2026-03-25

### Added

- `ControlFlow` enum (`CONTINUE` / `EXIT`) for handler return values
- `EventSource[E]` Protocol (structural subtyping via `@runtime_checkable`) with `def poll(self) -> List[E]`
- `EventLoop[E]` generic class with three-phase loop (collect → dispatch → idle)
- `add_source()`, `on_event()`, `run()`, `stop()` methods
- `threading.Event`-based stop for thread-safe exit signalling
- `time.sleep(0)` idle yield to avoid busy-spinning
- 12 unit tests; 100% statement coverage
