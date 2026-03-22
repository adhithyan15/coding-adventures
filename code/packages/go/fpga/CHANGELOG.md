# Changelog

All notable changes to the `fpga` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-03-21

### Added

- **LUT** (`lut.go`):
  - K-input Look-Up Table (K=2 to 6) with SRAM truth table storage
  - Configure and Evaluate methods using MuxN from logic-gates
  - TruthTable property for inspection

- **Slice** (`slice.go`):
  - 2 LUTs (A and B) for combinational logic
  - 2 D flip-flops for registered outputs
  - Output MUXes selecting between combinational and registered output
  - Carry chain with full-adder carry equation
  - SliceOutput struct

- **CLB** (`clb.go`):
  - Configurable Logic Block with 2 slices
  - Inter-slice carry chain (slice 0 → slice 1)
  - CLBOutput struct

- **Switch Matrix** (`switch_matrix.go`):
  - Programmable routing crossbar with named ports
  - Connect, Disconnect, Clear, Route methods
  - Fan-out support (one source, multiple destinations)
  - Destination contention detection

- **I/O Block** (`io_block.go`):
  - Bidirectional I/O pad with INPUT, OUTPUT, TRISTATE modes
  - DrivePad, DriveInternal, ReadInternal, ReadPad methods
  - Tri-state buffer integration from logic-gates

- **Bitstream** (`bitstream.go`):
  - Configuration data structures: Bitstream, CLBConfig, SliceConfig, RouteConfig, IOConfig
  - FromJSON: load from file
  - FromJSONBytes: parse from bytes
  - FromMap: create programmatically
  - Default LUT truth tables and lut_k=4

- **FPGA Fabric** (`fabric.go`):
  - Top-level FPGA model creating CLBs, switch matrices, I/O blocks from bitstream
  - EvaluateCLB, Route, SetInput, ReadOutput, DriveOutput methods
  - CLBs, Switches, IOs, GetBitstream properties

- **Tests** with high coverage across all components
