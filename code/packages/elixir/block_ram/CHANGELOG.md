# Changelog

## 0.1.0 — 2026-03-21

### Added

- `SRAMCell` — 1-bit storage element with read/write enable
- `SRAMArray` — M x N grid of SRAM cells with address-based access
- `SinglePortRAM` — memory with one read/write port and chip enable
- `DualPortRAM` — memory with two independent ports, Port A wins on conflict
- `ConfigurableBRAM` — FPGA-style BRAM with configurable aspect ratios and modes
  - Supports `:single_port`, `:dual_port`, and `:simple_dual_port` modes
  - `init_data/2` for ROM-style initialization
