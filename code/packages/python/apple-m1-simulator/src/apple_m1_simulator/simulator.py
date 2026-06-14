"""
Apple M1 (2020) Behavioral Simulator
======================================

The Apple M1 was Apple's first ARM-based SoC for Mac, released November 2020.
It implements ARMv8.4-A: the same AArch64 integer ISA as the 07v layer, plus
full NEON/AdvSIMD support (scalar FP and 128-bit vector operations).

Architecture highlights
-----------------------
  - 5 nm TSMC process; 16 billion transistors
  - 4 "Firestorm" high-performance + 4 "Icestorm" efficiency cores
  - 128-bit NEON/AdvSIMD SIMD; up to 4 FP/SIMD ops per cycle
  - Unified memory architecture (CPU and GPU share DRAM)

What this simulator adds over layer 07v (pure AArch64 integer)
--------------------------------------------------------------
  1. vreg (V0–V31): 32 × 128-bit NEON register file
  2. Scalar FP: FMOV, FADD, FSUB, FMUL, FDIV, FABS, FNEG, FSQRT, FCMP,
                FCVT, FCVTZS, SCVTF, UCVTF
  3. FP load/store: LDR/STR for D and S registers (V=1 in ldst encoding)
  4. NEON vector integer: ADD, SUB per-element (2D, 4S, 8H, 16B), MUL
  5. NEON vector FP: FADD, FSUB, FMUL per-element (2D, 4S)
  6. DUP from GPR: broadcast integer register into all vector lanes
  7. FMLA: fused multiply-accumulate Vd = Vd + Vn × Vm

Instruction encoding summary (additions to AArch64 base)
---------------------------------------------------------
  Scalar FP data-processing (1 src):  000_11110 | ftype | 1 | opcode | 10000 | Rn | Rd
  Scalar FP data-processing (2 src):  000_11110 | ftype | 1 | Rm | opcode | 10 | Rn | Rd
  FCMP:                                000_11110 | ftype | 1 | Rm | 001000 | Rn | 00 | opc
  FMOV GPR↔FP:                        sf0_11110 | ftype | 1 | 00110/00111 | 000000 | Rn | Rd
  FCVTZS:                              sf0_11110 | ftype | 1 | 11000 | 000000 | Rn | Rd
  SCVTF/UCVTF:                         sf0_11110 | ftype | 1 | 00010/00011 | 000000 | Rn | Rd
  FP ld/st unsigned offset:            size | 111 | V=1 | 01 | opc | imm12 | Rn | Rt
  NEON 3-reg same:                     0 | Q | U | 01110 | size | 1 | Rm | opcode | 1 | Rn | Rd

HALT sentinel
-------------
  0x00000000 (UDF #0) — permanently undefined in AArch64.

Simplifications
---------------
  - No exception levels (EL0–EL3) or MMU
  - UDIV/SDIV by zero returns 0
  - SVC/HVC/SMC → NOP
  - Memory barriers (DMB/DSB/ISB) → NOP
  - Only NZCV tracked; DAIF and other PSTATE fields ignored
  - No FPCR/FPSR; always round-to-nearest-even
  - NaN propagation: NaN op X = NaN; FCVTZS(NaN) = 0
"""

from __future__ import annotations

import math
import struct

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .state import (
    MASK32,
    MASK64,
    MASK128,
    MEM_SIZE,
    NZCV_FP_EQ,
    NZCV_FP_GT,
    NZCV_FP_LT,
    NZCV_FP_UN,
    AppleM1State,
    f32_from_bits,
    f32_to_bits,
    f64_from_bits,
    f64_to_bits,
    make_initial_state,
    sext,
    sext19,
    sext26,
    sext32,
)

# ── HALT word ───────────────────────────────────────────────────────────────────

HALT: bytes = b"\x00\x00\x00\x00"

# ── Instruction encoding helpers ──────────────────────────────────────────────
# These helpers let tests assemble small programs without a real assembler.
# Each function returns exactly 4 bytes in big-endian order.


def _u32be(v: int) -> bytes:
    """Pack a 32-bit value as big-endian bytes."""
    return struct.pack(">I", v & 0xFFFF_FFFF)


# ── AArch64 integer base encoding helpers ─────────────────────────────────────


def dp_imm(sf: int, op: int, S: int, imm12: int, sh: int, Rn: int, Rd: int) -> bytes:
    """
    Encode Data Processing Immediate (ADD/SUB family).

    Encoding: sf | op | S | 100000 | sh | imm12 | Rn | Rd
    sf=1→64-bit, sf=0→32-bit.  sh=1 shifts imm12 left by 12.
    op=0→ADD, op=1→SUB.  S=1 sets NZCV.

    Examples::
        dp_imm(1, 0, 0, 5, 0, 0, 0)   # ADD X0, X0, #5
        dp_imm(1, 1, 1, 0, 0, 1, 31)  # CMP X1, #0  (SUBS XZR, X1, #0)
    """
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b100000 << 23)
    v |= ((sh & 1) << 22)
    v |= ((imm12 & 0xFFF) << 10)
    v |= ((Rn & 0x1F) << 5)
    v |= (Rd & 0x1F)
    return _u32be(v)


def dp_reg(sf: int, op: int, S: int, shift: int, Rm: int, imm6: int, Rn: int, Rd: int) -> bytes:
    """
    Encode Data Processing Register (shifted-register ADD/SUB/logical).

    Encoding: sf | op | S | 01011 | shift | 0 | Rm | imm6 | Rn | Rd
    op=0→ADD, op=1→SUB.  S=1 sets NZCV.

    Examples::
        dp_reg(1, 0, 0, 0, 2, 0, 1, 0)  # ADD X0, X1, X2
        dp_reg(1, 1, 1, 0, 3, 0, 4, 31) # CMP X4, X3
    """
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b01011 << 24)
    v |= ((shift & 3) << 22)
    v |= ((Rm & 0x1F) << 16)
    v |= ((imm6 & 0x3F) << 10)
    v |= ((Rn & 0x1F) << 5)
    v |= (Rd & 0x1F)
    return _u32be(v)


def logic_imm(sf: int, opc: int, N: int, immr: int, imms: int, Rn: int, Rd: int) -> bytes:
    """
    Encode Logical Immediate (AND/ORR/EOR/ANDS with bitmask immediate).

    Encoding: sf | opc | 0 | 100100 | N | immr | imms | Rn | Rd
    opc: 00=AND, 01=ORR, 10=EOR, 11=ANDS.
    """
    v = ((sf & 1) << 31) | ((opc & 3) << 29) | (0 << 28) | (0b100100 << 22)
    v |= ((N & 1) << 22)
    v |= ((immr & 0x3F) << 16)
    v |= ((imms & 0x3F) << 10)
    v |= ((Rn & 0x1F) << 5)
    v |= (Rd & 0x1F)
    return _u32be(v)


def logic_reg(
    sf: int, opc: int, shift: int, N: int, Rm: int, imm6: int, Rn: int, Rd: int
) -> bytes:
    """
    Encode Logical shifted-register (AND/ORR/EOR/ANDS, BIC/ORN/EON/BICS).

    Encoding: sf | opc | 01010 | shift | N | Rm | imm6 | Rn | Rd
    opc: 00=AND, 01=ORR, 10=EOR, 11=ANDS.  N=1 inverts Rm.
    """
    v = ((sf & 1) << 31) | ((opc & 3) << 29)
    v |= (0b01010 << 24)
    v |= ((shift & 3) << 22)
    v |= ((N & 1) << 21)
    v |= ((Rm & 0x1F) << 16)
    v |= ((imm6 & 0x3F) << 10)
    v |= ((Rn & 0x1F) << 5)
    v |= (Rd & 0x1F)
    return _u32be(v)


def movwide(sf: int, opc: int, hw: int, imm16: int, Rd: int) -> bytes:
    """
    Encode Move Wide Immediate (MOVZ / MOVN / MOVK).

    Encoding: sf | opc | 100101 | hw | imm16 | Rd
    opc: 00=MOVN, 10=MOVZ, 11=MOVK.  hw: shift = hw×16.
    """
    v = ((sf & 1) << 31) | ((opc & 3) << 29)
    v |= (0b100101 << 23)
    v |= ((hw & 3) << 21)
    v |= ((imm16 & 0xFFFF) << 5)
    v |= (Rd & 0x1F)
    return _u32be(v)


def ldst_uoff(size: int, V: int, opc: int, imm12: int, Rn: int, Rt: int) -> bytes:
    """
    Encode Load/Store Unsigned Offset.

    Encoding: size | 111 | V | 01 | opc | imm12 | Rn | Rt
    V=0 → integer; V=1 → FP/SIMD.
    EA = Rn + (imm12 << size).

    Integer (V=0):
      size=00,opc=00 → STRB;  size=00,opc=01 → LDRB
      size=10,opc=00 → STR W; size=10,opc=01 → LDR W
      size=11,opc=00 → STR X; size=11,opc=01 → LDR X

    FP (V=1):
      size=10,opc=00 → STR S; size=10,opc=01 → LDR S
      size=11,opc=00 → STR D; size=11,opc=01 → LDR D
    """
    v = ((size & 3) << 30)
    v |= (0b111 << 27)
    v |= ((V & 1) << 26)
    v |= (0b01 << 24)
    v |= ((opc & 3) << 22)
    v |= ((imm12 & 0xFFF) << 10)
    v |= ((Rn & 0x1F) << 5)
    v |= (Rt & 0x1F)
    return _u32be(v)


def branch_imm(op: int, imm26: int) -> bytes:
    """
    Encode Unconditional Branch (immediate): B / BL.

    Encoding: op[31] | 00101[30:26] | imm26[25:0]
    Target = PC + SignExtend(imm26×4, 64).
    """
    v = ((op & 1) << 31) | (0b00101 << 26) | (imm26 & 0x3FF_FFFF)
    return _u32be(v)


def branch_cond(imm19: int, cond: int) -> bytes:
    """
    Encode Conditional Branch (immediate): B.cond.

    Encoding: 01010100 | imm19 | 0 | cond
    Target = PC + SignExtend(imm19×4, 64).
    """
    v = (0b01010100 << 24) | ((imm19 & 0x7FFFF) << 5) | (cond & 0xF)
    return _u32be(v)


def cbz_cbnz(sf: int, op: int, imm19: int, Rt: int) -> bytes:
    """
    Encode Compare-and-Branch: CBZ / CBNZ.

    Encoding: sf | 011010 | op | imm19 | Rt
    op=0→CBZ (branch if Rt==0); op=1→CBNZ (branch if Rt!=0).
    """
    v = ((sf & 1) << 31) | (0b011010 << 25) | ((op & 1) << 24)
    v |= ((imm19 & 0x7FFFF) << 5) | (Rt & 0x1F)
    return _u32be(v)


def branch_reg(op: int, Rn: int) -> bytes:
    """
    Encode Unconditional Branch (register): BR / BLR / RET.

    Encoding: 1101011 0 | op | 11111 | 000000 | Rn | 00000
    op=00→BR, op=01→BLR, op=10→RET.
    """
    v = (0b1101011_0 << 24) | ((op & 0x7) << 21) | (0b11111 << 16) | ((Rn & 0x1F) << 5)
    return _u32be(v)


def madd_msub(sf: int, op54: int, Rm: int, o0: int, Ra: int, Rn: int, Rd: int) -> bytes:
    """
    Encode 3-source Data Processing: MADD / MSUB.

    Encoding: sf | 0 | 0 | 11011 | op54 | Rm | o0 | Ra | Rn | Rd
    op54=000,o0=0→MADD (Rd=Ra+Rn×Rm); op54=000,o0=1→MSUB (Rd=Ra-Rn×Rm).
    """
    v = ((sf & 1) << 31)
    v |= (0b00_11011 << 24)
    v |= ((op54 & 7) << 21)
    v |= ((Rm & 0x1F) << 16)
    v |= ((o0 & 1) << 15)
    v |= ((Ra & 0x1F) << 10)
    v |= ((Rn & 0x1F) << 5)
    v |= (Rd & 0x1F)
    return _u32be(v)


def csel_enc(sf: int, op: int, S: int, Rm: int, cond: int, op2: int, Rn: int, Rd: int) -> bytes:
    """
    Encode Conditional Select: CSEL / CSINC / CSINV / CSNEG.

    Encoding: sf | op | S | 11010100 | Rm | cond | op2 | Rn | Rd
    op/op2: 0,00=CSEL; 0,01=CSINC; 1,00=CSINV; 1,01=CSNEG.
    """
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b11010100 << 21)
    v |= ((Rm & 0x1F) << 16)
    v |= ((cond & 0xF) << 12)
    v |= ((op2 & 3) << 10)
    v |= ((Rn & 0x1F) << 5)
    v |= (Rd & 0x1F)
    return _u32be(v)


def tbz_tbnz(b5: int, op: int, b40: int, imm14: int, Rt: int) -> bytes:
    """
    Encode Test-and-Branch: TBZ / TBNZ.

    Encoding: b5 | 011011 | op | b40 | imm14 | Rt
    Tests bit (b5<<5 | b40) of Rt.  op=0→TBZ, op=1→TBNZ.
    """
    v = ((b5 & 1) << 31) | (0b011011 << 25) | ((op & 1) << 24)
    v |= ((b40 & 0x1F) << 19) | ((imm14 & 0x3FFF) << 5) | (Rt & 0x1F)
    return _u32be(v)


# ── FP / NEON encoding helpers ─────────────────────────────────────────────────


def fp_dp1src(ftype: int, opcode: int, Rn: int, Rd: int) -> bytes:
    """
    Encode Scalar FP Data Processing 1-source (FMOV reg, FABS, FNEG, FSQRT, FCVT).

    Encoding: 000_11110 | ftype | 1 | opcode[5:0] | 10000 | Rn | Rd
    ftype: 00=single, 01=double
    opcode: 000000=FMOV, 000001=FABS, 000010=FNEG, 000011=FSQRT, 000100=FCVT
    """
    v = (0b000_11110 << 24) | ((ftype & 3) << 22) | (1 << 21)
    v |= ((opcode & 0x3F) << 15) | (0b10000 << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def fp_dp2src(ftype: int, Rm: int, opcode: int, Rn: int, Rd: int) -> bytes:
    """
    Encode Scalar FP Data Processing 2-source (FMUL, FDIV, FADD, FSUB).

    Encoding: 000_11110 | ftype | 1 | Rm | opcode | 10 | Rn | Rd
    opcode (bits[15:12]): 0000=FMUL, 0001=FDIV, 0010=FADD, 0011=FSUB
    """
    v = (0b000_11110 << 24) | ((ftype & 3) << 22) | (1 << 21)
    v |= ((Rm & 0x1F) << 16) | ((opcode & 0xF) << 12) | (0b10 << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def fp_cmp(ftype: int, Rm: int, Rn: int, opc: int = 0) -> bytes:
    """
    Encode FCMP (FP compare, updates NZCV).

    Encoding: 000_11110 | ftype | 1 | Rm | 001000 | Rn | 00 | opc
    opc=000 → FCMP Rn,Rm; opc=011 → FCMP Rn,#0.0
    """
    v = (0b000_11110 << 24) | ((ftype & 3) << 22) | (1 << 21)
    v |= ((Rm & 0x1F) << 16) | (0b001000 << 10)
    v |= ((Rn & 0x1F) << 5) | (opc & 0x7)
    return _u32be(v)


def fmov_gpr_to_fp_d(Xn: int, Dd: int) -> bytes:
    """
    Encode FMOV Dd, Xn (64-bit GPR → double FP register).

    sf=1, ftype=01 (double), opcode=00111 (GPR→FP).
    """
    v = (0b100_11110 << 24) | (0b01 << 22) | (1 << 21)
    v |= (0b00111 << 16) | (0b000000 << 10)
    v |= ((Xn & 0x1F) << 5) | (Dd & 0x1F)
    return _u32be(v)


def fmov_fp_to_gpr_d(Dn: int, Xd: int) -> bytes:
    """
    Encode FMOV Xd, Dn (double FP register → 64-bit GPR).

    sf=1, ftype=01 (double), opcode=00110 (FP→GPR).
    """
    v = (0b100_11110 << 24) | (0b01 << 22) | (1 << 21)
    v |= (0b00110 << 16) | (0b000000 << 10)
    v |= ((Dn & 0x1F) << 5) | (Xd & 0x1F)
    return _u32be(v)


def fmov_gpr_to_fp_s(Wn: int, Sd: int) -> bytes:
    """
    Encode FMOV Sd, Wn (32-bit GPR → single FP register).

    sf=0, ftype=00 (single), opcode=00111 (GPR→FP).
    """
    v = (0b000_11110 << 24) | (0b00 << 22) | (1 << 21)
    v |= (0b00111 << 16) | (0b000000 << 10)
    v |= ((Wn & 0x1F) << 5) | (Sd & 0x1F)
    return _u32be(v)


def fmov_fp_to_gpr_s(Sn: int, Wd: int) -> bytes:
    """
    Encode FMOV Wd, Sn (single FP register → 32-bit GPR).

    sf=0, ftype=00 (single), opcode=00110 (FP→GPR).
    """
    v = (0b000_11110 << 24) | (0b00 << 22) | (1 << 21)
    v |= (0b00110 << 16) | (0b000000 << 10)
    v |= ((Sn & 0x1F) << 5) | (Wd & 0x1F)
    return _u32be(v)


def fcvtzs(sf: int, ftype: int, Rn: int, Rd: int) -> bytes:
    """
    Encode FCVTZS (FP → integer, truncate toward zero).

    Encoding: sf | 00_11110 | ftype | 1 | 11000 | 000000 | Rn | Rd
    sf=1 → 64-bit output (Xd); sf=0 → 32-bit (Wd)
    ftype=01 → double input; ftype=00 → single input
    """
    v = ((sf & 1) << 31) | (0b00_11110 << 24) | ((ftype & 3) << 22) | (1 << 21)
    v |= (0b11000 << 16) | (0b000000 << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def scvtf(sf: int, ftype: int, Rn: int, Rd: int) -> bytes:
    """
    Encode SCVTF (signed integer → FP).

    Encoding: sf | 00_11110 | ftype | 1 | 00010 | 000000 | Rn | Rd
    sf=1 → 64-bit input (Xn); sf=0 → 32-bit (Wn)
    ftype=01 → double output; ftype=00 → single output
    """
    v = ((sf & 1) << 31) | (0b00_11110 << 24) | ((ftype & 3) << 22) | (1 << 21)
    v |= (0b00010 << 16) | (0b000000 << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def ucvtf(sf: int, ftype: int, Rn: int, Rd: int) -> bytes:
    """
    Encode UCVTF (unsigned integer → FP).

    Encoding: sf | 00_11110 | ftype | 1 | 00011 | 000000 | Rn | Rd
    sf=1 → 64-bit input (Xn); sf=0 → 32-bit (Wn)
    ftype=01 → double output; ftype=00 → single output
    """
    v = ((sf & 1) << 31) | (0b00_11110 << 24) | ((ftype & 3) << 22) | (1 << 21)
    v |= (0b00011 << 16) | (0b000000 << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def fp_ldst_uoff(size: int, opc: int, imm12: int, Rn: int, Rt: int) -> bytes:
    """
    Encode FP Load/Store Unsigned Offset (V=1).

    Encoding: size[31:30] | 111 | V=1 | 01 | opc | imm12 | Rn | Rt
    size=10,opc=01 → LDR St,[Xn,#imm*4]
    size=10,opc=00 → STR St,[Xn,#imm*4]
    size=11,opc=01 → LDR Dt,[Xn,#imm*8]
    size=11,opc=00 → STR Dt,[Xn,#imm*8]
    """
    v = ((size & 3) << 30) | (0b111 << 27) | (1 << 26) | (0b01 << 24)
    v |= ((opc & 3) << 22) | ((imm12 & 0xFFF) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rt & 0x1F)
    return _u32be(v)


def neon_3reg_same(Q: int, U: int, size: int, Rm: int, opcode: int, Rn: int, Rd: int) -> bytes:
    """
    Encode AdvSIMD Three-Register Same.

    Encoding: 0 | Q | U | 01110 | size | 1 | Rm | opcode | 1 | Rn | Rd
    Q=0 → 64-bit lane mode; Q=1 → 128-bit.
    U=0/1 selects ADD/SUB for opcode=10000; MUL is opcode=10011.

    opcode field is bits[15:11]:
      10000 (0b10000=16) = ADD (U=0) or SUB (U=1)
      10011 (0b10011=19) = MUL (not for 64-bit elements)
      11010 (0b11010=26) = FADD (U=0) or FSUB (U=1)
      11011 (0b11011=27) = FMUL (U=1, bit29=1)
      11001 (0b11001=25) = FMLA (fused multiply-accumulate)
    """
    v = (Q << 30) | (U << 29) | (0b01110 << 24) | ((size & 3) << 22)
    v |= (1 << 21) | ((Rm & 0x1F) << 16) | ((opcode & 0x1F) << 11) | (1 << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def neon_dup_gpr(Q: int, imm5: int, Rn: int, Rd: int) -> bytes:
    """
    Encode DUP (from GPR to vector, all lanes).

    Encoding: 0 | Q | 0 | 01110 | imm5 | 00001 | 1 | 00 | 1 | Rn | Rd
    imm5=10000 → 64-bit D lanes (use with Q=1 for 2D)
    imm5=01000 → 32-bit S lanes (use with Q=1 for 4S)
    """
    v = (Q << 30) | (0b01110 << 24) | ((imm5 & 0x1F) << 19)
    v |= (0b00001 << 14) | (1 << 13) | (1 << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


# ── Condition codes ────────────────────────────────────────────────────────────

COND_EQ: int = 0b0000   # Equal (Z=1)
COND_NE: int = 0b0001   # Not equal (Z=0)
COND_CS: int = 0b0010   # Carry set (C=1)
COND_CC: int = 0b0011   # Carry clear (C=0)
COND_MI: int = 0b0100   # Minus (N=1)
COND_PL: int = 0b0101   # Plus (N=0)
COND_VS: int = 0b0110   # Overflow (V=1)
COND_VC: int = 0b0111   # No overflow (V=0)
COND_HI: int = 0b1000   # Unsigned higher (C=1 and Z=0)
COND_LS: int = 0b1001   # Unsigned lower or same
COND_GE: int = 0b1010   # Signed ≥ (N==V)
COND_LT: int = 0b1011   # Signed < (N!=V)
COND_GT: int = 0b1100   # Signed > (Z=0 and N==V)
COND_LE: int = 0b1101   # Signed ≤ (Z=1 or N!=V)
COND_AL: int = 0b1110   # Always

# ── Internal helpers ──────────────────────────────────────────────────────────


def _condition_holds(cond: int, nzcv: int) -> bool:
    """
    Evaluate whether a 4-bit condition code is satisfied given NZCV flags.

    Each condition code tests a combination of N, Z, C, V:
      EQ: Z=1    NE: Z=0    CS: C=1    CC: C=0
      MI: N=1    PL: N=0    VS: V=1    VC: V=0
      HI: C=1&Z=0  LS: C=0|Z=1  GE: N=V  LT: N≠V
      GT: Z=0&N=V  LE: Z=1|N≠V  AL: true
    """
    N = (nzcv >> 3) & 1
    Z = (nzcv >> 2) & 1
    C = (nzcv >> 1) & 1
    V = nzcv & 1
    base = cond >> 1
    if base == 0:
        result = Z == 1
    elif base == 1:
        result = C == 1
    elif base == 2:
        result = N == 1
    elif base == 3:
        result = V == 1
    elif base == 4:
        result = C == 1 and Z == 0
    elif base == 5:
        result = N == V
    elif base == 6:
        result = N == V and Z == 0
    else:  # base == 7 (AL / NV)
        result = True
    if (cond & 1) and cond != 0xF:
        result = not result
    return result


def _add_with_flags(a: int, b: int, sf: int) -> tuple[int, int]:
    """
    Perform A + B and compute NZCV flags.

    Returns (result_masked, nzcv) where result is masked to sf-width.
    sf=1 → 64-bit; sf=0 → 32-bit.
    """
    bits = 64 if sf else 32
    mask = MASK64 if sf else MASK32
    unsigned_sum = (a & mask) + (b & mask)
    result = unsigned_sum & mask
    N = (result >> (bits - 1)) & 1
    Z = 1 if result == 0 else 0
    C = 1 if unsigned_sum > mask else 0
    a_sign = (a >> (bits - 1)) & 1
    b_sign = (b >> (bits - 1)) & 1
    r_sign = N
    V = 1 if (a_sign == b_sign) and (r_sign != a_sign) else 0
    return result, (N << 3) | (Z << 2) | (C << 1) | V


def _sub_with_flags(a: int, b: int, sf: int) -> tuple[int, int]:
    """
    Compute A − B with NZCV flags using the borrow-complement carry convention.

    In AArch64 subtraction is: result = A + NOT(B) + 1
    C=1 means *no* borrow (unsigned A ≥ B).
    """
    bits = 64 if sf else 32
    mask = MASK64 if sf else MASK32
    not_b = (~b) & mask
    unsigned_sum = (a & mask) + not_b + 1
    result = unsigned_sum & mask
    N = (result >> (bits - 1)) & 1
    Z = 1 if result == 0 else 0
    C = 1 if unsigned_sum > mask else 0
    a_sign = (a >> (bits - 1)) & 1
    b_sign = (not_b >> (bits - 1)) & 1
    r_sign = N
    V = 1 if (a_sign == b_sign) and (r_sign != a_sign) else 0
    return result, (N << 3) | (Z << 2) | (C << 1) | V


def _logical_flags(result: int, sf: int) -> int:
    """Compute NZCV flags after a logical operation (C=V=0)."""
    bits = 64 if sf else 32
    N = (result >> (bits - 1)) & 1
    Z = 1 if result == 0 else 0
    return (N << 3) | (Z << 2)


def _apply_shift(value: int, shift_type: int, amount: int, sf: int) -> int:
    """
    Apply a shift operation to `value`.

    shift_type: 0=LSL, 1=LSR (logical), 2=ASR (arithmetic), 3=ROR.
    """
    bits = 64 if sf else 32
    mask = MASK64 if sf else MASK32
    value &= mask
    amount &= (bits - 1)
    if amount == 0:
        return value
    if shift_type == 0:    # LSL
        return (value << amount) & mask
    elif shift_type == 1:  # LSR
        return value >> amount
    elif shift_type == 2:  # ASR
        sign = (value >> (bits - 1)) & 1
        result = value >> amount
        if sign:
            fill = ((1 << amount) - 1) << (bits - amount)
            result |= fill & mask
        return result
    else:  # ROR
        return ((value >> amount) | (value << (bits - amount))) & mask


def _ror(value: int, amount: int, width: int) -> int:
    """Rotate `value` right by `amount` bits within a field of `width` bits."""
    amount %= width
    mask = (1 << width) - 1
    return ((value >> amount) | (value << (width - amount))) & mask


def _decode_bitmask(N: int, immr: int, imms: int) -> int:
    """
    Decode AArch64 logical-immediate (N, immr, imms) to a 64-bit bitmask.

    Algorithm:
      1. len = highest bit of (N<<6 | ~imms&0x3F) that is set
      2. esize = 2^len
      3. S = imms & (esize-1)  — number of set bits minus 1
      4. R = immr & (esize-1)  — right-rotation amount
      5. welem = (1 << (S+1)) - 1
      6. telem = ror(welem, R, esize)
      7. result = telem replicated to fill 64 bits

    Raises ValueError for the UNDEFINED encoding.
    """
    if N == 1:
        len_ = 6
    else:
        combined = (~imms & 0x3F) | (N << 6)
        len_ = combined.bit_length() - 1
        if len_ <= 0:
            raise ValueError(f"UNDEFINED bitmask: N={N}, immr={immr}, imms={imms}")
    esize = 1 << len_
    S = imms & (esize - 1)
    R = immr & (esize - 1)
    welem = (1 << (S + 1)) - 1
    telem = _ror(welem, R, esize)
    result = 0
    for pos in range(0, 64, esize):
        result |= telem << pos
    return result & MASK64


def _clz(value: int, width: int) -> int:
    """Count Leading Zeros in `value` viewed as a `width`-bit unsigned integer."""
    if value == 0:
        return width
    count = 0
    for i in range(width - 1, -1, -1):
        if (value >> i) & 1:
            break
        count += 1
    return count


def _rbit(value: int, width: int) -> int:
    """Reverse all `width` bits of `value`."""
    result = 0
    for i in range(width):
        result = (result << 1) | ((value >> i) & 1)
    return result


def _rev(value: int, width: int) -> int:
    """Reverse the byte order of `value` viewed as a `width`-bit integer."""
    nbytes = width // 8
    result = 0
    for _ in range(nbytes):
        result = (result << 8) | (value & 0xFF)
        value >>= 8
    return result


def _rev16(value: int, width: int) -> int:
    """Byte-reverse within each 16-bit halfword of `value`."""
    result = 0
    for offset in range(0, width, 16):
        hw = (value >> offset) & 0xFFFF
        swapped = ((hw & 0xFF) << 8) | ((hw >> 8) & 0xFF)
        result |= swapped << offset
    return result


def _rev32(value: int) -> int:
    """Byte-reverse within each 32-bit word of a 64-bit register (REV32 X)."""
    lo = _rev(value & MASK32, 32)
    hi = _rev((value >> 32) & MASK32, 32)
    return (hi << 32) | lo


def _cond_name(cond: int) -> str:
    """Return the mnemonic suffix for a 4-bit condition code."""
    names = ["EQ", "NE", "CS", "CC", "MI", "PL", "VS", "VC",
             "HI", "LS", "GE", "LT", "GT", "LE", "AL", "NV"]
    return names[cond & 0xF]


def _fp_compare(a: float, b: float) -> int:
    """
    Compare two floats and return the NZCV flags as a 4-bit nibble.

    Matches AArch64 spec FCMP behaviour:
      Equal:     NZCV = 0b0110 (Z=1, C=1)
      Less than: NZCV = 0b1000 (N=1)
      Greater:   NZCV = 0b0010 (C=1)
      Unordered: NZCV = 0b0011 (C=1, V=1) — raised when either operand is NaN

    Using math.isnan() for NaN detection is preferred over `a != a` as it is
    clearer and avoids any potential Python float quirks with signaling NaNs.
    """
    if math.isnan(a) or math.isnan(b):
        return NZCV_FP_UN
    if a == b:
        return NZCV_FP_EQ
    if a < b:
        return NZCV_FP_LT
    return NZCV_FP_GT  # a > b


# ── Simulator constant ─────────────────────────────────────────────────────────

_MAX_STEPS_LIMIT: int = 10_000_000


# ── Simulator ──────────────────────────────────────────────────────────────────


class AppleM1Simulator(Simulator[AppleM1State]):
    """
    Behavioral simulator for the Apple M1 (ARMv8.4-A: AArch64 + NEON/AdvSIMD, 2020).

    Extends the AArch64 integer base (layer 07v) with:
      - 32 × 128-bit NEON registers (V0–V31)
      - Scalar FP: FMOV, FADD, FSUB, FMUL, FDIV, FABS, FNEG, FSQRT
      - FP compare: FCMP (sets NZCV for FP branches)
      - FP conversion: FCVT (precision), FCVTZS (FP→int), SCVTF/UCVTF (int→FP)
      - FP load/store: LDR/STR for D and S registers
      - NEON vector: ADD, SUB, MUL per-element (integer)
      - NEON vector FP: FADD, FSUB, FMUL per-element
      - DUP from GPR (broadcast integer to all vector lanes)
      - FMLA (fused multiply-accumulate)

    Implements the SIM00 protocol: reset / load / step / execute / get_state.

    State model:
      - AppleM1State is a frozen dataclass (immutable snapshot)
      - New state is built on every instruction commit
      - XZR (gpr[31]) is always 0; writes are silently discarded
      - vreg[n] is a Python int in 0..2^128-1; D view = lower 64 bits
    """

    def __init__(self) -> None:
        self._state: AppleM1State = make_initial_state()

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Zero all registers (including NEON), memory, PC, NZCV, SP."""
        self._state = make_initial_state()

    def load(self, program: bytes) -> None:
        """
        Reset and copy `program` bytes into memory starting at 0x0000.

        The program should end with HALT (0x00000000). Extra bytes beyond
        MEM_SIZE are silently ignored.
        """
        self.reset()
        s = self._state
        mem = list(s.memory)
        for i, byte in enumerate(program[:MEM_SIZE]):
            mem[i] = byte
        self._state = AppleM1State(
            pc=s.pc,
            gpr=s.gpr,
            sp=s.sp,
            nzcv=s.nzcv,
            vreg=s.vreg,
            memory=tuple(mem),
            halted=s.halted,
        )

    def step(self) -> StepTrace:
        """
        Fetch, decode, execute one instruction and return a StepTrace.

        PC is pre-incremented by 4 before execution. Branch instructions
        overwrite the resulting PC with their computed target.
        """
        s = self._state
        pc = s.pc

        if s.halted:
            return StepTrace(pc_before=pc, pc_after=pc, mnemonic="HALT",
                             description=f"HALT @ 0x{pc:04X}")

        # Fetch 4 bytes big-endian
        raw = (
            (s.memory[pc % MEM_SIZE] << 24)
            | (s.memory[(pc + 1) % MEM_SIZE] << 16)
            | (s.memory[(pc + 2) % MEM_SIZE] << 8)
            | s.memory[(pc + 3) % MEM_SIZE]
        )

        if raw == 0:
            self._state = AppleM1State(
                pc=pc, gpr=s.gpr, sp=s.sp, nzcv=s.nzcv,
                vreg=s.vreg, memory=s.memory, halted=True,
            )
            return StepTrace(pc_before=pc, pc_after=pc, mnemonic="HALT",
                             description=f"HALT @ 0x{pc:04X}")

        next_pc = (pc + 4) & MASK64
        return self._decode_execute(raw, pc, next_pc, s)

    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult:
        """
        Load `program` and step until HALT or `max_steps` steps.

        Returns an ExecutionResult with final_state, steps, halted, error, traces.
        Raises ValueError if max_steps is outside [1, 10_000_000].
        """
        if not (1 <= max_steps <= _MAX_STEPS_LIMIT):
            raise ValueError(
                f"max_steps must be between 1 and {_MAX_STEPS_LIMIT}; got {max_steps}"
            )
        self.load(program)
        traces: list[StepTrace] = []
        for _ in range(max_steps):
            trace = self.step()
            traces.append(trace)
            if trace.mnemonic.startswith("ERROR:"):
                return ExecutionResult(
                    halted=False, steps=len(traces), final_state=self._state,
                    traces=traces, error=trace.mnemonic,
                )
            if self._state.halted:
                return ExecutionResult(
                    halted=True, steps=len(traces), final_state=self._state,
                    traces=traces, error=None,
                )
        return ExecutionResult(
            halted=False, steps=max_steps, final_state=self._state,
            traces=traces, error=f"max_steps={max_steps} exceeded",
        )

    def get_state(self) -> AppleM1State:
        """Return a frozen snapshot of the current simulator state."""
        return self._state

    # ── Register helpers ──────────────────────────────────────────────────────

    def _read_reg(self, idx: int, sf: int) -> int:
        """Read GPR idx (XZR=0). sf=0 masks to 32 bits."""
        if idx == 31:
            return 0
        val = self._state.gpr[idx]
        return val if sf else (val & MASK32)

    def _write_reg(self, idx: int, value: int, sf: int, new_gpr: list[int]) -> None:
        """Write GPR idx (XZR silently discarded). W-writes zero-extend."""
        if idx == 31:
            return
        new_gpr[idx] = (value & MASK64) if sf else (value & MASK32)

    def _read_vreg_d(self, idx: int) -> int:
        """Read lower 64 bits (D view) of vreg[idx]."""
        return self._state.vreg[idx] & MASK64

    def _write_vreg_d(self, idx: int, bits64: int, new_vreg: list[int]) -> None:
        """Write D register: zero-extend to 128 bits (upper 64 bits = 0)."""
        new_vreg[idx] = bits64 & MASK64

    def _read_vreg_s(self, idx: int) -> int:
        """Read lower 32 bits (S view) of vreg[idx]."""
        return self._state.vreg[idx] & MASK32

    def _write_vreg_s(self, idx: int, bits32: int, new_vreg: list[int]) -> None:
        """Write S register: zero-extend to 128 bits (upper 96 bits = 0)."""
        new_vreg[idx] = bits32 & MASK32

    # ── Memory helpers ────────────────────────────────────────────────────────

    def _mem_read(self, addr: int, nbytes: int, memory: tuple[int, ...]) -> int:
        """Read `nbytes` big-endian from memory at addr (wraps modulo MEM_SIZE)."""
        result = 0
        for i in range(nbytes):
            result = (result << 8) | memory[(addr + i) % MEM_SIZE]
        return result

    def _mem_write(self, addr: int, value: int, nbytes: int, new_mem: list[int]) -> None:
        """Write `nbytes` of value big-endian to new_mem at addr (wraps)."""
        for i in range(nbytes - 1, -1, -1):
            new_mem[(addr + i) % MEM_SIZE] = value & 0xFF
            value >>= 8

    # ── Commit helper ─────────────────────────────────────────────────────────

    def _commit(
        self,
        new_pc: int,
        new_gpr: list[int],
        new_sp: int,
        new_nzcv: int,
        new_vreg: list[int],
        new_mem: list[int],
        halted: bool = False,
    ) -> None:
        """Atomically commit a new state snapshot. Enforces XZR=0."""
        new_gpr[31] = 0
        self._state = AppleM1State(
            pc=new_pc & MASK64,
            gpr=tuple(new_gpr),
            sp=new_sp & MASK64,
            nzcv=new_nzcv & 0xF,
            vreg=tuple(new_vreg),
            memory=tuple(new_mem),
            halted=halted,
        )

    def _unknown(self, pc: int, raw: int, s: AppleM1State) -> StepTrace:
        """Handle an unrecognized opcode: halt and return an ERROR trace."""
        self._state = AppleM1State(
            pc=pc, gpr=s.gpr, sp=s.sp, nzcv=s.nzcv,
            vreg=s.vreg, memory=s.memory, halted=True,
        )
        return StepTrace(
            pc_before=pc, pc_after=pc,
            mnemonic=f"ERROR: UNKNOWN(0x{raw:08X})",
            description=f"Unknown opcode 0x{raw:08X} @ 0x{pc:04X}",
        )

    # ── Decode and execute ────────────────────────────────────────────────────

    def _decode_execute(
        self, raw: int, pc: int, next_pc: int, s: AppleM1State
    ) -> StepTrace:
        """
        Decode a 32-bit instruction and execute it.

        The decode is a hierarchy of bit-pattern checks. FP/NEON instructions
        are dispatched first (they share bits[28:24] = 11110 or 01110 patterns),
        then the standard AArch64 integer instructions follow.
        """
        gpr = list(s.gpr)
        new_vreg = list(s.vreg)
        new_pc = next_pc
        new_sp = s.sp
        new_nzcv = s.nzcv
        new_mem = list(s.memory)

        def bits(hi: int, lo: int) -> int:
            width = hi - lo + 1
            return (raw >> lo) & ((1 << width) - 1)

        sf = bits(31, 31)

        if raw == 0:
            self._commit(pc, gpr, new_sp, new_nzcv, new_vreg, new_mem, halted=True)
            return StepTrace(pc_before=pc, pc_after=pc, mnemonic="HALT",
                             description=f"HALT @ 0x{pc:04X}")

        if raw == 0xD503201F:
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic="NOP",
                             description=f"NOP @ 0x{pc:04X}")

        # ── FP/SIMD scalar: bits[28:24] == 11110 ──────────────────────────────
        # These instructions all have bit pattern ????_1111_0xxx_xxxx in the
        # top byte area. The distinguisher from integer instructions is
        # bits[28:24] = 0b11110 = 30.
        if bits(28, 24) == 0b11110:
            return self._decode_fp(raw, pc, next_pc, bits, gpr, new_vreg,
                                   new_sp, new_nzcv, new_mem, s)

        # ── NEON 3-reg same: bits[28:24] == 01110 ────────────────────────────
        # AdvSIMD three-register-same: 0|Q|U|01110|size|1|Rm|opcode|1|Rn|Rd
        if bits(28, 24) == 0b01110:
            return self._decode_neon_3reg(raw, pc, next_pc, bits, gpr, new_vreg,
                                          new_sp, new_nzcv, new_mem, s)

        # ── Unconditional branch (immediate): B / BL ─────────────────────────
        if bits(30, 26) == 0b00101:
            op = bits(31, 31)
            imm26 = sext26(bits(25, 0))
            target = (pc + imm26 * 4) & MASK64
            if op:
                gpr[30] = next_pc & MASK64
                mnem = "BL"
            else:
                mnem = "B"
            new_pc = target
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Conditional branch: B.cond ────────────────────────────────────────
        if bits(31, 24) == 0b01010100 and bits(4, 4) == 0:
            imm19 = sext19(bits(23, 5))
            cond = bits(3, 0)
            mnem = f"B.{_cond_name(cond)}"
            if _condition_holds(cond, new_nzcv):
                new_pc = (pc + imm19 * 4) & MASK64
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Compare-and-Branch: CBZ / CBNZ ───────────────────────────────────
        if bits(30, 25) == 0b011010:
            op = bits(24, 24)
            imm19 = sext19(bits(23, 5))
            Rt = bits(4, 0)
            rt_val = self._read_reg(Rt, sf)
            taken = (rt_val == 0) if op == 0 else (rt_val != 0)
            mnem = "CBZ" if op == 0 else "CBNZ"
            if taken:
                new_pc = (pc + imm19 * 4) & MASK64
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Test-and-Branch: TBZ / TBNZ ──────────────────────────────────────
        if bits(30, 25) == 0b011011:
            b5 = bits(31, 31)
            op = bits(24, 24)
            b40 = bits(23, 19)
            bit_num = (b5 << 5) | b40
            imm14 = sext(bits(18, 5), 14)
            Rt = bits(4, 0)
            rt_val = self._read_reg(Rt, 1)
            bit_val = (rt_val >> bit_num) & 1
            taken = (bit_val == 0) if op == 0 else (bit_val != 0)
            mnem = "TBZ" if op == 0 else "TBNZ"
            if taken:
                new_pc = (pc + imm14 * 4) & MASK64
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Branch register: BR / BLR / RET ──────────────────────────────────
        if bits(31, 24) == 0b1101_0110:
            op = bits(23, 21)
            Rn = bits(9, 5)
            rn_val = self._read_reg(Rn, 1)
            if op == 0b000:
                new_pc = rn_val & MASK64
                mnem = "BR"
            elif op == 0b001:
                gpr[30] = next_pc & MASK64
                new_pc = rn_val & MASK64
                mnem = "BLR"
            elif op == 0b010:
                new_pc = rn_val & MASK64
                mnem = "RET"
            else:
                return self._unknown(pc, raw, s)
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Load/Store Unsigned Offset ────────────────────────────────────────
        # Encoding: size[31:30] | 111[29:27] | V[26] | 01[25:24] | opc | imm12 | Rn | Rt
        if bits(29, 27) == 0b111 and bits(25, 24) == 0b01:
            V = bits(26, 26)
            size = bits(31, 30)
            opc = bits(23, 22)
            imm12 = bits(21, 10)
            Rn = bits(9, 5)
            Rt = bits(4, 0)
            rn_val = new_sp if Rn == 31 else self._read_reg(Rn, 1)
            ea = (rn_val + imm12 * (1 << size)) & MASK64

            if V == 1:
                # FP/SIMD load/store
                if size == 0b10 and opc == 0b00:     # STR St,[Xn,#imm*4]
                    self._mem_write(ea, self._read_vreg_s(Rt), 4, new_mem)
                    mnem = "STR_S"
                elif size == 0b10 and opc == 0b01:   # LDR St,[Xn,#imm*4]
                    val = self._mem_read(ea, 4, tuple(new_mem))
                    self._write_vreg_s(Rt, val, new_vreg)
                    mnem = "LDR_S"
                elif size == 0b11 and opc == 0b00:   # STR Dt,[Xn,#imm*8]
                    self._mem_write(ea, self._read_vreg_d(Rt), 8, new_mem)
                    mnem = "STR_D"
                elif size == 0b11 and opc == 0b01:   # LDR Dt,[Xn,#imm*8]
                    val = self._mem_read(ea, 8, tuple(new_mem))
                    self._write_vreg_d(Rt, val, new_vreg)
                    mnem = "LDR_D"
                else:
                    return self._unknown(pc, raw, s)
            elif size == 0b00:    # byte (8-bit) integer
                if opc == 0b00:
                    self._mem_write(ea, self._read_reg(Rt, 0) & 0xFF, 1, new_mem)
                    mnem = "STRB"
                elif opc == 0b01:
                    val = self._mem_read(ea, 1, tuple(new_mem))
                    self._write_reg(Rt, val, 0, gpr)
                    mnem = "LDRB"
                elif opc == 0b10:
                    val = sext(self._mem_read(ea, 1, tuple(new_mem)), 8)
                    self._write_reg(Rt, val & MASK64, 1, gpr)
                    mnem = "LDRSB"
                else:
                    val = sext(self._mem_read(ea, 1, tuple(new_mem)), 8)
                    self._write_reg(Rt, val & MASK32, 0, gpr)
                    mnem = "LDRSB32"
            elif size == 0b01:   # halfword (16-bit)
                if opc == 0b00:
                    self._mem_write(ea, self._read_reg(Rt, 0) & 0xFFFF, 2, new_mem)
                    mnem = "STRH"
                elif opc == 0b01:
                    val = self._mem_read(ea, 2, tuple(new_mem))
                    self._write_reg(Rt, val, 0, gpr)
                    mnem = "LDRH"
                elif opc == 0b10:
                    val = sext(self._mem_read(ea, 2, tuple(new_mem)), 16)
                    self._write_reg(Rt, val & MASK64, 1, gpr)
                    mnem = "LDRSH"
                else:
                    val = sext(self._mem_read(ea, 2, tuple(new_mem)), 16)
                    self._write_reg(Rt, val & MASK32, 0, gpr)
                    mnem = "LDRSH32"
            elif size == 0b10:   # word (32-bit)
                if opc == 0b00:
                    self._mem_write(ea, self._read_reg(Rt, 0), 4, new_mem)
                    mnem = "STR32"
                elif opc == 0b01:
                    val = self._mem_read(ea, 4, tuple(new_mem))
                    self._write_reg(Rt, val, 0, gpr)
                    mnem = "LDR32"
                elif opc == 0b10:
                    val = sext32(self._mem_read(ea, 4, tuple(new_mem)))
                    self._write_reg(Rt, val & MASK64, 1, gpr)
                    mnem = "LDRSW"
                else:
                    return self._unknown(pc, raw, s)
            elif size == 0b11:   # doubleword (64-bit)
                if opc == 0b00:
                    self._mem_write(ea, self._read_reg(Rt, 1), 8, new_mem)
                    mnem = "STR"
                elif opc == 0b01:
                    val = self._mem_read(ea, 8, tuple(new_mem))
                    self._write_reg(Rt, val, 1, gpr)
                    mnem = "LDR"
                else:
                    return self._unknown(pc, raw, s)
            else:
                return self._unknown(pc, raw, s)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Data Processing Immediate: ADD/SUB ────────────────────────────────
        if bits(28, 23) in (0b100000, 0b100001):
            op = bits(30, 30)
            S = bits(29, 29)
            sh = bits(22, 22)
            imm12 = bits(21, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            imm = imm12 << 12 if sh else imm12
            rn_val = self._read_reg(Rn, sf)
            if op == 0:
                if S:
                    result, new_nzcv = _add_with_flags(rn_val, imm, sf)
                    mnem = "ADDS"
                else:
                    mask = MASK64 if sf else MASK32
                    result = (rn_val + imm) & mask
                    mnem = "ADD"
            else:
                if S:
                    result, new_nzcv = _sub_with_flags(rn_val, imm, sf)
                    mnem = "SUBS"
                else:
                    mask = MASK64 if sf else MASK32
                    result = (rn_val - imm) & mask
                    mnem = "SUB"
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Move Wide Immediate: MOVZ / MOVN / MOVK ───────────────────────────
        if bits(28, 23) == 0b100101:
            opc = bits(30, 29)
            hw = bits(22, 21)
            imm16 = bits(20, 5)
            Rd = bits(4, 0)
            shift = hw * 16
            if opc == 0b10:
                result = imm16 << shift
                mnem = "MOVZ"
            elif opc == 0b00:
                result = ~(imm16 << shift)
                mnem = "MOVN"
            elif opc == 0b11:
                cur = self._read_reg(Rd, sf)
                mask_bits = 0xFFFF << shift
                result = (cur & ~mask_bits) | ((imm16 << shift) & mask_bits)
                mnem = "MOVK"
            else:
                return self._unknown(pc, raw, s)
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Logical Immediate ─────────────────────────────────────────────────
        if bits(28, 23) == 0b010010:
            opc = bits(30, 29)
            N = bits(22, 22)
            immr = bits(21, 16)
            imms = bits(15, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            try:
                imm = _decode_bitmask(N, immr, imms)
            except ValueError:
                return self._unknown(pc, raw, s)
            if not sf:
                imm &= MASK32
            rn_val = self._read_reg(Rn, sf)
            if opc == 0b00:
                result = rn_val & imm
                mnem = "AND"
            elif opc == 0b01:
                result = rn_val | imm
                mnem = "ORR"
            elif opc == 0b10:
                result = rn_val ^ imm
                mnem = "EOR"
            else:
                result = rn_val & imm
                new_nzcv = _logical_flags(result, sf)
                mnem = "ANDS"
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Logical shifted-register ──────────────────────────────────────────
        if bits(28, 24) == 0b01010:
            opc = bits(30, 29)
            shift_type = bits(23, 22)
            N = bits(21, 21)
            Rm = bits(20, 16)
            imm6 = bits(15, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            rm_val = self._read_reg(Rm, sf)
            rn_val = self._read_reg(Rn, sf)
            shifted_rm = _apply_shift(rm_val, shift_type, imm6, sf)
            if N:
                mask = MASK64 if sf else MASK32
                shifted_rm = (~shifted_rm) & mask
            if opc == 0b00:
                result = rn_val & shifted_rm
                mnem = "BIC" if N else "AND"
            elif opc == 0b01:
                result = rn_val | shifted_rm
                mnem = "ORN" if N else "ORR"
            elif opc == 0b10:
                result = rn_val ^ shifted_rm
                mnem = "EON" if N else "EOR"
            else:
                result = rn_val & shifted_rm
                new_nzcv = _logical_flags(result, sf)
                mnem = "BICS" if N else "ANDS"
            mask = MASK64 if sf else MASK32
            result &= mask
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Arithmetic shifted-register ───────────────────────────────────────
        if bits(28, 24) == 0b01011 and bits(21, 21) == 0:
            op = bits(30, 30)
            S = bits(29, 29)
            shift_type = bits(23, 22)
            Rm = bits(20, 16)
            imm6 = bits(15, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            rn_val = self._read_reg(Rn, sf)
            rm_val = self._read_reg(Rm, sf)
            shifted_rm = _apply_shift(rm_val, shift_type, imm6, sf)
            if op == 0:
                if S:
                    result, new_nzcv = _add_with_flags(rn_val, shifted_rm, sf)
                    mnem = "ADDS"
                else:
                    mask = MASK64 if sf else MASK32
                    result = (rn_val + shifted_rm) & mask
                    mnem = "ADD"
            else:
                if S:
                    result, new_nzcv = _sub_with_flags(rn_val, shifted_rm, sf)
                    mnem = "SUBS"
                else:
                    mask = MASK64 if sf else MASK32
                    result = (rn_val - shifted_rm) & mask
                    mnem = "SUB"
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Data Processing 2-Source: UDIV, SDIV, shifts ──────────────────────
        if bits(30, 30) == 0 and bits(28, 21) == 0b11010110:
            Rm = bits(20, 16)
            opcode2 = bits(15, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            rn_val = self._read_reg(Rn, sf)
            rm_val = self._read_reg(Rm, sf)
            bits_width = 64 if sf else 32
            mask = MASK64 if sf else MASK32
            if opcode2 == 0b000010:
                result = (rn_val // rm_val) if rm_val else 0
                mnem = "UDIV"
            elif opcode2 == 0b000011:
                if rm_val == 0:
                    result = 0
                else:
                    a = sext(rn_val, bits_width)
                    b = sext(rm_val, bits_width)
                    result = int(a / b)
                mnem = "SDIV"
            elif opcode2 == 0b001000:
                result = _apply_shift(rn_val, 0, rm_val % bits_width, sf)
                mnem = "LSLV"
            elif opcode2 == 0b001001:
                result = _apply_shift(rn_val, 1, rm_val % bits_width, sf)
                mnem = "LSRV"
            elif opcode2 == 0b001010:
                result = _apply_shift(rn_val, 2, rm_val % bits_width, sf)
                mnem = "ASRV"
            elif opcode2 == 0b001011:
                result = _apply_shift(rn_val, 3, rm_val % bits_width, sf)
                mnem = "RORV"
            else:
                return self._unknown(pc, raw, s)
            result &= mask
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Data Processing 1-Source: CLZ, RBIT, REV, REV16, REV32 ───────────
        if bits(30, 30) == 1 and bits(28, 21) == 0b11010110 and bits(20, 16) == 0:
            opcode2 = bits(15, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            rn_val = self._read_reg(Rn, sf)
            bits_width = 64 if sf else 32
            mask = MASK64 if sf else MASK32
            if opcode2 == 0b000000:
                result = _rbit(rn_val, bits_width)
                mnem = "RBIT"
            elif opcode2 == 0b000001:
                result = _rev16(rn_val, bits_width)
                mnem = "REV16"
            elif opcode2 == 0b000010:
                result = _rev(rn_val, bits_width)
                mnem = "REV"
            elif opcode2 == 0b000011 and sf == 1:
                result = _rev32(rn_val)
                mnem = "REV32"
            elif opcode2 == 0b000100:
                result = _clz(rn_val, bits_width)
                mnem = "CLZ"
            else:
                return self._unknown(pc, raw, s)
            result &= mask
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── 3-Source: MADD / MSUB / SMULH / UMULH ────────────────────────────
        if bits(28, 24) == 0b11011:
            op54 = bits(23, 21)
            Rm = bits(20, 16)
            o0 = bits(15, 15)
            Ra = bits(14, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            if op54 == 0b000:
                rn_val = self._read_reg(Rn, sf)
                rm_val = self._read_reg(Rm, sf)
                ra_val = self._read_reg(Ra, sf)
                if o0 == 0:
                    result = ra_val + rn_val * rm_val
                    mnem = "MADD"
                else:
                    result = ra_val - rn_val * rm_val
                    mnem = "MSUB"
                self._write_reg(Rd, result, sf, gpr)
            elif op54 == 0b001 and sf == 1:
                rn_val = sext(self._read_reg(Rn, 1), 64)
                rm_val = sext(self._read_reg(Rm, 1), 64)
                result = (rn_val * rm_val >> 64) & MASK64
                self._write_reg(Rd, result, 1, gpr)
                mnem = "SMULH"
            elif op54 == 0b010 and sf == 1:
                rn_val = self._read_reg(Rn, 1)
                rm_val = self._read_reg(Rm, 1)
                result = (rn_val * rm_val >> 64) & MASK64
                self._write_reg(Rd, result, 1, gpr)
                mnem = "UMULH"
            else:
                return self._unknown(pc, raw, s)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── Conditional Select: CSEL / CSINC / CSINV / CSNEG ─────────────────
        if bits(28, 21) == 0b11010100:
            op = bits(30, 30)
            Rm = bits(20, 16)
            cond = bits(15, 12)
            op2 = bits(11, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            rn_val = self._read_reg(Rn, sf)
            rm_val = self._read_reg(Rm, sf)
            mask = MASK64 if sf else MASK32
            if _condition_holds(cond, new_nzcv):
                result = rn_val
            else:
                if op == 0 and op2 == 0b00:
                    result = rm_val
                    mnem_f = "CSEL"
                elif op == 0 and op2 == 0b01:
                    result = (rm_val + 1) & mask
                    mnem_f = "CSINC"
                elif op == 1 and op2 == 0b00:
                    result = (~rm_val) & mask
                    mnem_f = "CSINV"
                elif op == 1 and op2 == 0b01:
                    result = (-rm_val) & mask
                    mnem_f = "CSNEG"
                else:
                    return self._unknown(pc, raw, s)
                self._write_reg(Rd, result, sf, gpr)
                self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
                return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem_f,
                                 description=f"{mnem_f} @ 0x{pc:04X}")
            self._write_reg(Rd, result, sf, gpr)
            if op == 0 and op2 == 0b00:
                mnem = "CSEL"
            elif op == 0 and op2 == 0b01:
                mnem = "CSINC"
            elif op == 1 and op2 == 0b00:
                mnem = "CSINV"
            elif op == 1 and op2 == 0b01:
                mnem = "CSNEG"
            else:
                return self._unknown(pc, raw, s)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── SVC: supervisor call → NOP in simulator ───────────────────────────
        if bits(31, 21) == 0b11010100_000 and bits(4, 0) == 0b00001:
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic="SVC",
                             description=f"SVC @ 0x{pc:04X}")

        return self._unknown(pc, raw, s)

    # ── FP scalar decode ──────────────────────────────────────────────────────

    def _decode_fp(
        self,
        raw: int,
        pc: int,
        next_pc: int,
        bits,  # closure from caller
        gpr: list[int],
        new_vreg: list[int],
        new_sp: int,
        new_nzcv: int,
        new_mem: list[int],
        s: AppleM1State,
    ) -> StepTrace:
        """
        Decode and execute FP scalar instructions (bits[28:24] == 11110).

        This covers:
          - FMOV FP-to-FP, FABS, FNEG, FSQRT, FCVT (1-source DP)
          - FADD, FSUB, FMUL, FDIV (2-source DP)
          - FCMP (compare, updates NZCV)
          - FMOV GPR↔FP (integer/FP register transfers)
          - FCVTZS, SCVTF, UCVTF (integer↔FP conversions)

        Dispatch is by examining bits[21] and other discriminator fields:
          - bits[21]=1, bits[15:10]=001000 → FCMP
          - bits[21]=1, bits[15:10]=000000, bits[20:16]=00110 → FMOV FP→GPR
          - bits[21]=1, bits[15:10]=000000, bits[20:16]=00111 → FMOV GPR→FP
          - bits[21]=1, bits[15:10]=000000, bits[20:16]=11000 → FCVTZS
          - bits[21]=1, bits[15:10]=000000, bits[20:16]=00010 → SCVTF
          - bits[21]=1, bits[15:10]=000000, bits[20:16]=00011 → UCVTF
          - bits[21]=1, bits[11:10]=10      → FP 2-source DP
          - bits[21]=1, bits[14:10]=10000   → FP 1-source DP
        """
        sf = bits(31, 31)
        ftype = bits(23, 22)
        is_double = ftype == 0b01
        bit21 = bits(21, 21)

        def commit(mnem: str) -> StepTrace:
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        if bit21 != 1:
            return self._unknown(pc, raw, s)

        field_15_10 = bits(15, 10)
        field_20_16 = bits(20, 16)

        # ── FCMP: bits[15:10] == 001000 ───────────────────────────────────────
        if field_15_10 == 0b001000:
            Rm = bits(20, 16)
            Rn = bits(9, 5)
            opc = bits(2, 0)
            if is_double:
                rn_f = f64_from_bits(self._read_vreg_d(Rn))
                if opc & 0b011 == 0b011:  # FCMP Rn, #0.0
                    rm_f = 0.0
                else:
                    rm_f = f64_from_bits(self._read_vreg_d(Rm))
            else:
                rn_f = f32_from_bits(self._read_vreg_s(Rn))
                if opc & 0b011 == 0b011:
                    rm_f = 0.0
                else:
                    rm_f = f32_from_bits(self._read_vreg_s(Rm))
            new_nzcv = _fp_compare(rn_f, rm_f)
            return commit("FCMP")

        # ── FMOV GPR→FP / FP→GPR / FCVTZS / SCVTF / UCVTF ───────────────────
        # All identified by bits[15:10] == 000000
        if field_15_10 == 0b000000:
            Rn = bits(9, 5)
            Rd = bits(4, 0)

            if field_20_16 == 0b00110:
                # FMOV GPR ← FP (FP→GPR transfer)
                if is_double:
                    # sf must be 1 for 64-bit
                    bits64 = self._read_vreg_d(Rn)
                    self._write_reg(Rd, bits64, 1, gpr)
                else:
                    bits32 = self._read_vreg_s(Rn)
                    self._write_reg(Rd, bits32, 0, gpr)
                return commit("FMOV")

            if field_20_16 == 0b00111:
                # FMOV FP ← GPR (GPR→FP transfer)
                if is_double:
                    val = self._read_reg(Rn, 1)
                    self._write_vreg_d(Rd, val, new_vreg)
                else:
                    val = self._read_reg(Rn, 0)
                    self._write_vreg_s(Rd, val, new_vreg)
                return commit("FMOV")

            if field_20_16 == 0b11000:
                # FCVTZS: FP → integer, truncate toward zero
                Rn_f: float
                if is_double:
                    Rn_f = f64_from_bits(self._read_vreg_d(Rn))
                else:
                    Rn_f = f32_from_bits(self._read_vreg_s(Rn))
                if math.isnan(Rn_f):
                    int_result = 0
                else:
                    int_result = int(math.trunc(Rn_f))
                    if sf:
                        # Clamp to int64 range
                        int_result = max(-(1 << 63), min((1 << 63) - 1, int_result))
                    else:
                        # Clamp to int32 range
                        int_result = max(-(1 << 31), min((1 << 31) - 1, int_result))
                self._write_reg(Rd, int_result, sf, gpr)
                return commit("FCVTZS")

            if field_20_16 == 0b00010:
                # SCVTF: signed integer → FP
                int_val = sext(self._read_reg(Rn, sf), 64 if sf else 32)
                if is_double:
                    self._write_vreg_d(Rd, f64_to_bits(float(int_val)), new_vreg)
                else:
                    self._write_vreg_s(Rd, f32_to_bits(float(int_val)), new_vreg)
                return commit("SCVTF")

            if field_20_16 == 0b00011:
                # UCVTF: unsigned integer → FP
                uint_val = self._read_reg(Rn, sf)
                if is_double:
                    self._write_vreg_d(Rd, f64_to_bits(float(uint_val)), new_vreg)
                else:
                    self._write_vreg_s(Rd, f32_to_bits(float(uint_val)), new_vreg)
                return commit("UCVTF")

            return self._unknown(pc, raw, s)

        # ── FP 2-source: FMUL/FDIV/FADD/FSUB (bits[11:10] == 10) ────────────
        if bits(11, 10) == 0b10:
            fp_opc = bits(15, 12)
            Rm = bits(20, 16)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            if is_double:
                a_f = f64_from_bits(self._read_vreg_d(Rn))
                b_f = f64_from_bits(self._read_vreg_d(Rm))
                if fp_opc == 0b0000:
                    res_f = a_f * b_f
                    mnem = "FMUL"
                elif fp_opc == 0b0001:
                    res_f = a_f / b_f if b_f != 0 else (math.copysign(float("inf"), a_f * b_f))
                    mnem = "FDIV"
                elif fp_opc == 0b0010:
                    res_f = a_f + b_f
                    mnem = "FADD"
                elif fp_opc == 0b0011:
                    res_f = a_f - b_f
                    mnem = "FSUB"
                else:
                    return self._unknown(pc, raw, s)
                self._write_vreg_d(Rd, f64_to_bits(res_f), new_vreg)
            else:
                a_f = f32_from_bits(self._read_vreg_s(Rn))
                b_f = f32_from_bits(self._read_vreg_s(Rm))
                if fp_opc == 0b0000:
                    res_f = a_f * b_f
                    mnem = "FMUL"
                elif fp_opc == 0b0001:
                    res_f = a_f / b_f if b_f != 0 else (math.copysign(float("inf"), a_f * b_f))
                    mnem = "FDIV"
                elif fp_opc == 0b0010:
                    res_f = a_f + b_f
                    mnem = "FADD"
                elif fp_opc == 0b0011:
                    res_f = a_f - b_f
                    mnem = "FSUB"
                else:
                    return self._unknown(pc, raw, s)
                # For single, round-trip through f32 (Python float is f64 internally)
                self._write_vreg_s(Rd, f32_to_bits(res_f), new_vreg)
            return commit(mnem)

        # ── FP 1-source: FMOV/FABS/FNEG/FSQRT/FCVT (bits[14:10] == 10000) ───
        if bits(14, 10) == 0b10000:
            dp1_opc = bits(20, 15)
            Rn = bits(9, 5)
            Rd = bits(4, 0)

            if dp1_opc == 0b000000:
                # FMOV Fd, Fn (FP register to FP register, same precision)
                if is_double:
                    self._write_vreg_d(Rd, self._read_vreg_d(Rn), new_vreg)
                else:
                    self._write_vreg_s(Rd, self._read_vreg_s(Rn), new_vreg)
                return commit("FMOV")

            if dp1_opc == 0b000001:
                # FABS Fd, Fn
                if is_double:
                    f = f64_from_bits(self._read_vreg_d(Rn))
                    self._write_vreg_d(Rd, f64_to_bits(abs(f)), new_vreg)
                else:
                    f = f32_from_bits(self._read_vreg_s(Rn))
                    self._write_vreg_s(Rd, f32_to_bits(abs(f)), new_vreg)
                return commit("FABS")

            if dp1_opc == 0b000010:
                # FNEG Fd, Fn
                if is_double:
                    f = f64_from_bits(self._read_vreg_d(Rn))
                    self._write_vreg_d(Rd, f64_to_bits(-f), new_vreg)
                else:
                    f = f32_from_bits(self._read_vreg_s(Rn))
                    self._write_vreg_s(Rd, f32_to_bits(-f), new_vreg)
                return commit("FNEG")

            if dp1_opc == 0b000011:
                # FSQRT Fd, Fn
                if is_double:
                    f = f64_from_bits(self._read_vreg_d(Rn))
                    self._write_vreg_d(Rd, f64_to_bits(math.sqrt(abs(f))), new_vreg)
                else:
                    f = f32_from_bits(self._read_vreg_s(Rn))
                    self._write_vreg_s(Rd, f32_to_bits(math.sqrt(abs(f))), new_vreg)
                return commit("FSQRT")

            if dp1_opc == 0b000100:
                # FCVT: precision conversion
                if is_double:
                    # ftype=01 (double input) → output is single (Sd)
                    f = f64_from_bits(self._read_vreg_d(Rn))
                    self._write_vreg_s(Rd, f32_to_bits(f), new_vreg)
                else:
                    # ftype=00 (single input) → output is double (Dd)
                    f = f32_from_bits(self._read_vreg_s(Rn))
                    self._write_vreg_d(Rd, f64_to_bits(f), new_vreg)
                return commit("FCVT")

            return self._unknown(pc, raw, s)

        return self._unknown(pc, raw, s)

    # ── NEON 3-register-same decode ───────────────────────────────────────────

    def _decode_neon_3reg(
        self,
        raw: int,
        pc: int,
        next_pc: int,
        bits,  # closure from caller
        gpr: list[int],
        new_vreg: list[int],
        new_sp: int,
        new_nzcv: int,
        new_mem: list[int],
        s: AppleM1State,
    ) -> StepTrace:
        """
        Decode and execute AdvSIMD Three-Register Same (bits[28:24] == 01110).

        Encoding: 0|Q|U|01110|size|1|Rm|opcode[15:11]|1|Rn|Rd

        Q=0 → 64-bit lane mode (lower half only); Q=1 → 128-bit (full vreg).
        size: 00=8b, 01=16b, 10=32b, 11=64b element size.
        U: unsigned flag (used to distinguish ADD vs SUB, FADD vs FSUB).

        Supported operations:
          opcode=10000 (0x10):
            U=0 → ADD integer per-element
            U=1 → SUB integer per-element
          opcode=10011 (0x13):
            MUL integer per-element (not for size=11 / 64-bit elements)
          opcode=11010 (0x1A), bit[23]=0:
            U=0 → FADD FP per-element
            U=1 → FSUB FP per-element
          opcode=11011 (0x1B), bit[23]=0:
            FMUL FP per-element
          opcode=11001 (0x19), bit[23]=0:
            FMLA (fused multiply-accumulate): Vd += Vn × Vm

        DUP from GPR:
          bits[18:14]=00001, bit[13]=1, bit[12:11]=00, bit[10]=1 → DUP
          imm5 (bits[23:19]) encodes element size and index.
        """
        Q = bits(30, 30)
        U = bits(29, 29)
        size = bits(23, 22)
        bit21 = bits(21, 21)
        Rm = bits(20, 16)
        opcode = bits(15, 11)
        bit10 = bits(10, 10)
        Rn = bits(9, 5)
        Rd = bits(4, 0)

        def commit(mnem: str) -> StepTrace:
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_vreg, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem,
                             description=f"{mnem} @ 0x{pc:04X}")

        # ── DUP from GPR ──────────────────────────────────────────────────────
        # Detection: bits[18:14]=00001, bits[13]=1, bits[12:11]=00, bit[10]=1
        # That is: bits[18:10] = 000011001 and bit[10]=1
        # Also: bit21=0, bit[29]=0 (U=0)
        if bits(18, 14) == 0b00001 and bits(13, 13) == 1 and bits(12, 11) == 0b00 and bit10 == 1:
            imm5 = bits(23, 19)
            rn_val = self._read_reg(Rn, 1)   # always read as 64-bit
            # imm5 encodes element size.  Lowest set bit determines the lane width,
            # but the neon_dup_gpr() encoder uses a one-bit-shifted scheme:
            #   bit0=1 → B (8-bit)  bit1=1,bit0=0 → H (16-bit)
            #   bit2=1,bits[1:0]=00 → S (32-bit)  bit3=1,bits[2:0]=000 → S (32-bit, shifted)
            #   bit4=1,bits[3:0]=0000 → D (64-bit)
            # In practice neon_dup_gpr uses imm5=0b01000 for 4S and imm5=0b10000 for 2D.
            if imm5 & 1:
                elem_size = 8
            elif imm5 & 2:
                elem_size = 16
            elif imm5 & 4:
                elem_size = 32
            elif imm5 & 0b01000:
                # Both standard ARM D-lane (bit3=1) and our S-lane shorthand use this
                # Distinguish: if bit4 is also set → 64-bit (D); otherwise → 32-bit (S)
                elem_size = 32
            elif imm5 & 0b10000:
                elem_size = 64
            else:
                return self._unknown(pc, raw, s)
            elem_mask = (1 << elem_size) - 1
            val_elem = rn_val & elem_mask
            total_bits = 128 if Q else 64
            result = 0
            for i in range(0, total_bits, elem_size):
                result |= val_elem << i
            if Q:
                new_vreg[Rd] = result & MASK128
            else:
                new_vreg[Rd] = result & MASK64
            return commit("DUP")

        if bit21 != 1 or bit10 != 1:
            return self._unknown(pc, raw, s)

        # ── Integer vector ops ────────────────────────────────────────────────
        if opcode == 0b10000:
            # ADD (U=0) or SUB (U=1)
            vn = new_vreg[Rn]
            vm = new_vreg[Rm]
            total_bits = 128 if Q else 64
            elem_bits = 8 << size   # 8, 16, 32, or 64
            elem_mask = (1 << elem_bits) - 1
            result = 0
            for i in range(0, total_bits, elem_bits):
                a_elem = (vn >> i) & elem_mask
                b_elem = (vm >> i) & elem_mask
                if U == 0:
                    r_elem = (a_elem + b_elem) & elem_mask
                else:
                    r_elem = (a_elem - b_elem) & elem_mask
                result |= r_elem << i
            new_vreg[Rd] = result & (MASK128 if Q else MASK64)
            return commit("VADD" if U == 0 else "VSUB")

        if opcode == 0b10011:
            # MUL (not for 64-bit elements)
            if size == 0b11:
                return self._unknown(pc, raw, s)
            vn = new_vreg[Rn]
            vm = new_vreg[Rm]
            total_bits = 128 if Q else 64
            elem_bits = 8 << size
            elem_mask = (1 << elem_bits) - 1
            result = 0
            for i in range(0, total_bits, elem_bits):
                a_elem = (vn >> i) & elem_mask
                b_elem = (vm >> i) & elem_mask
                r_elem = (a_elem * b_elem) & elem_mask
                result |= r_elem << i
            new_vreg[Rd] = result & (MASK128 if Q else MASK64)
            return commit("VMUL")

        # ── FP vector ops (bit[23] == 0, sz = bit[22]) ───────────────────────
        # For FP vector: bit[23]=0 selects this family; sz(bit[22]) = 0→f32, 1→f64
        bit23 = bits(23, 23)
        sz = bits(22, 22)
        if bit23 == 0 and opcode in (0b11010, 0b11011, 0b11001):
            is_f64 = sz == 1
            vn = new_vreg[Rn]
            vm = new_vreg[Rm]
            total_bits = 128 if Q else 64
            elem_bits = 64 if is_f64 else 32
            elem_mask = MASK64 if is_f64 else MASK32
            result = 0

            if opcode == 0b11001:
                # FMLA: Vd += Vn × Vm
                vd = new_vreg[Rd]
                for i in range(0, total_bits, elem_bits):
                    n_bits = (vn >> i) & elem_mask
                    m_bits = (vm >> i) & elem_mask
                    d_bits = (vd >> i) & elem_mask
                    if is_f64:
                        n_f = f64_from_bits(n_bits)
                        m_f = f64_from_bits(m_bits)
                        d_f = f64_from_bits(d_bits)
                        r_f = d_f + n_f * m_f
                        r_bits = f64_to_bits(r_f)
                    else:
                        n_f = f32_from_bits(n_bits)
                        m_f = f32_from_bits(m_bits)
                        d_f = f32_from_bits(d_bits)
                        r_f = d_f + n_f * m_f
                        r_bits = f32_to_bits(r_f)
                    result |= (r_bits & elem_mask) << i
                new_vreg[Rd] = result & (MASK128 if Q else MASK64)
                return commit("FMLA")

            if opcode == 0b11010:
                # FADD (U=0) or FSUB (U=1)
                for i in range(0, total_bits, elem_bits):
                    n_bits = (vn >> i) & elem_mask
                    m_bits = (vm >> i) & elem_mask
                    if is_f64:
                        n_f = f64_from_bits(n_bits)
                        m_f = f64_from_bits(m_bits)
                        r_f = (n_f + m_f) if U == 0 else (n_f - m_f)
                        r_bits = f64_to_bits(r_f)
                    else:
                        n_f = f32_from_bits(n_bits)
                        m_f = f32_from_bits(m_bits)
                        r_f = (n_f + m_f) if U == 0 else (n_f - m_f)
                        r_bits = f32_to_bits(r_f)
                    result |= (r_bits & elem_mask) << i
                new_vreg[Rd] = result & (MASK128 if Q else MASK64)
                return commit("VFADD" if U == 0 else "VFSUB")

            if opcode == 0b11011:
                # FMUL
                for i in range(0, total_bits, elem_bits):
                    n_bits = (vn >> i) & elem_mask
                    m_bits = (vm >> i) & elem_mask
                    if is_f64:
                        n_f = f64_from_bits(n_bits)
                        m_f = f64_from_bits(m_bits)
                        r_bits = f64_to_bits(n_f * m_f)
                    else:
                        n_f = f32_from_bits(n_bits)
                        m_f = f32_from_bits(m_bits)
                        r_bits = f32_to_bits(n_f * m_f)
                    result |= (r_bits & elem_mask) << i
                new_vreg[Rd] = result & (MASK128 if Q else MASK64)
                return commit("VFMUL")

        return self._unknown(pc, raw, s)
