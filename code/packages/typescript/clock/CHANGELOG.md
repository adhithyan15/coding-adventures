# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial TypeScript port from Python implementation.
- `Clock` class: square-wave generator with configurable frequency, cycle counting, tick/fullCycle/run methods, and observer pattern (registerListener/unregisterListener).
- `ClockEdge` interface: records cycle number, signal value, and rising/falling edge flags.
- `ClockListener` type: callback type for edge observers.
- `ClockDivider` class: derives slower clocks from a faster master clock by integer division.
- `MultiPhaseClock` class: generates multiple non-overlapping clock phases for pipeline stages.
- `periodNs` getter: computes clock period in nanoseconds from frequency.
- `reset()` method: restores clock to initial state while preserving listeners.
- Full test suite ported from Python with 100% function and branch coverage.
- Knuth-style literate programming comments throughout all source files.
