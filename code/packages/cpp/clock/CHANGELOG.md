# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `clock` crate, in namespace
  `ca::clk`: a digital-clock simulator.
- `Clock` with `tick` / `full_cycle` / `run` (returns `std::vector<ClockEdge>`),
  `register_listener` (`std::function<void(const ClockEdge&)>` ~ `Box<dyn
  FnMut>`), `reset`, and the period/ticks/frequency/cycle/value accessors.
- `ClockDivider` (`source / divisor`) and `MultiPhaseClock` (N rotating phases).
- Faithful divergences: Rust `new` panics become `std::invalid_argument`;
  `get_phase` throws `std::out_of_range` out of range.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): tick edges, full cycles,
  run, period, reset, listeners, throwing boundaries, and functional divider /
  multi-phase checks — mirroring the Rust crate's tests.
