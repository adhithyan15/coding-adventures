"""
AArch64 (2011) Behavioral Simulator
======================================

AArch64 is the 64-bit instruction set architecture introduced in ARMv8-A
(2011) by Arm Ltd.  The first silicon to ship it was the Apple A7 (iPhone 5s,
2013); it later powered Apple M1 (2020), making AArch64 the most shipped 64-bit
architecture by unit count as of 2024.

Design philosophy
-----------------
AArch64 was a clean break from ARMv7 / Thumb.  Key choices:

  1. Fixed 32-bit instruction width — simpler decode, better I-cache density.
  2. 31 × 64-bit general-purpose registers (X0–X30) + XZR (always-zero) +
     separate SP and PC — eliminating the ARM1/ARMv7 "PC is a register" quirk.
  3. Load/store architecture — arithmetic never accesses memory directly,
     making pipeline design and out-of-order execution cleaner.
  4. NZCV condition flags — only updated by S-suffix or compare instructions,
     not implicitly by every instruction (unlike x86).
  5. Bitmask immediates — logical immediates encode complex repeating bitmasks
     in just 13 bits via a (N, immr, imms) triple.

Register conventions (ABI)
--------------------------
  X0–X7    argument / result registers
  X8       indirect result location / system call number
  X9–X15   caller-saved temporaries
  X16–X17  intra-procedure-call scratch (IP0/IP1)
  X18      platform register (reserved by some ABIs)
  X19–X28  callee-saved registers
  X29      frame pointer (FP)
  X30      link register (LR) — written by BL/BLR; implicitly read by RET

Instruction encoding summary
-----------------------------
All instructions are 32 bits.  The top bits select the encoding class:

  Data Processing Immediate     sf | op | S | 100010x or 100001x | ...
  Data Processing Register      sf | op | S | 01011 | shift | ...
  Logical Immediate             sf | opc | 0 | 100100 | N | immr | imms | ...
  Move Wide Immediate           sf | opc | 100101 | hw | imm16 | Rd
  Load/Store Unsigned Offset    size | 111 | V | 01 | opc | imm12 | Rn | Rt
  Unconditional Branch (imm)    op | 000101 | imm26
  Conditional Branch            01010100 | imm19 | 0 | cond
  Compare-and-Branch            sf | 011010 | op | imm19 | Rt
  Unconditional Branch (reg)    1101011 0 | op | 11111 | 000000 | Rn | 00000
  3-Source (MADD/MSUB)          sf | 0 | 0 | 11011 | op54 | Rm | 0 | Ra | Rn | Rd
  Data Processing 1-Source      sf | 1 | S | 11010110 | 00000 | opcode2 | Rn | Rd
  Data Processing 2-Source      sf | 0 | S | 11010110 | Rm | opcode2 | Rn | Rd
  Conditional Select             sf | op | S | 11010100 | Rm | cond | op2 | Rn | Rd
  NZCV affects                  only S-suffix (ADDS/SUBS/ANDS/BICS) + CMP/CMN/TST

NZCV update rules
-----------------
  Arithmetic (ADD/SUB with flags):
    N = MSB of result
    Z = (result == 0)
    C = unsigned carry-out (for ADD); borrow-complement (for SUB)
    V = signed overflow (both operands same sign, result different sign)

  Logical (AND/ORR/EOR/BIC with flags):
    N = MSB of result
    Z = (result == 0)
    C = 0
    V = 0

Bitmask immediate decoding
--------------------------
AArch64 logical immediates encode a repeating bitmask via (N, immr, imms):
  1. element size = 2^len where len is derived from N and ~imms
  2. S = imms & (esize-1) → number of set bits minus 1
  3. R = immr & (esize-1) → right-rotation amount within esize bits
  4. welem = (1 << (S+1)) - 1 (a run of S+1 ones)
  5. telem = ror(welem, R, esize)
  6. result = telem replicated to fill 64 bits

HALT sentinel
-------------
  0x00000000 (UDF #0) — permanently undefined in real AArch64; used here to
  stop the simulation loop.

Simplifications
---------------
  - No FPR/SIMD registers (V0–V31)
  - No exception levels (EL0–EL3)
  - No MMU; addresses wrap modulo 65536
  - UDIV/SDIV by zero returns 0 (UNDEFINED per spec; our choice)
  - SVC/HVC/SMC treated as NOP
  - Memory barriers (DMB/DSB/ISB) treated as NOP
  - Only NZCV tracked; DAIF and other PSTATE fields ignored
"""

from __future__ import annotations

import struct

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from .state import (
    MASK32,
    MASK64,
    MEM_SIZE,
    AArch64State,
    make_initial_state,
    sext,
    sext19,
    sext26,
    sext32,
)

# ── HALT word ───────────────────────────────────────────────────────────────────

HALT: bytes = b"\x00\x00\x00\x00"

# ── Instruction encoding helpers ─────────────────────────────────────────────────
# These helpers let tests assemble small programs without a real assembler.
# Each function returns exactly 4 bytes in big-endian order.


def _u32be(v: int) -> bytes:
    """Pack a 32-bit value as big-endian bytes."""
    return struct.pack(">I", v & 0xFFFF_FFFF)


def dp_imm(sf: int, op: int, S: int, imm12: int, sh: int, Rn: int, Rd: int) -> bytes:
    """
    Encode a Data Processing Immediate (ADD/SUB family) instruction.

    Encoding: sf | op | S | 100000 | sh | imm12 | Rn | Rd
    sf=1→64-bit, sf=0→32-bit.  sh=1 shifts imm12 left by 12.
    op=0→ADD, op=1→SUB.  S=1 sets NZCV.

    Examples::
        dp_imm(1, 0, 0, 5, 0, 0, 0)   # ADD X0, X0, #5
        dp_imm(1, 1, 1, 0, 0, 1, 31)  # CMP X1, #0  (SUBS XZR, X1, #0)
    """
    # bits[28:23] = 10000x where x=sh, bits[22]=sh, bits[21:10]=imm12,
    # bits[9:5]=Rn, bits[4:0]=Rd
    # Full encoding: [sf:1][op:1][S:1][100000:6][sh:1][imm12:12][Rn:5][Rd:5]
    # = bits 31..29 = sf,op,S; bits 28..23 = 100000; bit 22 = sh;
    #   bits 21..10 = imm12; bits 9..5 = Rn; bits 4..0 = Rd
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b100000 << 23)
    v |= ((sh & 1) << 22)
    v |= ((imm12 & 0xFFF) << 10)
    v |= ((Rn & 0x1F) << 5)
    v |= (Rd & 0x1F)
    return _u32be(v)


def dp_reg(sf: int, op: int, S: int, shift: int, Rm: int, imm6: int, Rn: int, Rd: int) -> bytes:
    """
    Encode a Data Processing Register (shifted-register ADD/SUB/logical) instruction.

    Encoding: sf | op | S | 01011 | shift | 0 | Rm | imm6 | Rn | Rd
    op/S select operation; shift: 00=LSL, 01=LSR, 10=ASR, 11=ROR.
    For ADD/SUB: op=0→ADD, op=1→SUB.  For logical see logic_reg().

    Examples::
        dp_reg(1, 0, 0, 0, 2, 0, 1, 0)  # ADD X0, X1, X2, LSL #0
        dp_reg(1, 1, 1, 0, 3, 0, 4, 31) # CMP X4, X3 (SUBS XZR,X4,X3)
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
    Encode a Logical Immediate instruction (AND/ORR/EOR/ANDS with bitmask immediate).

    Encoding: sf | opc | 0 | 100100 | N | immr | imms | Rn | Rd
    opc: 00=AND, 01=ORR, 10=EOR, 11=ANDS.

    Examples::
        logic_imm(1, 0b01, 1, 0, 62, 0, 1)  # ORR X1, X0, #-1  (MOV X1, #-1)
        logic_imm(1, 0b11, 1, 0, 62, 1, 31) # TST X1, #-1 (all bits)
    """
    v = ((sf & 1) << 31) | ((opc & 3) << 29)
    v |= (0b0100100 << 22)   # bits[28:22] = 0100100 → includes the fixed 0 and 100100
    v |= ((N & 1) << 22)
    v |= ((immr & 0x3F) << 16)
    v |= ((imms & 0x3F) << 10)
    v |= ((Rn & 0x1F) << 5)
    v |= (Rd & 0x1F)
    # Re-encode cleanly: sf[31] opc[30:29] 0[28] 100100[27:22] N[22] immr[21:16] imms[15:10] Rn[9:5] Rd[4:0]
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
    Encode a Logical (shifted-register) instruction.

    Encoding: sf | opc | 01010 | shift | N | Rm | imm6 | Rn | Rd
    opc: 00=AND, 01=ORR, 10=EOR, 11=ANDS.  N=1 inverts Rm (BIC/ORN/EON/BICS).

    Examples::
        logic_reg(1, 0b01, 0, 0, 2, 0, 31, 1)   # ORR X1, XZR, X2  (MOV X1, X2)
        logic_reg(1, 0b00, 0, 1, 3, 0, 2, 4)    # BIC X4, X2, X3
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
    Encode a Move Wide Immediate instruction.

    Encoding: sf | opc | 100101 | hw | imm16 | Rd
    opc: 00=MOVN, 10=MOVZ, 11=MOVK.  hw: shift = hw×16.

    Examples::
        movwide(1, 0b10, 0, 42, 0)    # MOVZ X0, #42
        movwide(1, 0b00, 0, 0, 1)     # MOVN X1, #0  (X1 = -1)
    """
    v = ((sf & 1) << 31) | ((opc & 3) << 29)
    v |= (0b100101 << 23)
    v |= ((hw & 3) << 21)
    v |= ((imm16 & 0xFFFF) << 5)
    v |= (Rd & 0x1F)
    return _u32be(v)


def ldst_uoff(size: int, V: int, opc: int, imm12: int, Rn: int, Rt: int) -> bytes:
    """
    Encode a Load/Store with Unsigned Offset instruction.

    Encoding: size | 111 | V | 01 | opc | imm12 | Rn | Rt
    EA = Rn + (imm12 << size).  V=0 (integer), V=1 (SIMD/FP — not simulated).

    size/opc combinations:
      size=00,opc=00 → STRB Wt, [Xn, #imm]
      size=00,opc=01 → LDRB Wt, [Xn, #imm]
      size=00,opc=10 → LDRSB Xt, [Xn, #imm] (sign-extend to 64)
      size=01,opc=00 → STRH Wt, [Xn, #imm]
      size=01,opc=01 → LDRH Wt, [Xn, #imm]
      size=01,opc=10 → LDRSH Xt, [Xn, #imm] (sign-extend to 64)
      size=10,opc=00 → STR Wt, [Xn, #imm]
      size=10,opc=01 → LDR Wt, [Xn, #imm]
      size=10,opc=10 → LDRSW Xt, [Xn, #imm] (sign-extend to 64)
      size=11,opc=00 → STR Xt, [Xn, #imm]
      size=11,opc=01 → LDR Xt, [Xn, #imm]

    Examples::
        ldst_uoff(3, 0, 0b01, 0, 0, 1)    # LDR X1, [X0]
        ldst_uoff(3, 0, 0b00, 0, 1, 0)    # STR X0, [X1]
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
    Encode an Unconditional Branch (immediate) instruction.

    Encoding: op[31] | 00101[30:26] | imm26[25:0]
    op=0 → B; op=1 → BL.  Target = PC + SignExtend(imm26×4, 64).

    Examples::
        branch_imm(0, 3)    # B #+12 (skip 3 instructions forward)
        branch_imm(1, -1)   # BL #-4 (call one instruction back)
    """
    v = ((op & 1) << 31) | (0b00101 << 26) | (imm26 & 0x3FF_FFFF)
    return _u32be(v)


def branch_cond(imm19: int, cond: int) -> bytes:
    """
    Encode a Conditional Branch (immediate) instruction.

    Encoding: 01010100 | imm19 | 0 | cond
    Target = PC + SignExtend(imm19×4, 64).

    Condition codes::
        COND_EQ = 0b0000   COND_NE = 0b0001
        COND_CS = 0b0010   COND_CC = 0b0011
        COND_MI = 0b0100   COND_PL = 0b0101
        COND_VS = 0b0110   COND_VC = 0b0111
        COND_HI = 0b1000   COND_LS = 0b1001
        COND_GE = 0b1010   COND_LT = 0b1011
        COND_GT = 0b1100   COND_LE = 0b1101
        COND_AL = 0b1110

    Examples::
        branch_cond(-5, COND_EQ)   # B.EQ #-20 (5 instructions back)
    """
    v = (0b01010100 << 24) | ((imm19 & 0x7FFFF) << 5) | (cond & 0xF)
    return _u32be(v)


def cbz_cbnz(sf: int, op: int, imm19: int, Rt: int) -> bytes:
    """
    Encode a Compare-and-Branch instruction.

    Encoding: sf | 011010 | op | imm19 | Rt
    op=0→CBZ (branch if Rt==0); op=1→CBNZ (branch if Rt!=0).

    Examples::
        cbz_cbnz(1, 0, -3, 0)   # CBZ X0, #-12 (3 back)
        cbz_cbnz(1, 1, 2, 1)    # CBNZ X1, #+8 (2 forward)
    """
    v = ((sf & 1) << 31) | (0b011010 << 25) | ((op & 1) << 24)
    v |= ((imm19 & 0x7FFFF) << 5) | (Rt & 0x1F)
    return _u32be(v)


def branch_reg(op: int, Rn: int) -> bytes:
    """
    Encode an Unconditional Branch (register) instruction.

    Encoding: 1101011 0 | op | 11111 | 000000 | Rn | 00000
    op=00→BR, op=01→BLR, op=10→RET.

    Examples::
        branch_reg(0b00, 1)    # BR X1
        branch_reg(0b01, 30)   # BLR X30 (call via X30)
        branch_reg(0b10, 30)   # RET  (return via X30/LR)
    """
    # bits[31:25]=1101011, bit[24]=0, bits[23:21]=op, bits[20:16]=11111,
    # bits[15:10]=000000, bits[9:5]=Rn, bits[4:0]=00000
    v = (0b1101011_0 << 24) | ((op & 0x7) << 21) | (0b11111 << 16) | ((Rn & 0x1F) << 5)
    return _u32be(v)


def madd_msub(sf: int, op54: int, Rm: int, o0: int, Ra: int, Rn: int, Rd: int) -> bytes:
    """
    Encode a 3-source Data Processing instruction (MADD / MSUB).

    Encoding: sf | 0 | 0 | 11011 | op54 | Rm | 0 | Ra | Rn | Rd
    op54=000,o0=0→MADD; op54=000,o0=1→MSUB.

    MADD: Rd = Ra + Rn × Rm
    MSUB: Rd = Ra − Rn × Rm

    Examples::
        madd_msub(1, 0, 2, 0, 31, 0, 1)   # MUL X1, X0, X2 (MADD Ra=XZR)
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
    Encode a Conditional Select instruction.

    Encoding: sf | op | S | 11010100 | Rm | cond | op2 | Rn | Rd
    op/op2 combinations:
      op=0,S=0,op2=00 → CSEL   Rd = cond ? Rn : Rm
      op=0,S=0,op2=01 → CSINC  Rd = cond ? Rn : Rm+1
      op=1,S=0,op2=00 → CSINV  Rd = cond ? Rn : ~Rm
      op=1,S=0,op2=01 → CSNEG  Rd = cond ? Rn : -Rm

    Examples::
        csel_enc(1, 0, 0, 2, COND_EQ, 0b00, 1, 0)  # CSEL X0, X1, X2, EQ
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
    Encode a Test-and-Branch instruction (TBZ / TBNZ).

    Encoding: b5 | 011011 | op | b40 | imm14 | Rt
    Tests bit (b5<<5 | b40) of Rt.  op=0→TBZ, op=1→TBNZ.
    Target = PC + SignExtend(imm14×4, 64).

    Examples::
        tbz_tbnz(0, 0, 0, 2, 1)   # TBZ W1, #0, #+8 (branch if bit0 clear)
    """
    v = ((b5 & 1) << 31) | (0b011011 << 25) | ((op & 1) << 24)
    v |= ((b40 & 0x1F) << 19) | ((imm14 & 0x3FFF) << 5) | (Rt & 0x1F)
    return _u32be(v)


# ── Condition codes ────────────────────────────────────────────────────────────

COND_EQ: int = 0b0000   # Equal (Z=1)
COND_NE: int = 0b0001   # Not equal (Z=0)
COND_CS: int = 0b0010   # Carry set / unsigned higher or same (C=1)
COND_CC: int = 0b0011   # Carry clear / unsigned lower (C=0)
COND_MI: int = 0b0100   # Minus / negative (N=1)
COND_PL: int = 0b0101   # Plus / positive or zero (N=0)
COND_VS: int = 0b0110   # Overflow (V=1)
COND_VC: int = 0b0111   # No overflow (V=0)
COND_HI: int = 0b1000   # Unsigned higher (C=1 and Z=0)
COND_LS: int = 0b1001   # Unsigned lower or same (C=0 or Z=1)
COND_GE: int = 0b1010   # Signed greater-than or equal (N==V)
COND_LT: int = 0b1011   # Signed less-than (N!=V)
COND_GT: int = 0b1100   # Signed greater-than (Z=0 and N==V)
COND_LE: int = 0b1101   # Signed less-than or equal (Z=1 or N!=V)
COND_AL: int = 0b1110   # Always

# ── Internal helpers ──────────────────────────────────────────────────────────


def _condition_holds(cond: int, nzcv: int) -> bool:
    """
    Evaluate whether a 4-bit condition code is satisfied given the NZCV flags.

    The top 3 bits of `cond` select the base test; the bottom bit inverts the
    result for the 'odd' members of each pair (NE vs EQ, CC vs CS, etc.),
    except that cond=0b1111 (NV — should not arise) is not inverted.

    Truth table::
        0b0000 EQ : Z==1
        0b0001 NE : Z==0
        0b0010 CS : C==1
        0b0011 CC : C==0
        0b0100 MI : N==1
        0b0101 PL : N==0
        0b0110 VS : V==1
        0b0111 VC : V==0
        0b1000 HI : C==1 and Z==0
        0b1001 LS : C==0 or Z==1
        0b1010 GE : N==V
        0b1011 LT : N!=V
        0b1100 GT : Z==0 and N==V
        0b1101 LE : Z==1 or N!=V
        0b1110 AL : always true
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
    # Invert for the odd member of each pair — but not for AL (cond=0b1110)
    if (cond & 1) and cond != 0xF:
        result = not result
    return result


def _add_with_flags(a: int, b: int, sf: int) -> tuple[int, int]:
    """
    Perform addition and compute NZCV flags.

    Implements A + B using borrow-complement carry convention (same as hardware).
    Returns (result_masked, nzcv) where result is masked to sf-width.

    sf=1 → 64-bit operation; sf=0 → 32-bit operation.

    Examples::
        _add_with_flags(0xFFFF_FFFF_FFFF_FFFF, 1, 1)
        # → (0, 0b0110) — zero, carry-out (unsigned overflow), no signed overflow
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


def _logical_flags(result: int, sf: int) -> int:
    """
    Compute NZCV flags after a logical operation (AND/ORR/EOR/BIC).

    C and V are always cleared; only N and Z reflect the result.
    """
    bits = 64 if sf else 32
    N = (result >> (bits - 1)) & 1
    Z = 1 if result == 0 else 0
    return (N << 3) | (Z << 2)


def _apply_shift(value: int, shift_type: int, amount: int, sf: int) -> int:
    """
    Apply a shift operation to `value`.

    shift_type: 0=LSL, 1=LSR (logical), 2=ASR (arithmetic), 3=ROR.
    amount: shift amount (0–63 for sf=1; 0–31 for sf=0).
    sf: 1→64-bit operands, 0→32-bit.

    For ASR the value is treated as a two's-complement signed integer.
    """
    bits = 64 if sf else 32
    mask = MASK64 if sf else MASK32
    value &= mask
    amount &= (bits - 1)   # modulo bit-width
    if amount == 0:
        return value
    if shift_type == 0:   # LSL
        return (value << amount) & mask
    elif shift_type == 1:  # LSR (logical — fills with 0)
        return value >> amount
    elif shift_type == 2:  # ASR (arithmetic — fills with sign bit)
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
    Decode the AArch64 logical-immediate encoding into a 64-bit bitmask.

    AArch64 stores logical immediates as a (N, immr, imms) triple that encodes
    a repeating bitmask of any element size from 2 to 64 bits.  The algorithm:

    1. Determine element size:
       - N=1 → 64-bit elements (len=6, esize=64)
       - N=0 → use highest set bit of (~imms & 63) | (N << 6) minus 1
    2. S = imms & (esize-1)   — number of set bits minus 1
       R = immr & (esize-1)   — right-rotation amount
    3. welem = (1 << (S+1)) - 1  — a run of S+1 consecutive 1s
    4. telem = ror(welem, R, esize)  — rotate right by R within esize
    5. Replicate telem to fill 64 bits.

    Raises ValueError for the UNDEFINED encoding (N=0, all-zero ~imms field).
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


# ── Simulator constants ──────────────────────────────────────────────────────────

# Hard ceiling on max_steps to prevent accidental DoS from very large values.
# At ~1 µs per step (interpreted Python), 10 M steps ≈ 10 seconds wall time.
_MAX_STEPS_LIMIT: int = 10_000_000


# ── Simulator ───────────────────────────────────────────────────────────────────


class AArch64Simulator(Simulator[AArch64State]):
    """
    Behavioral simulator for the AArch64 (ARMv8-A, 2011) integer instruction set.

    Implements the SIM00 Simulator[AArch64State] protocol:
      reset()        — zero all state (all X registers, SP, PC, NZCV, memory)
      load(prog)     — reset() then copy program bytes into memory[0x0000…]
      step()         — fetch-decode-execute one instruction; return StepTrace
      execute(prog)  — load() then step loop until HALT or max_steps
      get_state()    — return a frozen AArch64State snapshot

    Register model:
      - X0–X30 are general-purpose 64-bit registers.
      - XZR (index 31) always reads 0; writes are silently discarded.
      - SP is separate and used by load/store SP-based addressing.
      - PC is advanced by 4 per instruction; branches overwrite it.
      - NZCV is a 4-bit nibble updated only by S-suffix and compare instructions.
      - W-register writes zero-extend to fill the full 64-bit register.
      - 32-bit results are zero-extended to 64 bits before being written.

    Memory model:
      - 64 KiB byte-addressed big-endian memory.
      - Addresses wrap modulo MEM_SIZE (65 536).
      - Unaligned accesses succeed (aligned to natural boundary by masking).

    Simplifications:
      - No FPR/SIMD (V0–V31 not simulated).
      - No exception levels or MMU.
      - UDIV/SDIV by zero returns 0 (UNDEFINED in spec).
      - SVC/HVC/SMC → NOP.
      - Memory barriers (DMB/DSB/ISB) → NOP.
    """

    def __init__(self) -> None:
        self._state: AArch64State = make_initial_state()

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Zero all registers, memory, PC, NZCV, SP, and halted flag."""
        self._state = make_initial_state()

    def load(self, program: bytes) -> None:
        """
        Reset the simulator and copy `program` bytes into memory starting at 0x0000.

        The program should end with the 4-byte HALT word (0x00000000).  Extra
        bytes beyond the program are left as 0 from reset().  Bytes beyond
        MEM_SIZE are silently ignored (the address space is only 64 KiB).
        """
        self.reset()
        s = self._state
        mem = list(s.memory)
        for i, byte in enumerate(program[:MEM_SIZE]):  # truncate; no wrapping
            mem[i] = byte
        self._state = AArch64State(
            pc=s.pc,
            gpr=s.gpr,
            sp=s.sp,
            nzcv=s.nzcv,
            memory=tuple(mem),
            halted=s.halted,
        )

    def step(self) -> StepTrace:
        """
        Fetch the instruction at PC, decode it, execute it, and return a StepTrace.

        The StepTrace records the instruction's PC, mnemonic, and any register
        or memory changes.  If the simulator is already halted, returns a HALT
        trace without advancing PC.

        The PC is advanced to PC+4 *before* execution begins, so branch
        instructions that compute offsets relative to the instruction address
        must account for this (they add their offset to the *pre-advance* PC).
        """
        s = self._state
        pc = s.pc

        # ── Already halted ────────────────────────────────────────────────────
        if s.halted:
            return StepTrace(
                pc_before=pc,
                pc_after=pc,
                mnemonic="HALT",
                description=f"HALT @ 0x{pc:04X}",
            )

        # ── Fetch ─────────────────────────────────────────────────────────────
        # Read 4 bytes big-endian from memory; wrap on overflow.
        raw = (
            (s.memory[pc % MEM_SIZE] << 24)
            | (s.memory[(pc + 1) % MEM_SIZE] << 16)
            | (s.memory[(pc + 2) % MEM_SIZE] << 8)
            | s.memory[(pc + 3) % MEM_SIZE]
        )

        # ── HALT check ────────────────────────────────────────────────────────
        if raw == 0:
            self._state = AArch64State(
                pc=pc,
                gpr=s.gpr,
                sp=s.sp,
                nzcv=s.nzcv,
                memory=s.memory,
                halted=True,
            )
            return StepTrace(pc_before=pc, pc_after=pc, mnemonic="HALT", description=f"HALT @ 0x{pc:04X}")

        # Advance PC past the fetched instruction (branches may overwrite this)
        next_pc = (pc + 4) & MASK64
        return self._decode_execute(raw, pc, next_pc, s)

    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult:
        """
        Load `program` and step until HALT or `max_steps` steps.

        Returns an ExecutionResult with final_state, steps, halted, error, and traces.
        error is None on clean halt; a non-None string describes what went wrong.
        ok (property) is True only when halted=True and error=None.

        Raises ValueError if max_steps is outside [1, _MAX_STEPS_LIMIT].  This
        prevents accidental DoS from absurdly large step counts and rejects the
        silent no-op produced by max_steps=0.
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
                    halted=False,
                    steps=len(traces),
                    final_state=self._state,
                    traces=traces,
                    error=trace.mnemonic,
                )
            if self._state.halted:
                return ExecutionResult(
                    halted=True,
                    steps=len(traces),
                    final_state=self._state,
                    traces=traces,
                    error=None,
                )
        return ExecutionResult(
            halted=False,
            steps=max_steps,
            final_state=self._state,
            traces=traces,
            error=f"max_steps={max_steps} exceeded",
        )

    def get_state(self) -> AArch64State:
        """Return a frozen snapshot of the current simulator state."""
        return self._state

    # ── Register read/write helpers ───────────────────────────────────────────

    def _read_reg(self, idx: int, sf: int) -> int:
        """
        Read a register value.

        idx=31 always returns 0 (XZR).  If sf=0 the value is masked to 32 bits.
        """
        if idx == 31:
            return 0
        val = self._state.gpr[idx]
        return val if sf else (val & MASK32)

    def _write_reg(self, idx: int, value: int, sf: int, new_gpr: list[int]) -> None:
        """
        Write to a register in `new_gpr` (a mutable list being built).

        idx=31 is XZR — the write is silently discarded.  If sf=0 the value is
        zero-extended from 32 bits to 64 bits (W-register semantics).
        """
        if idx == 31:
            return   # XZR writes are discarded
        if sf:
            new_gpr[idx] = value & MASK64
        else:
            new_gpr[idx] = value & MASK32   # zero-extend to 64 bits

    # ── Memory read/write helpers ─────────────────────────────────────────────

    def _mem_read(self, addr: int, nbytes: int, memory: tuple[int, ...]) -> int:
        """Read `nbytes` big-endian from `memory` at `addr` (wraps modulo MEM_SIZE)."""
        result = 0
        for i in range(nbytes):
            result = (result << 8) | memory[(addr + i) % MEM_SIZE]
        return result

    def _mem_write(
        self, addr: int, value: int, nbytes: int, new_mem: list[int]
    ) -> None:
        """Write `nbytes` of `value` big-endian to `new_mem` at `addr` (wraps)."""
        for i in range(nbytes - 1, -1, -1):
            new_mem[(addr + i) % MEM_SIZE] = value & 0xFF
            value >>= 8

    # ── Decode and execute ────────────────────────────────────────────────────

    def _decode_execute(
        self, raw: int, pc: int, next_pc: int, s: AArch64State
    ) -> StepTrace:
        """
        Decode a 32-bit instruction word and execute it.

        AArch64 uses a hierarchical bit-field scheme to identify instruction
        classes.  The key discriminator bits are:

          bits[28:25] — the 'op0' field in the ARM Architecture Reference Manual,
          together with bits[31:29], select the major encoding class.

        Rather than a strict hierarchy, we use a sequence of pattern-match checks
        ordered from most-specific (narrow bit patterns) to least-specific.

        Returns a StepTrace.  On unknown opcode, returns an ERROR trace and halts.
        """
        # Snapshot mutable state we'll build on
        gpr = list(s.gpr)
        new_pc = next_pc
        new_sp = s.sp
        new_nzcv = s.nzcv
        new_mem = list(s.memory)

        # ── Bit-field extraction helpers ──────────────────────────────────────
        def bits(hi: int, lo: int) -> int:
            """Extract bits[hi:lo] inclusive from raw."""
            width = hi - lo + 1
            return (raw >> lo) & ((1 << width) - 1)

        sf = bits(31, 31)

        # ── HALT (already handled in step(), but be safe) ─────────────────────
        if raw == 0:
            self._commit(pc, s.gpr, s.sp, s.nzcv, s.memory, halted=True)
            return StepTrace(pc_before=pc, pc_after=pc, mnemonic="HALT", description=f"HALT @ 0x{pc:04X}")

        # ── NOP (canonical: 0xD503201F) ───────────────────────────────────────
        if raw == 0xD503201F:
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic="NOP", description=f"NOP @ 0x{pc:04X}")

        # ── Unconditional branch (immediate): B / BL ──────────────────────────
        # Encoding: op[31] | 00101[30:26] | imm26[25:0]
        if bits(30, 26) == 0b00101:
            op = bits(31, 31)
            imm26 = sext26(bits(25, 0))
            target = (pc + imm26 * 4) & MASK64
            if op:   # BL: save return address in X30
                gpr[30] = next_pc & MASK64
                mnem = "BL"
            else:
                mnem = "B"
            new_pc = target
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Conditional branch (immediate): B.cond ────────────────────────────
        # Encoding: 01010100[31:24] | imm19[23:5] | 0[4] | cond[3:0]
        if bits(31, 24) == 0b01010100 and bits(4, 4) == 0:
            imm19 = sext19(bits(23, 5))
            cond = bits(3, 0)
            mnem = f"B.{_cond_name(cond)}"
            if _condition_holds(cond, new_nzcv):
                new_pc = (pc + imm19 * 4) & MASK64
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Compare-and-Branch: CBZ / CBNZ ───────────────────────────────────
        # Encoding: sf[31] | 011010[30:25] | op[24] | imm19[23:5] | Rt[4:0]
        if bits(30, 25) == 0b011010:
            op = bits(24, 24)
            imm19 = sext19(bits(23, 5))
            Rt = bits(4, 0)
            rt_val = self._read_reg(Rt, sf)
            taken = (rt_val == 0) if op == 0 else (rt_val != 0)
            mnem = "CBZ" if op == 0 else "CBNZ"
            if taken:
                new_pc = (pc + imm19 * 4) & MASK64
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Test-and-Branch: TBZ / TBNZ ──────────────────────────────────────
        # Encoding: b5[31] | 011011[30:25] | op[24] | b40[23:19] | imm14[18:5] | Rt[4:0]
        if bits(30, 25) == 0b011011:
            b5 = bits(31, 31)
            op = bits(24, 24)
            b40 = bits(23, 19)
            bit_num = (b5 << 5) | b40
            imm14 = sext(bits(18, 5), 14)
            Rt = bits(4, 0)
            rt_val = self._read_reg(Rt, 1)   # always 64-bit for TBZ
            bit_val = (rt_val >> bit_num) & 1
            taken = (bit_val == 0) if op == 0 else (bit_val != 0)
            mnem = "TBZ" if op == 0 else "TBNZ"
            if taken:
                new_pc = (pc + imm14 * 4) & MASK64
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Unconditional branch (register): BR / BLR / RET ──────────────────
        # Encoding: 1101011_0[31:24] | op[23:21] | 11111[20:16] | 000000[15:10] | Rn[9:5] | 00000[4:0]
        if bits(31, 24) == 0b1101_0110:
            op = bits(23, 21)
            Rn = bits(9, 5)
            rn_val = self._read_reg(Rn, 1)   # always 64-bit
            if op == 0b000:   # BR
                new_pc = rn_val & MASK64
                mnem = "BR"
            elif op == 0b001:  # BLR
                gpr[30] = next_pc & MASK64
                new_pc = rn_val & MASK64
                mnem = "BLR"
            elif op == 0b010:  # RET
                new_pc = rn_val & MASK64
                mnem = "RET"
            else:
                return self._unknown(pc, next_pc, raw, s)
            self._commit(new_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=new_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Data Processing Immediate: ADD/SUB (immediate) ────────────────────
        # Encoding: sf[31] | op[30] | S[29] | 100000[28:23] | sh[22] | imm12[21:10] | Rn[9:5] | Rd[4:0]
        # Also catches 100001 (the shifted variant uses bit[22] for shift)
        if bits(28, 23) in (0b100000, 0b100001):
            op = bits(30, 30)     # 0=ADD, 1=SUB
            S = bits(29, 29)
            sh = bits(22, 22)
            imm12 = bits(21, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            imm = imm12 << 12 if sh else imm12
            rn_val = self._read_reg(Rn, sf)
            if op == 0:   # ADD
                if S:
                    result, new_nzcv = _add_with_flags(rn_val, imm, sf)
                    mnem = "ADDS"
                else:
                    mask = MASK64 if sf else MASK32
                    result = (rn_val + imm) & mask
                    mnem = "ADD"
            else:           # SUB: compute A + (~B + 1)
                if S:
                    result, new_nzcv = _add_with_flags(rn_val, (~imm) + 1, sf)
                    # For SUB, carry convention: C=1 means no borrow
                    # _add_with_flags handles this correctly since sub = a + ~b + 1
                    # But actually we compute sub as a + (~b), then + 1.
                    # Simpler: use twos-complement directly
                    result, new_nzcv = _sub_with_flags(rn_val, imm, sf)
                    mnem = "SUBS"
                else:
                    mask = MASK64 if sf else MASK32
                    result = (rn_val - imm) & mask
                    mnem = "SUB"
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Move Wide Immediate: MOVZ / MOVN / MOVK ──────────────────────────
        # Encoding: sf[31] | opc[30:29] | 100101[28:23] | hw[22:21] | imm16[20:5] | Rd[4:0]
        if bits(28, 23) == 0b100101:
            opc = bits(30, 29)
            hw = bits(22, 21)
            imm16 = bits(20, 5)
            Rd = bits(4, 0)
            shift = hw * 16
            if opc == 0b10:   # MOVZ
                result = imm16 << shift
                mnem = "MOVZ"
            elif opc == 0b00:  # MOVN
                result = ~(imm16 << shift)
                mnem = "MOVN"
            elif opc == 0b11:  # MOVK
                cur = self._read_reg(Rd, sf)
                mask_bits = 0xFFFF << shift
                result = (cur & ~mask_bits) | ((imm16 << shift) & mask_bits)
                mnem = "MOVK"
            else:
                return self._unknown(pc, next_pc, raw, s)
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Logical Immediate: AND / ORR / EOR / ANDS ─────────────────────────
        # Encoding: sf[31] | opc[30:29] | 0[28] | 10010[27:23] | N[22] | immr[21:16] | imms[15:10] | Rn[9:5] | Rd[4:0]
        # bits[28:23] = 0b010010 = 18 (bit28=0 fixed, bits27:23=10010 fixed; N is at bit22)
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
                return self._unknown(pc, next_pc, raw, s)
            if not sf:
                imm &= MASK32
            rn_val = self._read_reg(Rn, sf)
            if opc == 0b00:   # AND
                result = rn_val & imm
                mnem = "AND"
            elif opc == 0b01:  # ORR
                result = rn_val | imm
                mnem = "ORR"
            elif opc == 0b10:  # EOR
                result = rn_val ^ imm
                mnem = "EOR"
            else:              # ANDS (sets flags)
                result = rn_val & imm
                new_nzcv = _logical_flags(result, sf)
                mnem = "ANDS"
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Load/Store Unsigned Offset ─────────────────────────────────────────
        # Encoding: size[31:30] | 111[29:27] | V[26] | 01[25:24] | opc[23:22] | imm12[21:10] | Rn[9:5] | Rt[4:0]
        if bits(29, 27) == 0b111 and bits(25, 24) == 0b01 and bits(26, 26) == 0:
            size = bits(31, 30)
            opc = bits(23, 22)
            imm12 = bits(21, 10)
            Rn = bits(9, 5)
            Rt = bits(4, 0)
            # EA = Rn_val + imm12 * (1 << size); Rn=31 here means SP not XZR
            rn_val = new_sp if Rn == 31 else self._read_reg(Rn, 1)
            ea = (rn_val + imm12 * (1 << size)) & MASK64

            if size == 0b00:     # byte (8-bit)
                if opc == 0b00:   # STRB
                    self._mem_write(ea, self._read_reg(Rt, 0) & 0xFF, 1, new_mem)
                    mnem = "STRB"
                elif opc == 0b01:  # LDRB (zero-extend to 32→64)
                    val = self._mem_read(ea, 1, tuple(new_mem))
                    self._write_reg(Rt, val, 0, gpr)
                    mnem = "LDRB"
                elif opc == 0b10:  # LDRSB (sign-extend to 64)
                    val = sext(self._mem_read(ea, 1, tuple(new_mem)), 8)
                    self._write_reg(Rt, val & MASK64, 1, gpr)
                    mnem = "LDRSB"
                elif opc == 0b11:  # LDRSB (sign-extend to 32)
                    val = sext(self._mem_read(ea, 1, tuple(new_mem)), 8)
                    self._write_reg(Rt, val & MASK32, 0, gpr)
                    mnem = "LDRSB32"
                else:
                    return self._unknown(pc, next_pc, raw, s)
            elif size == 0b01:   # halfword (16-bit)
                if opc == 0b00:
                    self._mem_write(ea, self._read_reg(Rt, 0) & 0xFFFF, 2, new_mem)
                    mnem = "STRH"
                elif opc == 0b01:
                    val = self._mem_read(ea, 2, tuple(new_mem))
                    self._write_reg(Rt, val, 0, gpr)
                    mnem = "LDRH"
                elif opc == 0b10:  # LDRSH (sign-extend to 64)
                    val = sext(self._mem_read(ea, 2, tuple(new_mem)), 16)
                    self._write_reg(Rt, val & MASK64, 1, gpr)
                    mnem = "LDRSH"
                elif opc == 0b11:  # LDRSH (sign-extend to 32)
                    val = sext(self._mem_read(ea, 2, tuple(new_mem)), 16)
                    self._write_reg(Rt, val & MASK32, 0, gpr)
                    mnem = "LDRSH32"
                else:
                    return self._unknown(pc, next_pc, raw, s)
            elif size == 0b10:   # word (32-bit)
                if opc == 0b00:
                    self._mem_write(ea, self._read_reg(Rt, 0), 4, new_mem)
                    mnem = "STR32"
                elif opc == 0b01:
                    val = self._mem_read(ea, 4, tuple(new_mem))
                    self._write_reg(Rt, val, 0, gpr)
                    mnem = "LDR32"
                elif opc == 0b10:  # LDRSW (sign-extend to 64)
                    val = sext32(self._mem_read(ea, 4, tuple(new_mem)))
                    self._write_reg(Rt, val & MASK64, 1, gpr)
                    mnem = "LDRSW"
                else:
                    return self._unknown(pc, next_pc, raw, s)
            elif size == 0b11:   # doubleword (64-bit)
                if opc == 0b00:
                    self._mem_write(ea, self._read_reg(Rt, 1), 8, new_mem)
                    mnem = "STR"
                elif opc == 0b01:
                    val = self._mem_read(ea, 8, tuple(new_mem))
                    self._write_reg(Rt, val, 1, gpr)
                    mnem = "LDR"
                else:
                    return self._unknown(pc, next_pc, raw, s)
            else:
                return self._unknown(pc, next_pc, raw, s)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Data Processing Register (shifted): ADD/SUB/AND/ORR/EOR/BIC/etc. ──
        # Two sub-families share a similar pattern:
        #   Logical: sf | opc | 01010 | shift | N | Rm | imm6 | Rn | Rd
        #   Arith:   sf | op  | S | 01011 | shift | 0 | Rm | imm6 | Rn | Rd

        # Logical shifted-register: bits[28:24] == 01010
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
            if opc == 0b00:     # AND / BIC (N inverts Rm)
                result = rn_val & shifted_rm
                mnem = "BIC" if N else "AND"
            elif opc == 0b01:   # ORR / ORN
                result = rn_val | shifted_rm
                mnem = "ORN" if N else "ORR"
            elif opc == 0b10:   # EOR / EON
                result = rn_val ^ shifted_rm
                mnem = "EON" if N else "EOR"
            else:               # ANDS / BICS
                result = rn_val & shifted_rm
                new_nzcv = _logical_flags(result, sf)
                mnem = "BICS" if N else "ANDS"
            mask = MASK64 if sf else MASK32
            result &= mask
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # Arithmetic shifted-register: bits[28:24] == 01011 (bit21 must be 0)
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
            if op == 0:   # ADD
                if S:
                    result, new_nzcv = _add_with_flags(rn_val, shifted_rm, sf)
                    mnem = "ADDS"
                else:
                    mask = MASK64 if sf else MASK32
                    result = (rn_val + shifted_rm) & mask
                    mnem = "ADD"
            else:           # SUB
                if S:
                    result, new_nzcv = _sub_with_flags(rn_val, shifted_rm, sf)
                    mnem = "SUBS"
                else:
                    mask = MASK64 if sf else MASK32
                    result = (rn_val - shifted_rm) & mask
                    mnem = "SUB"
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Data Processing 2-Source: MUL, UDIV, SDIV, shifts-by-register ─────
        # Encoding: sf[31] | 0[30] | S[29] | 11010110[28:21] | Rm[20:16] | opcode[15:10] | Rn[9:5] | Rd[4:0]
        if bits(30, 30) == 0 and bits(28, 21) == 0b11010110:
            Rm = bits(20, 16)
            opcode2 = bits(15, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            rn_val = self._read_reg(Rn, sf)
            rm_val = self._read_reg(Rm, sf)
            bits_width = 64 if sf else 32
            mask = MASK64 if sf else MASK32
            if opcode2 == 0b000010:   # UDIV
                result = (rn_val // rm_val) if rm_val else 0
                mnem = "UDIV"
            elif opcode2 == 0b000011:  # SDIV
                if rm_val == 0:
                    result = 0
                else:
                    # Sign-extend operands to Python signed int
                    a = sext(rn_val, bits_width)
                    b = sext(rm_val, bits_width)
                    # Python // truncates toward -inf; C truncates toward 0
                    result = int(a / b)  # truncate-toward-zero
                mnem = "SDIV"
            elif opcode2 == 0b001000:  # LSLV
                result = _apply_shift(rn_val, 0, rm_val % bits_width, sf)
                mnem = "LSLV"
            elif opcode2 == 0b001001:  # LSRV
                result = _apply_shift(rn_val, 1, rm_val % bits_width, sf)
                mnem = "LSRV"
            elif opcode2 == 0b001010:  # ASRV
                result = _apply_shift(rn_val, 2, rm_val % bits_width, sf)
                mnem = "ASRV"
            elif opcode2 == 0b001011:  # RORV
                result = _apply_shift(rn_val, 3, rm_val % bits_width, sf)
                mnem = "RORV"
            else:
                return self._unknown(pc, next_pc, raw, s)
            result &= mask
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Data Processing 1-Source: CLZ, RBIT, REV, REV16, REV32 ──────────
        # Encoding: sf[31] | 1[30] | S[29] | 11010110[28:21] | 00000[20:16] | opcode[15:10] | Rn[9:5] | Rd[4:0]
        if bits(30, 30) == 1 and bits(28, 21) == 0b11010110 and bits(20, 16) == 0:
            opcode2 = bits(15, 10)
            Rn = bits(9, 5)
            Rd = bits(4, 0)
            rn_val = self._read_reg(Rn, sf)
            bits_width = 64 if sf else 32
            mask = MASK64 if sf else MASK32
            if opcode2 == 0b000000:   # RBIT
                result = _rbit(rn_val, bits_width)
                mnem = "RBIT"
            elif opcode2 == 0b000001:  # REV16
                result = _rev16(rn_val, bits_width)
                mnem = "REV16"
            elif opcode2 == 0b000010:  # REV (also REV32 for sf=1)
                if sf:
                    result = _rev(rn_val, 64)
                    mnem = "REV"
                else:
                    result = _rev(rn_val, 32)
                    mnem = "REV"
            elif opcode2 == 0b000011 and sf == 1:  # REV32 (X only)
                result = _rev32(rn_val)
                mnem = "REV32"
            elif opcode2 == 0b000100:  # CLZ
                result = _clz(rn_val, bits_width)
                mnem = "CLZ"
            else:
                return self._unknown(pc, next_pc, raw, s)
            result &= mask
            self._write_reg(Rd, result, sf, gpr)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── 3-Source: MADD / MSUB (MUL, MNeg aliases) ─────────────────────────
        # Encoding: sf[31] | 0[30] | 0[29] | 11011[28:24] | op54[23:21] | Rm[20:16] | 0[15] | Ra[14:10] | Rn[9:5] | Rd[4:0]
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
                if o0 == 0:   # MADD: Rd = Ra + Rn * Rm
                    product = rn_val * rm_val
                    result = ra_val + product
                    mnem = "MADD"
                else:          # MSUB: Rd = Ra - Rn * Rm
                    product = rn_val * rm_val
                    result = ra_val - product
                    mnem = "MSUB"
                self._write_reg(Rd, result, sf, gpr)
            elif op54 == 0b001 and sf == 1:  # SMULH (64-bit only)
                rn_val = sext(self._read_reg(Rn, 1), 64)
                rm_val = sext(self._read_reg(Rm, 1), 64)
                product = rn_val * rm_val
                result = (product >> 64) & MASK64
                self._write_reg(Rd, result, 1, gpr)
                mnem = "SMULH"
            elif op54 == 0b010 and sf == 1:  # UMULH (64-bit only)
                rn_val = self._read_reg(Rn, 1)
                rm_val = self._read_reg(Rm, 1)
                product = rn_val * rm_val
                result = (product >> 64) & MASK64
                self._write_reg(Rd, result, 1, gpr)
                mnem = "UMULH"
            else:
                return self._unknown(pc, next_pc, raw, s)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── Conditional Select: CSEL / CSINC / CSINV / CSNEG ─────────────────
        # Encoding: sf[31] | op[30] | S[29] | 11010100[28:21] | Rm[20:16] | cond[15:12] | op2[11:10] | Rn[9:5] | Rd[4:0]
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
                if op == 0 and op2 == 0b00:    # CSEL false: Rm
                    result = rm_val
                    mnem_false = "CSEL"
                elif op == 0 and op2 == 0b01:  # CSINC false: Rm+1
                    result = (rm_val + 1) & mask
                    mnem_false = "CSINC"
                elif op == 1 and op2 == 0b00:  # CSINV false: ~Rm
                    result = (~rm_val) & mask
                    mnem_false = "CSINV"
                elif op == 1 and op2 == 0b01:  # CSNEG false: -Rm
                    result = (-rm_val) & mask
                    mnem_false = "CSNEG"
                else:
                    return self._unknown(pc, next_pc, raw, s)
                self._write_reg(Rd, result, sf, gpr)
                self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
                return StepTrace(
                    pc_before=pc, pc_after=next_pc, mnemonic=mnem_false,
                    description=f"{mnem_false} @ 0x{pc:04X}",
                )
            self._write_reg(Rd, result, sf, gpr)
            # Determine mnemonic from op/op2
            if op == 0 and op2 == 0b00:
                mnem = "CSEL"
            elif op == 0 and op2 == 0b01:
                mnem = "CSINC"
            elif op == 1 and op2 == 0b00:
                mnem = "CSINV"
            elif op == 1 and op2 == 0b01:
                mnem = "CSNEG"
            else:
                return self._unknown(pc, next_pc, raw, s)
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic=mnem, description=f"{mnem} @ 0x{pc:04X}")

        # ── SVC (supervisor call) — NOP in simulator ───────────────────────────
        # Encoding: 11010100 000 | imm16 | 00001
        if bits(31, 21) == 0b11010100_000 and bits(4, 0) == 0b00001:
            self._commit(next_pc, gpr, new_sp, new_nzcv, new_mem)
            return StepTrace(pc_before=pc, pc_after=next_pc, mnemonic="SVC", description=f"SVC @ 0x{pc:04X}")

        # ── Unknown opcode ─────────────────────────────────────────────────────
        return self._unknown(pc, next_pc, raw, s)

    # ── Internal commit helper ────────────────────────────────────────────────

    def _commit(
        self,
        new_pc: int,
        new_gpr: list[int],
        new_sp: int,
        new_nzcv: int,
        new_mem: list[int],
        halted: bool = False,
    ) -> None:
        """Atomically commit a new state snapshot."""
        # XZR (index 31) must always be 0 — enforce on commit
        new_gpr[31] = 0
        self._state = AArch64State(
            pc=new_pc & MASK64,
            gpr=tuple(new_gpr),
            sp=new_sp & MASK64,
            nzcv=new_nzcv & 0xF,
            memory=tuple(new_mem),
            halted=halted,
        )

    def _unknown(
        self, pc: int, next_pc: int, raw: int, s: AArch64State
    ) -> StepTrace:
        """Handle an unrecognized opcode: halt and return an ERROR trace."""
        self._state = AArch64State(
            pc=pc,
            gpr=s.gpr,
            sp=s.sp,
            nzcv=s.nzcv,
            memory=s.memory,
            halted=True,
        )
        return StepTrace(
            pc_before=pc,
            pc_after=pc,
            mnemonic=f"ERROR: UNKNOWN(0x{raw:08X})",
            description=f"Unknown opcode 0x{raw:08X} @ 0x{pc:04X}",
        )


# ── Subtraction with flags (SUB/SUBS/CMP/CMN) ────────────────────────────────


def _sub_with_flags(a: int, b: int, sf: int) -> tuple[int, int]:
    """
    Compute A − B with NZCV flags using the borrow-complement carry convention.

    In AArch64 (like all ARM generations), subtraction is defined as:
        result = A + NOT(B) + 1

    This means:
      - C=1 indicates *no* borrow (unsigned A >= B)
      - C=0 indicates borrow (unsigned A < B)

    This is the *complement* of x86 subtract carry convention.

    Examples::
        _sub_with_flags(5, 3, 1) → (2, N=0, Z=0, C=1, V=0) — no borrow
        _sub_with_flags(3, 5, 1) → (very large, N=1, Z=0, C=0, V=0) — borrow
    """
    bits = 64 if sf else 32
    mask = MASK64 if sf else MASK32
    # A - B = A + (~B) + 1
    not_b = (~b) & mask
    unsigned_sum = (a & mask) + not_b + 1
    result = unsigned_sum & mask
    N = (result >> (bits - 1)) & 1
    Z = 1 if result == 0 else 0
    # Carry = 1 if unsigned_sum >= 2^bits (no borrow)
    C = 1 if unsigned_sum > mask else 0
    a_sign = (a >> (bits - 1)) & 1
    b_sign = (not_b >> (bits - 1)) & 1
    r_sign = N
    V = 1 if (a_sign == b_sign) and (r_sign != a_sign) else 0
    return result, (N << 3) | (Z << 2) | (C << 1) | V


# ── Bit-manipulation helpers ──────────────────────────────────────────────────


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
    for i in range(nbytes):
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
