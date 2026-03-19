# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- Clock struct with tick(), full_cycle(), run(), and reset()
- ClockEdge record type with cycle, value, is_rising, is_falling
- Observer pattern via register_listener() with Box<dyn FnMut>
- ClockDivider for frequency division (manual on_edge pattern)
- MultiPhaseClock for non-overlapping pipeline phase generation
- period_ns() for clock period calculation
- Comprehensive doc comments explaining hardware concepts and Rust ownership model
- Inline unit tests and integration test file
- Ported from Python clock package
