# Changelog

All notable changes to the intel8008-gatelevel package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-12

### Added
- Initial gate-level implementation of the Intel 8008 CPU
- `bits.py`: int-to-bits/bits-to-int conversion helpers; `compute_parity()` via XOR_N gate chain
- `alu.py`: 8-bit `GateALU8` class wrapping `ALU(bit_width=8)` from arithmetic package;
  all operations (add, subtract, bitwise_and/or/xor, rotate, compare, increment, decrement)
  route through real logic gates; `compute_flags()` uses gate functions for zero/sign/parity
- `registers.py`: `RegisterFile` with 7 × 8-bit registers (B,C,D,E,H,L,A) and flag register
- `decoder.py`: Combinational instruction decoder using AND/OR/NOT gate logic;
  produces `DecoderOutput` control signals for every opcode
- `stack.py`: `PushDownStack` — 8-level circular stack where entry 0 is always the PC
- `cpu.py`: `Intel8008GateLevel` top-level class wiring all components;
  implements same public API as `Intel8008Simulator` for cross-validation;
  `gate_count()` estimates total logic gate usage
- Cross-validation tests verifying gate-level and behavioral results match exactly
