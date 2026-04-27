# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- **`GateLevelCpu`** — top-level gate-level Intel 8008 simulator. All arithmetic
  routes through `GateAlu8` (which wraps the `arithmetic` crate's ripple-carry
  adder), and registers are stored as D flip-flop state arrays driven by
  `logic_gates::sequential::register`.

- **`GateAlu8`** — 8-bit ALU backed by the `arithmetic` crate's `alu()` function
  which itself uses a ripple-carry adder (8 full-adders = 40 gates). Operations:
  `add`, `add_with_carry`, `subtract`, `subtract_with_borrow`, `and`, `or`, `xor`,
  `increment`, `decrement`, plus four rotate variants. Two's complement subtraction
  convention: `CY=1` means borrow occurred (matching 8008 hardware); the arithmetic
  crate's carry output is inverted to produce this.

- **`RegisterFile`** — 8-slot × 8-bit register file. Each slot is a
  `Vec<FlipFlopState>` of 8 elements (LSB-first). Writes simulate the two-phase
  D flip-flop clock cycle via `logic_gates::sequential::register`.

- **`PushDownStack`** — 8-level × 14-bit push-down stack. Each slot is a
  `Vec<FlipFlopState>` of 14 elements. `push_and_jump` rotates slots down;
  `pop_return` rotates up. Used for CALL/RETURN without a traditional stack pointer.

- **`ProgramCounter`** — 14-bit PC stored as a `Vec<FlipFlopState>` of 14 elements.
  Incrementing uses a half-adder chain (14 × 2 = 28 gates): each stage computes
  `sum = XOR(bit, carry)`, `carry = AND(bit, carry)`. Loading a value uses the
  two-phase clock cycle.

- **`decode()`** — Pure gate-logic opcode decoder. All branch decisions are
  expressed as `and_gate`, `or_gate`, `not_gate` calls (no if/else on raw opcodes
  except the three special cases 0x76/0x7C/0x7E). Returns a `DecoderOutput` struct
  with flags for each instruction class and operand fields.

- **`bits.rs`** — Bit-manipulation utilities: `int_to_bits` (u8 → LSB-first Vec),
  `bits_to_int` (LSB-first Vec → u8), `compute_parity` (uses `xor_n` from
  `logic_gates` + `not_gate` to compute 8008 even-parity flag).

- **Cross-validation** — `test_cross_validate` runs a 12-instruction program
  through both `GateLevelCpu` and the behavioral `Simulator`, asserting that every
  trace entry has identical `a_after` and `flags_after`. The gate-level result
  must exactly match the behavioral reference for all arithmetic and control-flow
  paths exercised.

- **`gate_count()`** — Returns a `HashMap<&str, usize>` with gate-count estimates
  for each major component: ripple-carry adder (40), parity XOR tree (7),
  program counter half-adder chain (28), register file flip-flops (384),
  stack flip-flops (672), and decoder AND/OR/NOT gates (≈48).
