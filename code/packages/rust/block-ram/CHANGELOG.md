# Changelog

All notable changes to the `block-ram` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `SRAMCell` — single-bit storage element modeled at the gate level
- `SRAMArray` — 2D grid of SRAM cells with row/column addressing
- `SinglePortRAM` — synchronous single-port RAM with configurable read mode (ReadFirst, WriteFirst, NoChange)
- `DualPortRAM` — true dual-port RAM with write collision detection (`WriteCollisionError`)
- `ConfigurableBRAM` — FPGA Block RAM with reconfigurable width/depth aspect ratio
- `ReadMode` enum — controls data output behavior during write operations
- Comprehensive tests for all modules (sram, ram, bram)
