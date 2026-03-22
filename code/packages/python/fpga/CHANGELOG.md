# Changelog

All notable changes to the `fpga` package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- **LUT (Look-Up Table)**: K-input look-up table (K=2 to 6)
  - Stores truth table in SRAM cells
  - Uses MUX tree (via `mux_n`) for evaluation
  - Configurable/reprogrammable — any boolean function of K variables
  - Can implement AND, OR, XOR, or any arbitrary function

- **Slice**: Building block of a CLB
  - 2 LUTs (A and B) for combinational logic
  - 2 D flip-flops for registered (sequential) outputs
  - Output MUXes to select combinational vs registered
  - Carry chain with standard full-adder carry equation

- **CLB (Configurable Logic Block)**: Core compute tile
  - Contains 2 slices (4 LUTs total)
  - Carry chain propagation from slice 0 to slice 1
  - External carry input for inter-CLB arithmetic

- **SwitchMatrix**: Programmable routing crossbar
  - Named ports with configurable connections
  - Fan-out support (one source, multiple destinations)
  - Connect, disconnect, clear, and route operations

- **IOBlock**: Bidirectional I/O pad
  - Three modes: INPUT, OUTPUT, TRISTATE
  - Tri-state uses logic-gates `tri_state` buffer
  - Pad and internal signal interfaces

- **Bitstream**: JSON-based FPGA configuration format
  - `from_json` and `from_dict` loading
  - Parses CLB, routing, and I/O configurations
  - Sensible defaults for missing fields

- **FPGA**: Top-level fabric model
  - Creates and configures CLBs, switch matrices, I/O blocks from bitstream
  - `evaluate_clb`, `route`, `set_input`, `read_output`, `drive_output`

- **JSON Examples**: Three worked configuration examples
  - `and_gate.json`: Simple AND gate in one LUT
  - `two_bit_adder.json`: 2-bit ripple-carry adder with carry chain
  - `registered_counter.json`: Toggle flip-flop using registered output
