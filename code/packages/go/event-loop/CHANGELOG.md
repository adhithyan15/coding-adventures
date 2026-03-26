# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-25

### Added

- `ControlFlow` type (`Continue` / `Exit`) for handler return values
- `EventSource[E]` interface with non-blocking `Poll() []E`
- `EventLoop[E]` generic struct with three-phase loop (collect → dispatch → idle)
- `New[E]()` constructor
- `AddSource()`, `OnEvent()`, `Run()`, `Stop()` methods
- `runtime.Gosched()` idle yield to avoid busy-spinning at 100% CPU
- Full test suite: 7 tests covering delivery, exit, stop, multiple handlers/sources
- 100% statement coverage
