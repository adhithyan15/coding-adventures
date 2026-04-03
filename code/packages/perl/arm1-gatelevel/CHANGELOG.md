# Changelog — CodingAdventures::ARM1Gatelevel (Perl)

## 0.01 — 2026-03-31

### Added
- Gate-level ARM1 simulator: every ALU and barrel-shift operation routes through
  logic gate function calls from CodingAdventures::LogicGates and
  CodingAdventures::Arithmetic
- `int_to_bits($v, $w)` / `bits_to_int(\@bits)`: 0-indexed LSB-first bit array helpers
- `gate_barrel_shift`: 5-level Mux2 tree implementation of the ARM1 barrel shifter
  - LSL, LSR, ASR, ROR each modelled as cascade of 32 mux2 calls per level (5 levels)
  - RRX (rotate right through carry) correctly handles carry_in→MSB, LSB→carry_out
  - Special cases: LSL/LSR #32, ASR #32, ROR by multiples of 32
- `gate_alu_execute`: all 16 ARM1 ALU operations via gate primitives
  - AND/EOR/ORR/BIC/MVN/MOV: 32 gate calls per op (bit-by-bit)
  - ADD/ADC/SUB/SBC/RSB/RSC: ripple_carry_adder from CodingAdventures::Arithmetic
  - TST/TEQ/CMP/CMN: flag-only variants (write_result=0)
  - Signed overflow detection via XNOR and XOR on sign bits
- `_eval_cond`: gate-level condition evaluation for all 16 ARM1 conditions
- `gate_ops` counter: cumulative gate function calls per CPU object
- Complete integration with CodingAdventures::ARM1Simulator for instruction
  decode, register banking, memory, and encoding helper delegation
- Comprehensive test suite: bit helpers, barrel shifter edge cases, all 16 ALU
  operations, conditional execution, and sum-1-to-10 integration test (R1=55)
