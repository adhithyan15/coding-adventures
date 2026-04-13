# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- `CodingAdventures::Intel8008GateLevel` — gate-level Intel 8008 simulator where
  every arithmetic and logical operation routes through gate functions from
  `CodingAdventures::LogicGates` and `CodingAdventures::Arithmetic`. Provides an
  identical public API to the behavioral simulator so the two can be cross-validated
  instruction-by-instruction.

- **`CodingAdventures::Intel8008GateLevel::Bits`** — bit-conversion helpers:
  - `int_to_bits($value, $width)` — integer to LSB-first bit array.
  - `bits_to_int(\@bits)` — bit array to integer.
  - `compute_parity(@bits)` — even-parity flag via `NOT(XORn(@bits))`, matching
    the 8008 hardware convention (P=1 means even parity).

- **`CodingAdventures::Intel8008GateLevel::ALU`** — 8-bit gate-level ALU:
  - `alu_add($a, $b, $carry_in)` — 8-bit addition via `ripple_carry_adder`.
  - `alu_sub($a, $b, $borrow_in)` — subtraction via two's complement (NOT each
    bit of B, add with carry=1). Carry-out inverted to borrow convention (CY=1
    means borrow occurred, i.e., unsigned A < B).
  - `alu_and`, `alu_or`, `alu_xor` — 8 bitwise AND/OR/XOR gates each.
  - `alu_inr`, `alu_dcr` — increment/decrement using adder; preserve carry flag.
  - `alu_rlc`, `alu_rrc`, `alu_ral`, `alu_rar` — four rotate operations matching
    8008 hardware behaviour (RLC/RRC rotate through accumulator only; RAL/RAR
    rotate through carry).
  - `compute_flags($result, $carry)` — zero/sign/parity/carry from 8-bit result.

- **`CodingAdventures::Intel8008GateLevel::Registers`** — 7×8-bit register file:
  - Each register is modelled as 8 D flip-flop state hashrefs via `Register()` from
    `CodingAdventures::LogicGates`.
  - Two-phase clock write (clock=0 → master latch, clock=1 → slave output).
  - Index mapping: B=0, C=1, D=2, E=3, H=4, L=5, M=6 (not physical), A=7.
  - `hl_address($file)` — computes 14-bit address as `(H & 0x3F) << 8 | L`.

- **`CodingAdventures::Intel8008GateLevel::Decoder`** — combinational instruction
  decoder: extracts all 8 opcode bits as named signals, computes group (bits[7:6])
  and instruction-type signals (is_hlt, is_inr, is_dcr, is_rot, is_mvi, is_ret,
  is_rst, is_out, is_in, is_alu_r, is_alu_i) using AND/OR/NOT gate trees.

- **`CodingAdventures::Intel8008GateLevel::Stack`** — 8-level push-down stack:
  - 8 × 14-bit integer entries; entry 0 is always the PC.
  - `push_stack($stack, $target)` — rotates entries down, loads target into [0].
  - `pop_stack($stack)` — rotates entries up, clears deepest slot.

- Comprehensive Test2::V0 test suite in `t/test_intel8008_gatelevel.t` covering
  all instruction groups (MOV, MVI, ALU register, ALU immediate, INR/DCR, rotates,
  jumps, calls/returns, IN/OUT, HLT) and cross-validating gate-level output against
  the behavioral simulator for a multi-instruction program.
