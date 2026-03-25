# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-25

### Added

- `ControlFlow` enum (`Continue` / `Exit`) for handler return values
- `EventSource<E>` trait with non-blocking `fn poll(&mut self) -> Vec<E>`
- `StopHandle` — cloneable `Arc<AtomicBool>` wrapper for cross-thread stop
- `EventLoop<E>` generic struct with three-phase loop (collect → dispatch → idle)
- `new()` / `default()` constructors
- `add_source()`, `on_event()`, `run()`, `stop()`, `stop_handle()` methods
- `std::thread::yield_now()` idle yield to avoid busy-spinning
- 8 unit tests + 3 doc-tests; all passing
