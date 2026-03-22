# Changelog

All notable changes to the intel4004-gatelevel package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-21

### Added
- Complete Intel 4004 gate-level simulator — all 46 instructions + HLT
- Every computation routes through real logic gates:
  - ALU: uses `ALU(bit_width=4)` from arithmetic package (XOR → AND → OR → full_adder → ripple_carry)
  - Registers: 16 × 4-bit built from D flip-flops (logic_gates.register)
  - Program Counter: 12-bit register with half-adder incrementer chain
  - Hardware Stack: 3 × 12-bit registers with mod-3 pointer
  - Decoder: combinational AND/OR/NOT gate network
  - RAM: 4 banks × 4 registers × 20 nibbles, all in flip-flops
- Gate count estimation: ~8,894 gates
- `Intel4004GateLevel` class with same API as behavioral simulator
- `GateTrace` dataclass for execution tracing
- Cross-validation tests: identical results to behavioral simulator
- 63 tests, 92%+ coverage
