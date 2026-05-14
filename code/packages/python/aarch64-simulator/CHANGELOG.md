# Changelog — coding-adventures-aarch64-simulator

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-13

### Added

- Initial release: full AArch64 (ARMv8-A, 2011) integer ISA behavioral simulator.
- **State model**: 31×64-bit GPRs (X0–X30) + XZR (always-zero) + SP + PC + NZCV flags
  + 64 KiB big-endian memory. Frozen `AArch64State` dataclass for snapshot independence.
- **SIM00 protocol**: `AArch64Simulator` implements `Simulator[AArch64State]` with
  `reset()`, `load()`, `step()`, `execute()`, `get_state()`.
- **Instruction support** (fixed 32-bit encoding):
  - Data Processing Immediate: ADD/SUB/ADDS/SUBS with 12-bit immediate (optional LSL #12)
  - Data Processing Register: ADD/SUB/ADDS/SUBS with shift (LSL/LSR/ASR/ROR)
  - Logical Immediate: AND/ORR/EOR/ANDS with bitmask immediates (N, immr, imms)
  - Logical Register: AND/ORR/EOR/ANDS/BIC/ORN/EON/BICS (shifted register)
  - Move Wide: MOVZ/MOVN/MOVK with 16-bit immediate and hw (0/16/32/48) shift
  - Load/Store Unsigned Offset: LDR/STR/LDRB/LDRH/LDRSB/LDRSH/LDRSW/STRB/STRH
  - Branches: B/BL (immediate), BR/BLR/RET (register), B.cond, CBZ/CBNZ, TBZ/TBNZ
  - Conditional Select: CSEL/CSINC/CSINV/CSNEG
  - Multiply/Divide: MADD/MSUB/MUL, UMULH/SMULH, UDIV/SDIV
  - Shift by register: LSLV/LSRV/ASRV/RORV
  - Bit operations: CLZ, RBIT, REV, REV16, REV32
  - System: NOP, SVC (no-op)
- **Instruction encoding helpers** for tests: `dp_imm()`, `dp_reg()`, `logic_imm()`,
  `logic_reg()`, `movwide()`, `ldst_uoff()`, `branch_imm()`, `branch_cond()`,
  `branch_reg()`, `cbz_cbnz()`, `madd_msub()`, `csel_enc()`, `tbz_tbnz()`.
- **Condition code constants**: `COND_EQ` through `COND_AL`.
- **148 tests** across protocol, instruction, coverage, and integration suites.
- **95.6% test coverage** (target: ≥80%).
- Passes `ruff check` with zero warnings.
