# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `LUT` -- K-input Look-Up Table with configurable truth table and MUX-tree evaluation
- `Slice` -- 2 LUTs + 2 flip-flops + output MUXes + carry chain
- `CLB` -- Configurable Logic Block with 2 slices and inter-slice carry chain
- `SwitchMatrix` -- programmable routing crossbar with connect/disconnect/route
- `IOBlock` -- bidirectional I/O pad with INPUT, OUTPUT, and TRISTATE modes
- `Bitstream` -- JSON-based FPGA configuration with from_json and from_hash
- `FPGAFabric` -- top-level FPGA model that configures and evaluates the fabric
- `SliceOutput`, `CLBOutput`, `SimResult` structs for structured outputs
- `SliceConfig`, `CLBConfig`, `RouteConfig`, `IOConfig` config structs
- Full test suite with >80% coverage
