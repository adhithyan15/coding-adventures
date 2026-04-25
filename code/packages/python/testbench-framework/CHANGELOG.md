# Changelog

## [0.1.0] — Unreleased

### Added
- `@test` decorator that registers a function in a global registry, optionally with `name=`, `timeout_s=`, `should_fail=` overrides.
- `discover()` returns the current registry; `clear_registry()` empties it.
- `run(hir, tests=None) -> TestReport` runs each test in a fresh `HardwareVM(hir)`, capturing `AssertionError` (failure) and other exceptions, supporting negative tests via `should_fail=True`.
- `DUTHandle` / `SignalHandle`: attribute-access shim over `HardwareVM`. `dut.signal.value` reads via `vm.read`; `dut.signal.value = x` writes via `vm.set_input`.
- `exhaustive(dut, inputs, on_step=None)` — every combination of up to 20 bits worth of inputs.
- `random_stimulus(dut, inputs, iterations, seed=42, on_step=None)` — reproducible random vectors.
- `TestReport.summary()` formats a human-readable line.

### Out of scope (v0.2.0)
- Async / clocked sequential testbenches (waits on hardware-vm v0.2).
- Constrained-random with full constraint solver.
- Scoreboard FIFO helpers.
- Wave dump integration with `vcd-writer`.
