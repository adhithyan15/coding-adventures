# Changelog

All notable changes to `coding-adventures-apple-m1-simulator` are documented here.

## [0.1.0] — 2026-05-15

### Added

**Layer 07z: Apple M1 (AArch64 + NEON/AdvSIMD) behavioral simulator**

First release. Implements a complete behavioral simulator for the Apple M1
instruction set:

**State model:**
- `AppleM1State` frozen dataclass with 32 × 64-bit GPR, 32 × 128-bit vreg,
  SP, PC, NZCV, 64 KiB memory, halted flag
- Full AArch64 integer register properties (x0–x30, w0–w5, n/z/c/v)
- FP register properties: d0–d7 (float), s0–s7 (float), d0_bits–d7_bits (int),
  v0–v7 (raw 128-bit)

**AArch64 integer base (identical to 07v):**
- ADD, SUB, ADDS, SUBS (immediate and shifted-register)
- MOVZ, MOVN, MOVK
- AND, ORR, EOR, ANDS, BIC, ORN, EON, BICS (immediate and shifted-register)
- UDIV, SDIV, LSLV, LSRV, ASRV, RORV
- CLZ, RBIT, REV, REV16, REV32
- MADD, MSUB, SMULH, UMULH
- CSEL, CSINC, CSINV, CSNEG
- LDR, STR, LDRB, STRB, LDRH, STRH, LDRSB, LDRSH, LDRSW
- B, BL, B.cond (all 14 condition codes), BR, BLR, RET
- CBZ, CBNZ, TBZ, TBNZ
- SVC (NOP), NOP

**NEW: Scalar FP (vs 07v):**
- FMOV: FP↔FP (same precision), GPR→FP, FP→GPR (double and single)
- FADD, FSUB, FMUL, FDIV (double and single precision)
- FABS, FNEG, FSQRT (double and single)
- FCMP: sets NZCV for FP branches (EQ/LT/GT/unordered)
- FCVT: single↔double precision conversion
- FCVTZS: FP → integer (truncate toward zero; NaN → 0)
- SCVTF, UCVTF: signed/unsigned integer → FP

**NEW: FP Load/Store:**
- LDR/STR for D (64-bit double) registers with unsigned offset
- LDR/STR for S (32-bit single) registers with unsigned offset

**NEW: NEON Vector:**
- ADD, SUB per-element integer (all arrangements: 8B, 16B, 4H, 8H, 2S, 4S, 1D, 2D)
- MUL per-element integer (except 64-bit elements)
- FADD, FSUB, FMUL per-element FP (4S and 2D)
- DUP from GPR (broadcast integer register to all lanes)
- FMLA: Vd = Vd + Vn × Vm (fused multiply-accumulate)

**Protocol compliance:**
- Implements `Simulator[AppleM1State]` from `simulator_protocol`
- `reset()`, `load()`, `step()`, `execute()`, `get_state()`
- HALT sentinel: `0x00000000`
- ERROR trace on unknown opcode (halts simulator)

**Test coverage:** >80% (pytest-cov)

**Linting:** ruff clean (E, F, I, UP rules)
