# Changelog

## [1.0.0] — 2026-05-15

### Added

- Initial implementation of the Motorola 68000 gate-level simulator.
- `bits.py` — bit conversion utilities and multi-width ripple-carry adders
  (8, 16, 32-bit) built on the `arithmetic` package's `ripple_carry_adder`.
- `alu.py` — complete ALU with add/sub/and/or/xor/not/neg/cmp at all three
  sizes (byte, word, long), plus all shift/rotate operations (ASL, ASR, LSL,
  LSR, ROL, ROR, ROXL, ROXR) and MULS/MULU/DIVS/DIVU.
- `register_file.py` — `RegisterFile68k` with D0–D7 data registers, A0–A7
  address registers, PC, and individual CCR/SR flag bits stored as bit arrays.
- `decoder.py` — `decode()` function for all major instruction classes:
  MOVE, ADD, SUB, AND, OR, XOR, CMP, shifts, branches, misc.
- `simulator.py` — `Motorola68kGateLevelSimulator` implementing
  `Simulator[M68KState]` with:
  - All 12 effective address modes
  - 16 Bcc conditions, DBcc, Scc
  - MOVEM, LINK/UNLK, BSR/RTS/RTR/RTE
  - MULS/MULU/DIVS/DIVU
  - ABCD/SBCD/NBCD BCD arithmetic
  - EXG, SWAP, EXT, CLR, NEG, NEGX, NOT, TST
  - BTST/BCHG/BCLR/BSET
  - TRAP, STOP, ILLEGAL, JSR/JMP, NOP, RESET
- 300+ unit tests across 7 test modules.
- Cross-validation against the behavioral `motorola-68000-simulator`.
- `code/specs/07n2-motorola68k-gatelevel.md` specification document.
