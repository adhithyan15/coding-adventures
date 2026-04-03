# Changelog — coding-adventures-arm1-gatelevel (Lua)

## 0.1.0 — 2026-03-31

### Added
- Complete ARM1 gate-level processor simulator in Lua 5.4
- Every arithmetic operation routes through logic gate function calls from the
  logic_gates and arithmetic packages
- **Gate-level ALU**: all 16 ARM1 opcodes (AND, EOR, SUB, RSB, ADD, ADC, SBC,
  RSC, TST, TEQ, CMP, CMN, ORR, MOV, BIC, MVN); logical ops via 32-parallel
  AND/OR/XOR/NOT calls; arithmetic ops via ripple_carry_adder (~160 gate calls)
- **Gate-level barrel shifter**: 5-level Mux2 tree; LSL/LSR/ASR/ROR (immediate
  and register-controlled); RRX (rotate right through carry); ~160 Mux2 calls
  per shift
- **Gate-level condition evaluator**: all 16 ARM1 conditions using AND/OR/XOR/NOT
  gate calls (4-6 gate calls per condition)
- **Bit conversion helpers**: `int_to_bits(v, w)` and `bits_to_int(bits)` bridge
  the integer API and gate-level bit arrays (LSB-first)
- **Register file**: 27 physical registers stored as 32-bit LSB-first arrays;
  mode banking (USR/FIQ/IRQ/SVC) same as behavioral simulator
- **Full ARMv1 instruction set**: data processing, load/store, block transfer,
  branch, SWI; same decode and execution logic as behavioral simulator
- **gate_ops counter**: tracks cumulative gate function call count per CPU
- **Encoding helpers**: delegates to arm1_simulator for all encode_* functions
- Comprehensive test suite: bit conversion, all ALU opcodes, barrel shifter
  (LSL/LSR/ASR/ROR/RRX), full simulation tests matching behavioral simulator
  output, end-to-end sum 1..10 = 55 program
