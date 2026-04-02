# Changelog — coding-adventures-arm1-gatelevel (Lua)

## 0.1.0 — 2026-03-31

### Added
- Gate-level ARM1 simulator: every ALU and barrel-shift operation routes through
  logic gate function calls from coding_adventures.logic_gates and
  coding_adventures.arithmetic.adder
- `int_to_bits(v, w)` / `bits_to_int(bits)`: 1-indexed LSB-first bit array helpers
- `gate_barrel_shift`: 5-level Mux2 tree implementation of ARM1 barrel shifter
  - LSL, LSR, ASR, ROR each modelled as a cascade of 32 mux2 calls per level
  - RRX (rotate right through carry) correctly handles carry_in→MSB, LSB→carry_out
- `gate_alu_execute`: all 16 ARM1 ALU operations through gate primitives
  - AND/EOR/ORR/BIC/MVN/MOV: bit-by-bit gate calls (32 per instruction)
  - ADD/ADC/SUB/SBC/RSB/RSC: ripple_carry_adder (~160 gate calls for 32-bit)
  - TST/TEQ/CMP/CMN: flag-only variants (write_result=false)
  - Signed overflow detection via AND(XNOR(sa,sb), XOR(sa,sr))
- Gate-level condition evaluation for all 16 ARM1 conditions using AND/OR/XOR/NOT/XNOR
- `gate_ops` counter on CPU struct: tracks cumulative gate function calls
- Full integration with arm1_simulator for instruction decode, memory, and
  branch/load-store/block-transfer operations
- Comprehensive test suite: bit helpers, barrel shifter, all 16 ALU ops,
  conditional execution, and sum-1-to-10 integration test
