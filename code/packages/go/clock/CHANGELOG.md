# Changelog

All notable changes to the `clock` package will be documented in this file.

## [0.1.0] - 2026-03-18

### Added

- `Clock` struct: square-wave generator with configurable frequency
  - `Tick()`: advance one half-cycle, returns `ClockEdge`
  - `FullCycle()`: execute one complete cycle (rising + falling)
  - `Run(n)`: execute N complete cycles, returns all edges
  - `RegisterListener()` / `UnregisterListener()`: observer pattern
  - `Reset()`: restore initial state (preserves listeners)
  - `PeriodNs()`: clock period in nanoseconds
  - `TotalTicks()`: total half-cycles elapsed
  - `ListenerCount()`: number of registered listeners
- `ClockEdge` struct: records Cycle, Value, IsRising, IsFalling
- `ClockDivider` struct: derives slower clocks from a fast master clock
- `MultiPhaseClock` struct: generates N non-overlapping clock phases
