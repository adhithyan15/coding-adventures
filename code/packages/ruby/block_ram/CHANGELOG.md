# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `SRAMCell` -- single-bit storage element modeled at the gate level
- `SRAMArray` -- 2D grid of SRAM cells with row/column addressing
- `SinglePortRAM` -- synchronous single-port RAM with configurable read modes (read-first, write-first, no-change)
- `DualPortRAM` -- true dual-port synchronous RAM with write collision detection
- `ConfigurableBRAM` -- FPGA-style Block RAM with reconfigurable width/depth ratio
- `ReadMode` module with READ_FIRST, WRITE_FIRST, NO_CHANGE constants
- `WriteCollisionError` exception for dual-port write collisions
- Full test suite with >90% coverage
