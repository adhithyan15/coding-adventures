# Changelog

All notable changes to the `coding-adventures-clock` package will be documented in this file.

## [0.1.0] - 2026-03-18

### Added

- `Clock` class: square-wave generator with configurable frequency
  - `tick()`: advance one half-cycle, returns `ClockEdge`
  - `full_cycle()`: execute one complete cycle (rising + falling)
  - `run(n)`: execute N complete cycles, returns all edges
  - `register_listener()` / `unregister_listener()`: observer pattern
  - `reset()`: restore initial state (preserves listeners)
  - `period_ns` property: clock period in nanoseconds
  - `total_ticks` property: total half-cycles elapsed
- `ClockEdge` dataclass: records cycle, value, is_rising, is_falling
- `ClockDivider` class: derives slower clocks from a fast master clock
- `MultiPhaseClock` class: generates N non-overlapping clock phases
