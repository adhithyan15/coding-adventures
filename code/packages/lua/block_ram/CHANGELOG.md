# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- SRAMCell: single-bit storage element modeled at the gate level (6T cell)
- SRAMArray: 2D grid of SRAM cells with row/column addressing
- SinglePortRAM: synchronous RAM with one read/write port and three read modes
  - READ_FIRST: output shows old value during writes
  - WRITE_FIRST: output shows new value during writes
  - NO_CHANGE: output retains previous value during writes
- DualPortRAM: true dual-port synchronous RAM with independent ports A and B
  - Write collision detection (both ports writing same address)
  - Independent read modes per port
- ConfigurableBRAM: FPGA-style Block RAM with reconfigurable aspect ratio
  - Fixed total storage, configurable depth/width trade-off
  - Dual-port access via tick_a and tick_b
  - Reconfigure method clears data and rewires address decoder
- Comprehensive busted test suite (82 tests)
- Ported from Go implementation at code/packages/go/block-ram/
