# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-25

### Added

- `ControlFlow` string enum (`Continue` / `Exit`) for handler return values
- `EventSource<E>` interface with non-blocking `poll(): E[]`
- `EventLoop<E>` generic class with three-phase loop (collect → dispatch → idle)
- `addSource()`, `onEvent()`, `run()`, `stop()` methods
- 10 vitest tests covering delivery, exit, stop, multiple handlers/sources, order preservation
