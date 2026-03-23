# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `LUT` -- K-input Look-Up Table with SRAM storage and MUX tree evaluation
- `Slice` -- 2 LUTs + 2 D flip-flops + output MUXes + carry chain
- `CLB` -- Configurable Logic Block with 2 slices and carry chain interconnect
- `SwitchMatrix` -- programmable routing crossbar with connect/disconnect/route
- `IOBlock` -- bidirectional I/O pad with INPUT, OUTPUT, TRISTATE modes
- `Bitstream` -- JSON/object configuration format with fromJSON/fromObject
- `FPGA` -- top-level fabric model that assembles and configures all components
- `SliceOutput`, `CLBOutput` interfaces for evaluation results
- `SliceConfig`, `CLBConfig`, `RouteConfig`, `IOConfig` configuration types
- Full test suite with >80% coverage
