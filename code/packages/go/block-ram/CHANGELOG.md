# Changelog

All notable changes to the `block-ram` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-03-21

### Added

- **SRAM** (`sram.go`):
  - `SRAMCell`: single-bit storage with Read/Write/Value methods, word-line gating
  - `SRAMArray`: 2D grid of SRAM cells with row/column addressing, Read/Write/Shape methods
  - `validateBit` helper for input validation

- **RAM Modules** (`ram.go`):
  - `ReadMode` type with `ReadFirst`, `WriteFirst`, `NoChange` constants
  - `WriteCollisionError` for dual-port write collision detection
  - `SinglePortRAM`: synchronous single-port RAM with configurable read mode, Tick/Dump/Depth/Width methods
  - `DualPortRAM`: true dual-port RAM with per-port read modes, collision detection, Tick/Depth/Width methods

- **Configurable Block RAM** (`bram.go`):
  - `ConfigurableBRAM`: FPGA-style BRAM with reconfigurable aspect ratio
  - `Reconfigure` method to change width/depth while preserving total capacity
  - `TickA`/`TickB` for independent dual-port access
  - Properties: Depth, Width, TotalBits

- **Tests** with high coverage:
  - `sram_test.go`: cell init, read/write with word line, not-selected cases, overwrite, array operations, invalid input
  - `ram_test.go`: single-port write/read, all 3 read modes, multiple addresses, rising edge detection, dump; dual-port independent ports, write collision, read-read same address, write-read different, all read modes
  - `bram_test.go`: create and properties, port A/B operations, reconfigure width, multiple addresses, invalid inputs
