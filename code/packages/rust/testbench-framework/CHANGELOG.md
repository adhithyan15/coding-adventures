# Changelog — testbench-framework

## [0.1.0] — 2026-06-13

### Added

- `DutHandle` wrapping `HardwareVm` with `get(name) -> i64` and `set(name, value)`; exposes `vm_mut()` for advanced use (e.g. coverage recording)
- `TestCase` with builder methods `with_timeout(f64)` and `expect_fail()`
- `run(hir, tests)` — fresh VM per test, panic capture via `catch_unwind(AssertUnwindSafe(...))`
- `TestReport` with `all_passed()` and `summary()` methods
- Global thread-local registry: `register_test`, `discover`, `clear_registry`
- `exhaustive` stimulus helper (up to 20 input bits, `Err` on overflow)
- `random_stimulus` using xorshift64 PRNG (Marsaglia 2003, period 2^64−1) for deterministic reproducible sequences
- 18 integration tests + 5 doctests; all pass

### Notes

- Rust port of the Python `testbench_framework` package
- Timeout enforcement is informational only in v0.1.0; a watcher thread is planned for v0.2.0
