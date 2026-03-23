# Changelog

## 0.1.0 — 2026-03-21

### Added

- `LUT` — Lookup Table with configurable truth table and N-input evaluation
- `Slice` — 2 LUTs + 2 flip-flops + optional carry chain
- `CLB` — Configurable Logic Block containing 2 Slices
- `SwitchMatrix` — Programmable routing crossbar with named ports
- `IOBlock` — Input/Output blocks with input, output, and bidirectional modes
- `Bitstream` — Configuration parser accepting plain Elixir maps
- `Fabric` — Complete FPGA top-level with CLB grid, routing, I/O, and evaluation
