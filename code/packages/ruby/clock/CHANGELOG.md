# Changelog

All notable changes to the `coding_adventures_clock` gem will be documented in this file.

## [0.1.0] - 2026-03-18

### Added

- `ClockGenerator` class: square-wave generator with configurable frequency
  - `tick`: advance one half-cycle, returns `ClockEdge`
  - `full_cycle`: execute one complete cycle (rising + falling)
  - `run(n)`: execute N complete cycles, returns all edges
  - `register_listener` / `unregister_listener`: observer pattern
  - `reset`: restore initial state (preserves listeners)
  - `period_ns`: clock period in nanoseconds
  - `total_ticks`: total half-cycles elapsed
- `ClockEdge` (Data.define): records cycle, value, rising?, falling?
- `ClockDivider` class: derives slower clocks from a fast master clock
- `MultiPhaseClock` class: generates N non-overlapping clock phases
