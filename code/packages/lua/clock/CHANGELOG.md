# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- `Clock` -- square-wave clock generator with configurable frequency, tick/full_cycle/run methods, and observer pattern (register/unregister listeners)
- `ClockEdge` -- immutable record of a single clock transition (cycle, value, is_rising, is_falling)
- `ClockDivider` -- frequency divider that generates a slower output clock from a source clock by counting rising edges
- `MultiPhaseClock` -- non-overlapping multi-phase clock generator for CPU pipeline simulation
- Full input validation on all constructors and methods
- Comprehensive test suite (93 tests) covering basic behavior, edge detection, cycle counting, listeners, reset, period calculation, clock division, multi-phase rotation, and integration scenarios
- Ported from the Go implementation at `code/packages/go/clock/`
