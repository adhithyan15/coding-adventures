# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `SRAMCell` — single-bit gate-level storage element
- `SRAMArray` — 2D grid of SRAM cells with row/column addressing
- `SinglePortRAM` — synchronous single-port memory with READ_FIRST, WRITE_FIRST, and NO_CHANGE read modes
- `DualPortRAM` — true dual-port memory with write collision detection
- `ConfigurableBRAM` — FPGA-style Block RAM with reconfigurable aspect ratio
- `WriteCollisionError` — error thrown on dual-port write collision
- `ReadMode` enum — controls data output behavior during writes
- Full test suite with >80% coverage
