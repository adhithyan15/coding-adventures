# Changelog — CodingAdventures::FPGA (Perl)

## 0.01 — 2026-03-31

### Added
- Complete FPGA fabric simulation in pure Perl
- LUT: N-input lookup table with MSB-first truth-table addressing
- Slice: 2 LUTs + 2 D flip-flops + carry chain; registered and combinational output modes
- CLB: 2 slices with carry propagation Slice0→Slice1
- SwitchMatrix: validated programmable routing crossbar; fan-out allowed
- IOBlock: input/output/bidirectional I/O with output enable
- Bitstream: configuration data structure (clbs/routing/io sections)
- Fabric: rows×cols CLB grid with perimeter I/O; bitstream loading; evaluate($clock)
- Comprehensive test suite with end-to-end AND gate and carry chain adder tests
