# Changelog

All notable changes to the `fpga` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `LUT` — K-input look-up table with SRAM-based truth table storage and MUX tree evaluation
- `Slice` — 2 LUTs + 2 D flip-flops + output MUXes + carry chain logic
- `CLB` — Configurable Logic Block with 2 slices and inter-slice carry chain
- `SwitchMatrix` — programmable routing crossbar with named ports, fan-out, and contention detection
- `IOBlock` — bidirectional I/O pad with Input/Output/Tristate modes
- `Bitstream` — JSON-based FPGA configuration parser for CLBs, routing, and I/O
- `FPGA` — top-level fabric model that creates and configures all elements from a bitstream
- Comprehensive integration tests for all modules
