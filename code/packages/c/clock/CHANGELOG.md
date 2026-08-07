# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `clock` crate: a digital-clock simulator.
- `Clock` — `clock_new` / `clock_free`, `tick` (returns a `ClockEdge`),
  `full_cycle`, `run` (malloc'd `2·cycles` edge array, overflow-guarded),
  `register_listener` (C callback + userdata), `reset`, and the
  period/ticks/frequency/cycle/value getters.
- `ClockDivider` (`source / divisor`, driven by `on_edge`) and `MultiPhaseClock`
  (N non-overlapping rotating phases via `on_edge` / `get_phase`).
- Faithful divergences: Rust `new` panics become NULL returns (invalid frequency
  / divisor / phases); `mpc_get_phase` returns 0 out of range; the `Box<dyn
  FnMut>` listener becomes a callback + `void *userdata`.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): tick edges, full cycles,
  run, period, reset, listeners, invalid-argument boundaries, and functional
  divider / multi-phase checks — mirroring the Rust crate's tests.
