# Changelog — coding-adventures-fpga (Lua)

## 0.1.0 — 2026-03-31

### Added
- Complete FPGA fabric simulation in Lua 5.4
- LUT: N-input lookup table with truth-table programming; MSB-first input addressing
- Slice: 2 LUTs + 2 D flip-flops + carry chain; registered and combinational modes
- CLB: 2 slices with carry propagation Slice0→Slice1
- SwitchMatrix: programmable routing crossbar with validated port names; fan-out allowed
- IOBlock: input/output/bidirectional I/O with output enable control
- Bitstream: configuration data structure with clb/routing/io sections
- Fabric: rows×cols CLB grid with perimeter I/O blocks; bitstream loading; evaluate(clock)
- Comprehensive test suite covering all components
- End-to-end AND gate and 1-bit full adder tests
