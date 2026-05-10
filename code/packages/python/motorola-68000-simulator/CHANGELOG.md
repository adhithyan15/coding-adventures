# Changelog — motorola-68000-simulator

All notable changes to this project are documented here.

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of the Motorola 68000 (1979) behavioral simulator
  following the SIM00 `Simulator[M68KState]` protocol (Layer 07n).
- `M68KState` — frozen dataclass with:
  - 8 × 32-bit data registers (D0–D7)
  - 8 × 32-bit address registers (A0–A7, where A7 = supervisor stack pointer)
  - 32-bit program counter (PC)
  - 16-bit status register (SR) with both system byte and CCR
  - Convenience properties: `.x`, `.n`, `.z`, `.v`, `.c` (CCR bits)
  - Convenience properties: `.d` (tuple of data registers), `.a` (address regs)
  - 16 MB linear memory (tuple of 16,777,216 unsigned bytes)
  - `halted` flag
- `flags.py` — pure-function CCR computation helpers:
  - `compute_nzvc_add`, `compute_nzvc_sub` (full flag sets for ADD/SUB families)
  - `compute_nz_logic` (N/Z for AND/OR/EOR/NOT, V/C cleared)
  - `compute_nzvc_neg` (NEG-specific carry/overflow semantics)
  - `compute_v_add`, `compute_v_sub` (overflow detection)
- `M68KSimulator` — full behavioral simulator for ~50 instructions:
  - **Data movement**: MOVE (all sizes + 14 addressing modes), MOVEQ, MOVEA,
    MOVE SR↔Dn, MOVE CCR↔Dn, CLR
  - **Arithmetic**: ADD/ADDI/ADDQ/ADDX, SUB/SUBI/SUBQ/SUBX, NEG/NEGX,
    MULU/MULS (16×16→32), DIVU/DIVS
  - **Logic**: AND/ANDI, OR/ORI, EOR/EORI, NOT, TST
  - **Shifts and rotates**: ASL/ASR, LSL/LSR, ROL/ROR, ROXL/ROXR
    (register and memory forms, immediate and register count)
  - **Compare**: CMP/CMPI/CMPA
  - **Branches**: BRA, BSR, Bcc (16 conditions), DBcc
  - **Jumps and calls**: JMP, JSR, RTS, RTR
  - **Miscellaneous register**: SWAP, EXT.W, EXT.L
  - **Stack frame**: LINK, UNLK
  - **Address**: LEA, PEA
  - **System**: NOP, STOP (halt), RESET, TRAP #n (TRAP #15 = halt)
- Comprehensive test suite: 4 test files, 280+ test cases, ≥95% line coverage.
- Big-endian byte ordering throughout (most-significant byte at lowest address).
- Linear 24-bit address space (16 MB), programs load at 0x001000, PC starts at
  0x001000, SSP starts at 0x00F00000.
- Literate-programming style: all source files include architecture diagrams,
  truth tables, and prose explanations inline with the code.
