# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial implementation of gate-level Intel 4004 CPU simulator in Rust.
- `bits` module: `int_to_bits()` and `bits_to_int()` for LSB-first bit vector conversion.
- `gate_alu` module: 4-bit ALU with add, subtract, complement, increment, decrement, bitwise AND/OR -- all routing through the arithmetic crate's real gate-level ALU.
- `registers` module: 16x4-bit RegisterFile, 4-bit Accumulator, and 1-bit CarryFlag, all built from D flip-flops via the logic-gates crate.
- `decoder` module: combinational instruction decoder using AND/OR/NOT gates to pattern-match all 4004 opcode families.
- `pc` module: 12-bit ProgramCounter with half-adder chain incrementer.
- `stack` module: 3-level x 12-bit HardwareStack with circular pointer and silent overflow (matching real 4004 behavior).
- `ram` module: 4 banks x 4 registers x 20 nibbles of RAM, each nibble stored in 4 D flip-flops.
- `cpu` module: Intel4004GateLevel top-level CPU implementing all 46 instructions with GateTrace execution logging.
- 88 unit tests + 4 doc tests covering all modules and instruction families.
- BUILD file for CI integration.
