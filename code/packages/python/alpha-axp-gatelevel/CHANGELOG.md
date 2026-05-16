# Changelog — coding-adventures-alpha-axp-gatelevel

All notable changes to this package are documented here.

## [0.1.0] — 2026-05-15

### Added

- Initial implementation: DEC Alpha AXP 21064 (1992) gate-level simulator.
- **`bits.py`** — bridge between the integer world and the gate world:
  - `int_to_bits(value, width)` / `bits_to_int(bits)` — LSB-first bit lists
  - `add_64bit`, `add_128bit`, `add_32bit` — ripple-carry adder wrappers
  - `invert_64bit`, `invert_32bit` — bitwise NOT via gate calls
  - `shl_64`, `shr_64_logical`, `shr_64_arith` — shift helpers
  - `sext32_to_64` — sign-extension via gate operations
  - `compute_zero` — all-zero NOR tree via gate calls
- **`alu.py`** — gate-level ALU for all 64-bit data-path operations:
  - `addq`, `subq` — 64-bit addition/subtraction (subq uses invert + carry=1)
  - `addl`, `subl`, `mull` — 32-bit longword variants with sign-extension
  - `andq`, `orq`, `xorq`, `bicq`, `ornot`, `eqvq` — 64 gate calls per bit
  - `sll64`, `srl64`, `sra64` — logical and arithmetic shifts
  - `cmpeq`, `cmplt`, `cmple`, `cmpult`, `cmpule` — 0/1 comparison results
  - `s4addq`, `s4addl`, `s8addq`, `s8addl` — scaled add (×4, ×8)
  - `s4subq`, `s4subl`, `s8subq`, `s8subl` — scaled subtract
  - `mulq` — 64×64 shift-and-add multiply (64 iterations via `add_64bit`)
  - `umulh` — upper 64 bits of 128-bit product (via `add_128bit`)
  - No Python arithmetic operators in execution path
- **`register_file.py`** — `RegisterFile64`: 32 GPRs + PC stored as bit lists;
  r31 hardwired to zero; `increment_pc` uses gate-level adder.
- **`decoder.py`** — `decode_instruction`: extracts all Alpha instruction
  fields (op, ra, rb, rc, func7, i_bit, lit8, disp16, disp21, jump_func,
  palcode) with mnemonic lookup tables.
- **`simulator.py`** — `AlphaAXPGateLevelSimulator` implementing the
  `Simulator[AlphaState]` protocol:
  - All INTA instructions: ADDL/Q, SUBL/Q, S4/S8 ADD/SUB L/Q, CMPEQ/LT/LE,
    CMPULT/CMPULE, CMPBGE, MULL/Q (overflow variants)
  - All INTL instructions: AND, BIC, BIS (OR), ORNOT, XOR, EQV,
    CMOVLBS, CMOVLBC, CMOVEQ, CMOVNE, CMOVLT, CMOVGE, CMOVLE, CMOVGT,
    AMASK, IMPLVER
  - All INTS instructions: SLL, SRL, SRA (both func=0x3A and 0x3C),
    ZAP, ZAPNOT, EXTBL/EXTWL/EXTLL/EXTQL, INSBL/INSWL/INSLL/INSQL,
    MSKBL/MSKWL/MSKLL/MSKQL, SEXTB, SEXTW
  - INTM: MULL, MULQ, UMULH
  - Memory: LDA, LDAH, LDL/LDL_L, LDQ/LDQ_L, LDQ_U, LDBU, STL/STL_C,
    STQ/STQ_C, STQ_U (with alignment checking)
  - Branches: BR, BSR, BEQ, BNE, BLT, BLE, BGT, BGE, BLBC, BLBS (FP branches
    treated as NOP)
  - Jumps: JMP, JSR, RET, JSR_COROUTINE
  - PALcode: HALT (palcode=0); all other PALcodes raise ValueError
  - CMOV conditions use gate-level `compute_zero`, `AND`, `NOT`, sign-bit
    extraction — no Python conditionals on integer values in data path
  - ZAP/ZAPNOT use 64 gate-level AND calls (one per bit)
- **`tests/`** — 351 tests at 98.91% coverage:
  - `test_bits.py` — 65 tests: round-trips, carries, overflows, shifts,
    sext32, compute_zero
  - `test_alu.py` — 100 tests: all ALU operations with edge cases
  - `test_register_file.py` — 18 tests: all GPRs, r31 guard, PC, reset
  - `test_decoder.py` — 29 tests: all instruction formats and opcodes
  - `test_programs.py` — 20 tests: full programs (halt, load-immediate,
    sum 1..10 loop, MULQ 6×7=42, max via CMPLT+CMOVNE, ZAP/ZAPNOT,
    BSR/RET subroutine, STQ/LDQ roundtrip, LDL sign-extension)
  - `test_equivalence.py` — 25 tests: cross-validates gate-level vs
    behavioral AlphaSimulator across arithmetic, bitwise, shift, compare,
    MULQ, and branch instruction classes
  - `test_simulator_coverage.py` — 94 tests: error paths, CMOV variants,
    scaled add/sub, byte manipulation, branch variants, UMULH, MULL, LDAH,
    LDBU, LDQ_U, STQ_U, alternate func codes

### Technical notes

- Overflow detection uses split 63-bit + 1-bit ripple approach: `XOR(carry_63,
  carry_64)` — exact mirror of hardware overflow detection logic.
- Two's complement subtraction: `a - b = a + NOT(b) + 1` (invert_64bit + addq
  with carry_in=1) — no Python subtraction in ALU.
- `umulh` uses a 128-bit accumulator built from `add_128bit`, which itself
  chains two 64-bit ripple-carry adders.
- The conftest.py at package root works around the macOS UF_HIDDEN flag issue
  in Python 3.13 + uv editable installs: it manually reads `_editable_impl_*.pth`
  files and inserts their paths into `sys.path`.
