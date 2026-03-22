# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial implementation of gate-level Intel 4004 CPU simulator in Go
- `bits.go`: IntToBits/BitsToInt conversion helpers (LSB-first ordering)
- `alu.go`: GateALU wrapping arithmetic.ALU(4) — Add, Subtract, Complement, Increment, Decrement, BitwiseAnd, BitwiseOr
- `registers.go`: RegisterFile (16x4-bit), Accumulator (4-bit), CarryFlag (1-bit) — all built from D flip-flops
- `pc.go`: ProgramCounter — 12-bit register with half-adder increment chain
- `stack.go`: HardwareStack — 3-level x 12-bit hardware call stack with silent overflow wrapping
- `ram.go`: RAM — 4 banks x 4 registers x 20 nibbles, all stored in D flip-flops
- `decoder.go`: Instruction decoder — combinational AND/OR/NOT gate network for all 46 opcodes
- `cpu.go`: Intel4004GateLevel — full fetch-decode-execute pipeline with GateTrace recording
- `cpu_test.go`: Comprehensive tests covering all instructions, components, and end-to-end programs
- Port of the Python intel4004-gatelevel package to Go
