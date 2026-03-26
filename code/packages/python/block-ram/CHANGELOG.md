# Changelog

All notable changes to the `block-ram` package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- **SRAMCell**: Single-bit storage element modeled after a 6-transistor SRAM cell
  - Read/write with word line selection
  - Hold mode (word_line=0 retains value)
  - Full input validation

- **SRAMArray**: 2D grid of SRAM cells with row/column addressing
  - Row-level read and write operations
  - Configurable rows × columns dimensions

- **SinglePortRAM**: Synchronous single-port RAM module
  - Rising-edge-triggered operations
  - Three read modes: READ_FIRST, WRITE_FIRST, NO_CHANGE
  - Full contents dump for debugging

- **DualPortRAM**: True dual-port synchronous RAM
  - Two independent ports (A and B) with separate read modes
  - Simultaneous read/write on different addresses
  - Write collision detection (raises `WriteCollisionError`)

- **ConfigurableBRAM**: FPGA-style Block RAM with reconfigurable aspect ratio
  - Fixed total storage with configurable width/depth
  - Dual-port access via tick_a/tick_b
  - Reconfiguration clears stored data (matches real FPGA behavior)
