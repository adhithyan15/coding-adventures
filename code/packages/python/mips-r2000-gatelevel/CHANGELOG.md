# Changelog — mips-r2000-gatelevel

All notable changes to this package will be documented in this file.

## [1.0.0] — 2026-05-15

### Added

- **`bits.py`** — Integer ↔ bit-list bridge. All conversion between Python ints and
  gate-level `list[int]` representations lives here. Key primitives:
  - `int_to_bits(value, width)` — LSB-first fixed-width bit list
  - `bits_to_int(bits)` — unsigned integer from LSB-first bit list
  - `add_32bit(a, b, carry_in)` — 33-bit ripple-carry adder with signed overflow
    detection via carry_into_bit31 XOR carry_out_of_bit31
  - `add_64bit(a, b, carry_in)` — 64-bit ripple-carry adder for MULT/DIV
  - `invert_32bit(value)` — 32 parallel NOT gates
  - `shl_32`, `shr_32_logical`, `shr_32_arith` — barrel-shifter models via
    bit-list manipulation
  - `compute_zero(bits)` — NOR reduction tree (zero flag)
  - `compute_parity(bits)` — XOR reduction tree

- **`alu.py`** — 32-bit gate-level ALU. EVERY data-path operation routes through
  `AND`, `OR`, `XOR`, `NOT` from `logic_gates` and `ripple_carry_adder` from
  `arithmetic`. No Python arithmetic operators (`+`, `-`, `*`, `/`, `&`, `|`,
  `^`, `~`) appear in the execution path.
  - `add32`, `sub32` — ripple-carry adder / two's complement subtraction
  - `and32`, `or32`, `xor32`, `nor32` — bitwise via gate arrays
  - `slt32`, `sltu32` — signed/unsigned compare via subtraction + flag logic
  - `sll32`, `srl32`, `sra32` — logical/arithmetic shifts via barrel shifter
  - `multu32`, `mult32` — 32-iteration shift-and-add multiplication over 64-bit
    bit lists (avoids overflow truncation)
  - `divu32`, `div32` — 32-iteration long division via 64-bit shifted divisor
    comparison and `sub32` borrow detection

- **`register_file.py`** — `RegisterFile32` with physically modelled storage:
  - 32 general-purpose registers stored as `list[list[int]]` (32×32 bit arrays)
  - R0 hardwired to zero (writes silently discarded, reads return 0)
  - HI, LO, PC stored as 32-bit bit lists
  - `increment_pc(amount)` implemented via `add_32bit` (no `+` operator)

- **`decoder.py`** — Gate-level instruction decoder:
  - Converts a 32-bit word to an LSB-first bit list then extracts fields via
    list slicing (no bit-masking arithmetic in the gate path)
  - Handles R-type (op=0), I-type (imm16), and J-type (target26) formats
  - Sign-extends imm16 by replicating bit 15 across positions 16–31

- **`simulator.py`** — `MIPSR2000GateLevelSimulator(Simulator[MIPSState])`:
  - Implements the full MIPS R2000 integer instruction set (40+ instructions)
  - Instruction fetch is big-endian (4 bytes → word via bit-list concatenation)
  - Branch target = PC_after_fetch + sign_ext(imm16) * 4 (no delay slots)
  - Jump target = (PC_after_fetch & 0xF000_0000) | (target26 << 2)
  - MULT/MULTU write HI:LO; MFHI/MFLO/MTHI/MTLO transfer to/from GPRs
  - LWL/LWR/SWL/SWR implement correct MIPS big-endian unaligned semantics
  - SYSCALL triggers halt; BREAK raises `RuntimeError`
  - Exposes `MIPSState` snapshot via `get_state()` for cross-validation

- **Tests** — 276 tests, 96.78% line coverage (well above the 80% minimum):
  - `test_bits.py` — 30+ tests for every bit-manipulation primitive
  - `test_alu.py` — 80 tests covering all ALU ops, flags, edge cases
  - `test_register_file.py` — all 32 GPRs, HI/LO/PC, increment/wrap
  - `test_decoder.py` — R/I/J format decoding, sign extension
  - `test_equivalence.py` — cross-validation against `mips-r2000-simulator`
    (behavioral reference) on 8 programs
  - `test_programs.py` — end-to-end: sum 1..10, factorial 5, GCD(48,18),
    bitwise, SLT, LW/SW round-trip, branch/jump, overflow detection
  - `test_simulator_coverage.py` — BLEZ/BGTZ, BLTZAL/BGEZAL, MFHI/MTHI,
    SLTI/SLTIU/XORI/ADDI, SLLV/SRLV/SRAV, LWL/LWR/SWL/SWR, error paths,
    max_steps, halted-step guard, I/O stubs
