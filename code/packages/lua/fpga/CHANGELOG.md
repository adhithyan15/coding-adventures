# Changelog — coding-adventures-fpga (Lua)

## 0.1.0 — 2026-03-31

### Added
- Complete FPGA simulation library in Lua 5.4
- **LUT** (Lookup Table): N-input truth table element; any Boolean function of N
  variables; `new(n)`, `configure(truth_table)`, `evaluate(inputs)`
- **Slice**: 2 LUTs + 2 D flip-flops + carry chain; optional FF registration
  for sequential logic; carry_enable for adder-style arithmetic;
  `new(opts)`, `configure(config)`, `evaluate(inputs_a, inputs_b, clock, carry_in)`
- **CLB** (Configurable Logic Block): 2 slices with carry chain propagation from
  Slice 0 to Slice 1; grid position tracking (row, col);
  `new(row, col, opts)`, `configure(config)`, `evaluate(inputs, clock, carry_in)`
- **SwitchMatrix**: programmable routing crossbar; output-to-input connections map;
  fan-out supported; `new(n_in, n_out)`, `configure(connections)`, `route(signals)`
- **IOBlock**: external pin interface; input/output/bidirectional modes; output
  enable for tri-state control; `new(name, direction)`, `set_pin()`, `set_fabric()`,
  `set_output_enable()`, `read_fabric()`, `read_pin()`
- **Fabric**: complete FPGA top-level; rows×cols CLB grid with perimeter I/O
  blocks (top/bottom = input/output, left/right = input/output); switch matrix
  per CLB position; `new(rows, cols, opts)`, `load_bitstream()`, `set_input()`,
  `read_output()`, `evaluate()`, `summary()`
- **Bitstream**: configuration data parser from Lua tables; structured as
  `{clbs={}, routing={}, io={}}`; `from_map()`, `clb_config()`, `routing_config()`,
  `io_config()`
- Comprehensive test suite: unit tests for all 7 components + end-to-end tests
  (AND gate programmed onto FPGA, 1-bit full adder in a slice)
