"""
Instruction-level tests for AppleM1Simulator.

Covers:
  1. AArch64 integer base instructions (key cases)
  2. Scalar FP instructions (FMOV, FADD, FSUB, FMUL, FDIV, FABS, FNEG, FSQRT)
  3. FP compare (FCMP → NZCV)
  4. FP conversions (FCVTZS, SCVTF, UCVTF, FCVT)
  5. FP load/store (LDR/STR D and S)
  6. NEON vector integer (ADD, SUB, MUL)
  7. NEON vector FP (FADD, FSUB, FMUL)
  8. DUP from GPR
  9. FMLA (fused multiply-accumulate)
  10. FP-heavy program (distance computation)
"""

from __future__ import annotations

import math
import struct

from conftest import run

from apple_m1_simulator import AppleM1Simulator
from apple_m1_simulator.simulator import (
    COND_EQ,
    COND_LT,
    HALT,
    branch_cond,
    branch_imm,
    branch_reg,
    cbz_cbnz,
    csel_enc,
    dp_imm,
    dp_reg,
    fcvtzs,
    fmov_fp_to_gpr_d,
    fmov_fp_to_gpr_s,
    fmov_gpr_to_fp_d,
    fmov_gpr_to_fp_s,
    fp_cmp,
    fp_dp1src,
    fp_dp2src,
    fp_ldst_uoff,
    ldst_uoff,
    logic_imm,
    logic_reg,
    madd_msub,
    movwide,
    neon_3reg_same,
    neon_dup_gpr,
    scvtf,
    tbz_tbnz,
    ucvtf,
)

# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

def _f64_bits(f: float) -> list[bytes]:
    """Encode a double as 4 MOVW instructions that load it into X0."""
    bits64 = struct.unpack(">Q", struct.pack(">d", f))[0]
    return [
        movwide(1, 0b10, 0, bits64 & 0xFFFF, 0),
        movwide(1, 0b11, 1, (bits64 >> 16) & 0xFFFF, 0),
        movwide(1, 0b11, 2, (bits64 >> 32) & 0xFFFF, 0),
        movwide(1, 0b11, 3, (bits64 >> 48) & 0xFFFF, 0),
        fmov_gpr_to_fp_d(0, 0),   # FMOV D0, X0
    ]


def _f64_into_reg(f: float, gpr_idx: int, vreg_idx: int) -> list[bytes]:
    """Load double `f` into vreg[vreg_idx] using GPR gpr_idx as scratch."""
    bits64 = struct.unpack(">Q", struct.pack(">d", f))[0]
    return [
        movwide(1, 0b10, 0, bits64 & 0xFFFF, gpr_idx),
        movwide(1, 0b11, 1, (bits64 >> 16) & 0xFFFF, gpr_idx),
        movwide(1, 0b11, 2, (bits64 >> 32) & 0xFFFF, gpr_idx),
        movwide(1, 0b11, 3, (bits64 >> 48) & 0xFFFF, gpr_idx),
        fmov_gpr_to_fp_d(gpr_idx, vreg_idx),
    ]


def _f32_into_reg(f: float, gpr_idx: int, vreg_idx: int) -> list[bytes]:
    """Load single `f` into vreg[vreg_idx] (S register) using GPR gpr_idx."""
    bits32 = struct.unpack(">I", struct.pack(">f", f))[0]
    return [
        movwide(0, 0b10, 0, bits32 & 0xFFFF, gpr_idx),
        movwide(0, 0b11, 1, (bits32 >> 16) & 0xFFFF, gpr_idx),
        fmov_gpr_to_fp_s(gpr_idx, vreg_idx),
    ]


# ─────────────────────────────────────────────────────────────────────────────
# AArch64 Integer Base (key coverage)
# ─────────────────────────────────────────────────────────────────────────────


def test_add_immediate() -> None:
    state = run([dp_imm(1, 0, 0, 10, 0, 31, 0)])   # ADD X0, XZR, #10
    assert state.x0 == 10


def test_sub_immediate() -> None:
    state = run([
        dp_imm(1, 0, 0, 20, 0, 31, 0),   # ADD X0, XZR, #20
        dp_imm(1, 1, 0, 7, 0, 0, 0),      # SUB X0, X0, #7
    ])
    assert state.x0 == 13


def test_movz() -> None:
    state = run([movwide(1, 0b10, 0, 0xABCD, 0)])   # MOVZ X0, #0xABCD
    assert state.x0 == 0xABCD


def test_movn() -> None:
    state = run([movwide(1, 0b00, 0, 0, 0)])   # MOVN X0, #0  → X0 = -1
    assert state.x0 == 0xFFFF_FFFF_FFFF_FFFF


def test_movk() -> None:
    state = run([
        movwide(1, 0b10, 0, 0x1234, 0),   # MOVZ X0, #0x1234
        movwide(1, 0b11, 1, 0x5678, 0),   # MOVK X0, #0x5678, lsl 16
    ])
    assert state.x0 == 0x5678_1234


def test_and_reg() -> None:
    state = run([
        movwide(1, 0b10, 0, 0xFF, 0),               # MOVZ X0, #0xFF
        movwide(1, 0b10, 0, 0x0F, 1),               # MOVZ X1, #0x0F
        logic_reg(1, 0b00, 0, 0, 1, 0, 0, 2),       # AND X2, X0, X1
    ])
    assert state.x2 == 0x0F


def test_orr_reg() -> None:
    state = run([
        movwide(1, 0b10, 0, 0xF0, 0),
        movwide(1, 0b10, 0, 0x0F, 1),
        logic_reg(1, 0b01, 0, 0, 1, 0, 0, 2),       # ORR X2, X0, X1
    ])
    assert state.x2 == 0xFF


def test_mul_via_madd() -> None:
    state = run([
        dp_imm(1, 0, 0, 6, 0, 31, 0),              # ADD X0, XZR, #6
        dp_imm(1, 0, 0, 7, 0, 31, 1),              # ADD X1, XZR, #7
        madd_msub(1, 0b000, 1, 0, 31, 0, 2),       # MUL X2, X0, X1 (MADD X2, X0, X1, XZR)
    ])
    assert state.x2 == 42


def test_ldr_str_64bit() -> None:
    state = run([
        dp_imm(1, 0, 0, 0x100, 0, 31, 1),          # ADD X1, XZR, #0x100  (base addr)
        movwide(1, 0b10, 0, 0xBEEF, 0),            # MOVZ X0, #0xBEEF
        ldst_uoff(3, 0, 0b00, 0, 1, 0),            # STR X0, [X1]
        ldst_uoff(3, 0, 0b01, 0, 1, 2),            # LDR X2, [X1]
    ])
    assert state.x2 == 0xBEEF


def test_branch_unconditional() -> None:
    state = run([
        branch_imm(0, 2),                           # B #+8 (skip next 1 instruction)
        dp_imm(1, 0, 0, 1, 0, 31, 0),              # ADD X0, XZR, #1  (skipped)
        dp_imm(1, 0, 0, 2, 0, 31, 0),              # ADD X0, XZR, #2
    ])
    assert state.x0 == 2


def test_branch_conditional_eq() -> None:
    state = run([
        dp_imm(1, 0, 0, 5, 0, 31, 0),              # ADD X0, XZR, #5
        dp_imm(1, 1, 1, 5, 0, 0, 31),              # SUBS XZR, X0, #5  → Z=1
        branch_cond(2, COND_EQ),                    # B.EQ #+8 (skip next)
        dp_imm(1, 0, 0, 99, 0, 31, 1),             # ADD X1, XZR, #99 (skipped)
        dp_imm(1, 0, 0, 42, 0, 31, 1),             # ADD X1, XZR, #42
    ])
    assert state.x1 == 42


def test_cbz() -> None:
    state = run([
        movwide(1, 0b10, 0, 0, 0),                  # MOVZ X0, #0
        cbz_cbnz(1, 0, 2, 0),                       # CBZ X0, #+8 (skip)
        dp_imm(1, 0, 0, 1, 0, 31, 1),              # skipped
        dp_imm(1, 0, 0, 2, 0, 31, 1),              # ADD X1, XZR, #2
    ])
    assert state.x1 == 2


def test_ret() -> None:
    # BL to subroutine, subroutine does RET, check X2 was set
    # Subroutine is at offset +8, sets X2=99, then RET
    state = run([
        branch_imm(1, 3),                           # BL #+12 (to sub @ offset 12)
        dp_imm(1, 0, 0, 1, 0, 31, 3),              # ADD X3, XZR, #1 (after return)
        HALT,                                        # pad
        dp_imm(1, 0, 0, 99, 0, 31, 2),             # sub: ADD X2, XZR, #99
        branch_reg(0b010, 30),                       # RET (X30=return address)
    ])
    assert state.x2 == 99
    assert state.x3 == 1


def test_csel() -> None:
    state = run([
        dp_imm(1, 0, 0, 10, 0, 31, 0),             # ADD X0, XZR, #10
        dp_imm(1, 0, 0, 20, 0, 31, 1),             # ADD X1, XZR, #20
        dp_imm(1, 1, 1, 10, 0, 0, 31),             # SUBS XZR, X0, #10  → Z=1
        csel_enc(1, 0, 0, 1, COND_EQ, 0b00, 0, 2), # CSEL X2, X0, X1, EQ → X2=X0=10
    ])
    assert state.x2 == 10


def test_tbz() -> None:
    state = run([
        movwide(1, 0b10, 0, 0b0110, 0),            # MOVZ X0, #6 (bits 1 and 2 set)
        tbz_tbnz(0, 0, 0, 2, 0),                   # TBZ X0, #0, #+8 (bit0=0, branch)
        dp_imm(1, 0, 0, 1, 0, 31, 1),              # skipped
        dp_imm(1, 0, 0, 2, 0, 31, 1),              # ADD X1, XZR, #2
    ])
    assert state.x1 == 2


def test_ldrb_strb() -> None:
    state = run([
        dp_imm(1, 0, 0, 0x200, 0, 31, 1),          # ADD X1, XZR, #0x200
        movwide(1, 0b10, 0, 0xAB, 0),              # MOVZ X0, #0xAB
        ldst_uoff(0, 0, 0b00, 0, 1, 0),            # STRB W0, [X1]
        ldst_uoff(0, 0, 0b01, 0, 1, 2),            # LDRB W2, [X1]
    ])
    assert state.x2 == 0xAB


def test_clz() -> None:
    # CLZ is dp-1src: sf=1, bit30=1, bits28:21=11010110, bits20:16=0, opcode=000100
    # Use raw encoding since no dedicated helper
    # MOVZ X0, #1 then CLZ X1, X0
    state = run([
        movwide(1, 0b10, 0, 1, 0),                  # X0 = 1
        # CLZ X1, X0: sf=1, 1_11010110_00000_000100_Rn_Rd
        # = 0b1_1_0_11010110_00000_000100_00000_00001
        # sf=1, bit30=1(1-src), S=0, bits28:21=11010110, bits20:16=00000, opcode=000100=4, Rn=0, Rd=1
        # raw = 1<<31 | 1<<30 | 0<<29 | 0b11010110<<21 | 0<<16 | 4<<10 | 0<<5 | 1
        # = 0x80000000 | 0x40000000 | 0x1AC00000 | 0x1000 | 1
    ] + [
        struct.pack(">I", (1 << 31) | (1 << 30) | (0b11010110 << 21) | (0 << 16) | (0b000100 << 10) | (0 << 5) | 1),
    ])
    assert state.x1 == 63   # CLZ(1) = 63 in 64-bit


# ─────────────────────────────────────────────────────────────────────────────
# Scalar FP: FMOV
# ─────────────────────────────────────────────────────────────────────────────


def test_fmov_gpr_to_fp_double() -> None:
    """FMOV D0, X0 copies 64-bit GPR bits into vreg[0] lower 64 bits."""
    bits64 = struct.unpack(">Q", struct.pack(">d", 1.5))[0]
    state = run(
        [
            movwide(1, 0b10, 0, bits64 & 0xFFFF, 0),
            movwide(1, 0b11, 1, (bits64 >> 16) & 0xFFFF, 0),
            movwide(1, 0b11, 2, (bits64 >> 32) & 0xFFFF, 0),
            movwide(1, 0b11, 3, (bits64 >> 48) & 0xFFFF, 0),
            fmov_gpr_to_fp_d(0, 0),               # FMOV D0, X0
        ]
    )
    assert abs(state.d0 - 1.5) < 1e-12


def test_fmov_fp_to_gpr_double() -> None:
    """FMOV X0, D0 reads the lower 64 bits of vreg[0] into GPR X0."""
    bits64 = struct.unpack(">Q", struct.pack(">d", -2.0))[0]
    state = run(
        [
            movwide(1, 0b10, 0, bits64 & 0xFFFF, 0),
            movwide(1, 0b11, 1, (bits64 >> 16) & 0xFFFF, 0),
            movwide(1, 0b11, 2, (bits64 >> 32) & 0xFFFF, 0),
            movwide(1, 0b11, 3, (bits64 >> 48) & 0xFFFF, 0),
            fmov_gpr_to_fp_d(0, 0),               # FMOV D0, X0
            fmov_fp_to_gpr_d(0, 1),               # FMOV X1, D0
        ]
    )
    assert state.x1 == bits64


def test_fmov_gpr_to_fp_single() -> None:
    """FMOV S0, W0 copies 32-bit GPR bits into vreg[0] lower 32 bits."""
    bits32 = struct.unpack(">I", struct.pack(">f", 3.14))[0]
    state = run(
        [
            movwide(0, 0b10, 0, bits32 & 0xFFFF, 0),
            movwide(0, 0b11, 1, (bits32 >> 16) & 0xFFFF, 0),
            fmov_gpr_to_fp_s(0, 0),               # FMOV S0, W0
        ]
    )
    assert abs(state.s0 - 3.14) < 1e-5


def test_fmov_fp_to_gpr_single() -> None:
    """FMOV W0, S0 reads the lower 32 bits of vreg[0] into GPR W0."""
    bits32 = struct.unpack(">I", struct.pack(">f", 7.0))[0]
    state = run(
        [
            movwide(0, 0b10, 0, bits32 & 0xFFFF, 0),
            movwide(0, 0b11, 1, (bits32 >> 16) & 0xFFFF, 0),
            fmov_gpr_to_fp_s(0, 0),
            fmov_fp_to_gpr_s(0, 1),               # FMOV W1, S0
        ]
    )
    assert state.w1 == bits32


def test_fmov_fp_reg_to_fp_reg_double() -> None:
    """FMOV D1, D0 copies vreg[0] lower 64 bits to vreg[1]."""
    state = run(
        _f64_into_reg(2.718, 0, 0) + [
            fp_dp1src(0b01, 0b000000, 0, 1),       # FMOV D1, D0
        ]
    )
    assert abs(state.d1 - 2.718) < 1e-10


# ─────────────────────────────────────────────────────────────────────────────
# Scalar FP: Arithmetic
# ─────────────────────────────────────────────────────────────────────────────


def test_fadd_double() -> None:
    """FADD D2, D0, D1 adds two doubles."""
    state = run(
        _f64_into_reg(1.5, 0, 0) +
        _f64_into_reg(2.5, 1, 1) + [
            fp_dp2src(0b01, 1, 0b0010, 0, 2),     # FADD D2, D0, D1
        ]
    )
    assert abs(state.d2 - 4.0) < 1e-12


def test_fsub_double() -> None:
    """FSUB D2, D0, D1 subtracts two doubles."""
    state = run(
        _f64_into_reg(10.0, 0, 0) +
        _f64_into_reg(3.0, 1, 1) + [
            fp_dp2src(0b01, 1, 0b0011, 0, 2),     # FSUB D2, D0, D1
        ]
    )
    assert abs(state.d2 - 7.0) < 1e-12


def test_fmul_double() -> None:
    """FMUL D2, D0, D1 multiplies two doubles."""
    state = run(
        _f64_into_reg(3.0, 0, 0) +
        _f64_into_reg(4.0, 1, 1) + [
            fp_dp2src(0b01, 1, 0b0000, 0, 2),     # FMUL D2, D0, D1
        ]
    )
    assert abs(state.d2 - 12.0) < 1e-12


def test_fdiv_double() -> None:
    """FDIV D2, D0, D1 divides two doubles."""
    state = run(
        _f64_into_reg(9.0, 0, 0) +
        _f64_into_reg(2.0, 1, 1) + [
            fp_dp2src(0b01, 1, 0b0001, 0, 2),     # FDIV D2, D0, D1
        ]
    )
    assert abs(state.d2 - 4.5) < 1e-12


def test_fadd_single() -> None:
    """FADD S2, S0, S1 adds two singles."""
    state = run(
        _f32_into_reg(1.5, 0, 0) +
        _f32_into_reg(2.5, 1, 1) + [
            fp_dp2src(0b00, 1, 0b0010, 0, 2),     # FADD S2, S0, S1
        ]
    )
    assert abs(state.s2 - 4.0) < 1e-6


def test_fsub_single() -> None:
    """FSUB S2, S0, S1 subtracts two singles."""
    state = run(
        _f32_into_reg(7.0, 0, 0) +
        _f32_into_reg(3.0, 1, 1) + [
            fp_dp2src(0b00, 1, 0b0011, 0, 2),     # FSUB S2, S0, S1
        ]
    )
    assert abs(state.s2 - 4.0) < 1e-6


def test_fmul_single() -> None:
    """FMUL S2, S0, S1 multiplies two singles."""
    state = run(
        _f32_into_reg(2.5, 0, 0) +
        _f32_into_reg(4.0, 1, 1) + [
            fp_dp2src(0b00, 1, 0b0000, 0, 2),     # FMUL S2, S0, S1
        ]
    )
    assert abs(state.s2 - 10.0) < 1e-5


def test_fdiv_single() -> None:
    """FDIV S2, S0, S1 divides two singles."""
    state = run(
        _f32_into_reg(8.0, 0, 0) +
        _f32_into_reg(4.0, 1, 1) + [
            fp_dp2src(0b00, 1, 0b0001, 0, 2),     # FDIV S2, S0, S1
        ]
    )
    assert abs(state.s2 - 2.0) < 1e-6


def test_fabs_double() -> None:
    """FABS D1, D0 takes absolute value."""
    state = run(
        _f64_into_reg(-5.0, 0, 0) + [
            fp_dp1src(0b01, 0b000001, 0, 1),       # FABS D1, D0
        ]
    )
    assert state.d1 == 5.0


def test_fneg_double() -> None:
    """FNEG D1, D0 negates a double."""
    state = run(
        _f64_into_reg(3.14, 0, 0) + [
            fp_dp1src(0b01, 0b000010, 0, 1),       # FNEG D1, D0
        ]
    )
    assert abs(state.d1 - (-3.14)) < 1e-10


def test_fsqrt_double() -> None:
    """FSQRT D1, D0 computes square root."""
    state = run(
        _f64_into_reg(9.0, 0, 0) + [
            fp_dp1src(0b01, 0b000011, 0, 1),       # FSQRT D1, D0
        ]
    )
    assert abs(state.d1 - 3.0) < 1e-12


def test_fsqrt_single() -> None:
    """FSQRT S1, S0 computes square root for single precision."""
    state = run(
        _f32_into_reg(16.0, 0, 0) + [
            fp_dp1src(0b00, 0b000011, 0, 1),       # FSQRT S1, S0
        ]
    )
    assert abs(state.s1 - 4.0) < 1e-6


# ─────────────────────────────────────────────────────────────────────────────
# FCMP: FP compare → NZCV
# ─────────────────────────────────────────────────────────────────────────────


def test_fcmp_equal_double() -> None:
    """FCMP Dn, Dm sets NZCV=0b0110 (Z=1,C=1) when equal."""
    state = run(
        _f64_into_reg(2.0, 0, 0) +
        _f64_into_reg(2.0, 1, 1) + [
            fp_cmp(0b01, 1, 0, opc=0),            # FCMP D0, D1
        ]
    )
    assert state.nzcv == 0b0110
    assert state.z is True
    assert state.c is True


def test_fcmp_less_double() -> None:
    """FCMP Dn, Dm sets NZCV=0b1000 (N=1) when Dn < Dm."""
    state = run(
        _f64_into_reg(1.0, 0, 0) +
        _f64_into_reg(2.0, 1, 1) + [
            fp_cmp(0b01, 1, 0, opc=0),            # FCMP D0, D1
        ]
    )
    assert state.nzcv == 0b1000
    assert state.n is True


def test_fcmp_greater_double() -> None:
    """FCMP Dn, Dm sets NZCV=0b0010 (C=1) when Dn > Dm."""
    state = run(
        _f64_into_reg(3.0, 0, 0) +
        _f64_into_reg(1.0, 1, 1) + [
            fp_cmp(0b01, 1, 0, opc=0),            # FCMP D0, D1
        ]
    )
    assert state.nzcv == 0b0010


def test_fcmp_zero_double() -> None:
    """FCMP Dn, #0.0 compares against positive zero."""
    state = run(
        _f64_into_reg(1.0, 0, 0) + [
            fp_cmp(0b01, 0, 0, opc=0b011),        # FCMP D0, #0.0
        ]
    )
    # 1.0 > 0.0, so C=1
    assert state.nzcv == 0b0010


def test_fcmp_enables_b_eq() -> None:
    """After FCMP, B.EQ branches correctly on equal doubles."""
    state = run(
        _f64_into_reg(5.0, 0, 0) +
        _f64_into_reg(5.0, 1, 1) + [
            fp_cmp(0b01, 1, 0, opc=0),
            branch_cond(2, COND_EQ),               # B.EQ #+8 (skip next)
            dp_imm(1, 0, 0, 99, 0, 31, 2),        # skipped
            dp_imm(1, 0, 0, 42, 0, 31, 2),        # ADD X2, XZR, #42
        ]
    )
    assert state.x2 == 42


def test_fcmp_enables_b_lt() -> None:
    """B.LT branches after FCMP on less-than."""
    state = run(
        _f64_into_reg(1.0, 0, 0) +
        _f64_into_reg(2.0, 1, 1) + [
            fp_cmp(0b01, 1, 0, opc=0),
            branch_cond(2, COND_LT),               # B.LT #+8
            dp_imm(1, 0, 0, 99, 0, 31, 2),        # skipped
            dp_imm(1, 0, 0, 7, 0, 31, 2),
        ]
    )
    assert state.x2 == 7


def test_fcmp_single() -> None:
    """FCMP Sn, Sm works for single precision."""
    state = run(
        _f32_into_reg(1.0, 0, 0) +
        _f32_into_reg(1.0, 1, 1) + [
            fp_cmp(0b00, 1, 0, opc=0),            # FCMP S0, S1
        ]
    )
    assert state.nzcv == 0b0110   # equal


# ─────────────────────────────────────────────────────────────────────────────
# FP Conversions
# ─────────────────────────────────────────────────────────────────────────────


def test_fcvtzs_double_to_int64() -> None:
    """FCVTZS Xd, Dn converts double to int64 truncating toward zero."""
    state = run(
        _f64_into_reg(3.99, 0, 0) + [
            fcvtzs(1, 0b01, 0, 1),                 # FCVTZS X1, D0
        ]
    )
    assert state.x1 == 3


def test_fcvtzs_negative() -> None:
    """FCVTZS truncates toward zero (not floor) for negative values."""
    state = run(
        _f64_into_reg(-3.99, 0, 0) + [
            fcvtzs(1, 0b01, 0, 1),                 # FCVTZS X1, D0
        ]
    )
    assert state.x1 == 0xFFFF_FFFF_FFFF_FFFD   # -3 in two's complement


def test_fcvtzs_single_to_int32() -> None:
    """FCVTZS Wd, Sn converts single to int32."""
    state = run(
        _f32_into_reg(7.9, 0, 0) + [
            fcvtzs(0, 0b00, 0, 1),                 # FCVTZS W1, S0
        ]
    )
    assert state.w1 == 7


def test_scvtf_int64_to_double() -> None:
    """SCVTF Dd, Xn converts signed int64 to double."""
    state = run([
        movwide(1, 0b10, 0, 100, 0),               # MOVZ X0, #100
        scvtf(1, 0b01, 0, 1),                       # SCVTF D1, X0
    ])
    assert abs(state.d1 - 100.0) < 1e-12


def test_scvtf_negative_int() -> None:
    """SCVTF correctly converts negative signed integer."""
    state = run([
        movwide(1, 0b00, 0, 0, 0),                  # MOVN X0, #0 → X0 = -1
        scvtf(1, 0b01, 0, 1),                        # SCVTF D1, X0
    ])
    assert state.d1 == -1.0


def test_scvtf_int32_to_single() -> None:
    """SCVTF Sd, Wn converts signed int32 to single."""
    state = run([
        movwide(0, 0b10, 0, 42, 0),                 # MOVZ W0, #42
        scvtf(0, 0b00, 0, 1),                        # SCVTF S1, W0
    ])
    assert abs(state.s1 - 42.0) < 1e-6


def test_ucvtf_uint64_to_double() -> None:
    """UCVTF Dd, Xn converts unsigned int64 to double."""
    state = run([
        movwide(1, 0b10, 0, 255, 0),                # MOVZ X0, #255
        ucvtf(1, 0b01, 0, 1),                        # UCVTF D1, X0
    ])
    assert abs(state.d1 - 255.0) < 1e-12


def test_ucvtf_uint32_to_single() -> None:
    """UCVTF Sd, Wn converts unsigned int32 to single."""
    state = run([
        movwide(0, 0b10, 0, 1000, 0),               # MOVZ W0, #1000
        ucvtf(0, 0b00, 0, 1),                        # UCVTF S1, W0
    ])
    assert abs(state.s1 - 1000.0) < 1.0


def test_fcvt_single_to_double() -> None:
    """FCVT Dd, Sn widens single precision to double."""
    state = run(
        _f32_into_reg(1.5, 0, 0) + [
            fp_dp1src(0b00, 0b000100, 0, 1),        # FCVT Dd, Sn (ftype=00→single, output=double)
        ]
    )
    assert abs(state.d1 - 1.5) < 1e-12


def test_fcvt_double_to_single() -> None:
    """FCVT Sd, Dn narrows double precision to single."""
    state = run(
        _f64_into_reg(3.14, 0, 0) + [
            fp_dp1src(0b01, 0b000100, 0, 1),        # FCVT Sd, Dn (ftype=01→double, output=single)
        ]
    )
    assert abs(state.s1 - 3.14) < 1e-5


# ─────────────────────────────────────────────────────────────────────────────
# FP Load / Store
# ─────────────────────────────────────────────────────────────────────────────


def test_str_ldr_double_fp() -> None:
    """STR Dt, [Xn] and LDR Dt, [Xn] round-trip a double through memory."""
    state = run(
        _f64_into_reg(3.14, 0, 0) + [
            dp_imm(1, 0, 0, 0x100, 0, 31, 1),      # ADD X1, XZR, #256 (base addr)
            fp_ldst_uoff(0b11, 0b00, 0, 1, 0),     # STR D0, [X1]
            fp_ldst_uoff(0b11, 0b01, 0, 1, 2),     # LDR D2, [X1]
        ]
    )
    assert abs(state.d2 - 3.14) < 1e-10


def test_str_ldr_single_fp() -> None:
    """STR St, [Xn] and LDR St, [Xn] round-trip a single through memory."""
    state = run(
        _f32_into_reg(2.5, 0, 0) + [
            dp_imm(1, 0, 0, 0x200, 0, 31, 1),      # ADD X1, XZR, #512
            fp_ldst_uoff(0b10, 0b00, 0, 1, 0),     # STR S0, [X1]
            fp_ldst_uoff(0b10, 0b01, 0, 1, 2),     # LDR S2, [X1]
        ]
    )
    assert abs(state.s2 - 2.5) < 1e-6


def test_fp_ldst_with_offset() -> None:
    """LDR/STR with imm12 offset works correctly."""
    state = run(
        _f64_into_reg(1.0, 0, 0) + [
            dp_imm(1, 0, 0, 0x100, 0, 31, 1),      # X1 = 256
            fp_ldst_uoff(0b11, 0b00, 1, 1, 0),     # STR D0, [X1, #8]  (offset=1*8=8)
            fp_ldst_uoff(0b11, 0b01, 1, 1, 3),     # LDR D3, [X1, #8]
        ]
    )
    assert abs(state.d3 - 1.0) < 1e-12


# ─────────────────────────────────────────────────────────────────────────────
# NEON Vector Integer
# ─────────────────────────────────────────────────────────────────────────────


def test_neon_add_2d() -> None:
    """ADD Vd.2D, Vn.2D, Vm.2D adds two 64-bit element vectors."""
    # Write V0 and V1 via DUP from GPR (just test that add works)
    # Simpler: MOVZ X0, #1, DUP V0.2D, X0 then same for V1 with 3
    state = run([
        movwide(1, 0b10, 0, 1, 0),                  # MOVZ X0, #1
        neon_dup_gpr(1, 0b10000, 0, 0),             # DUP V0.2D, X0 → [1, 1]
        movwide(1, 0b10, 0, 3, 1),                  # MOVZ X1, #3
        neon_dup_gpr(1, 0b10000, 1, 1),             # DUP V1.2D, X1 → [3, 3]
        neon_3reg_same(1, 0, 0b11, 1, 0b10000, 0, 2),  # ADD V2.2D, V0.2D, V1.2D
    ])
    # Both lanes should be 1+3=4
    lo = state.vreg[2] & 0xFFFF_FFFF_FFFF_FFFF
    hi = (state.vreg[2] >> 64) & 0xFFFF_FFFF_FFFF_FFFF
    assert lo == 4
    assert hi == 4


def test_neon_sub_2d() -> None:
    """SUB Vd.2D, Vn.2D, Vm.2D subtracts vector elements (U=1, opcode=10000)."""
    state = run([
        movwide(1, 0b10, 0, 10, 0),
        neon_dup_gpr(1, 0b10000, 0, 0),             # DUP V0.2D, X0 → [10, 10]
        movwide(1, 0b10, 0, 3, 1),
        neon_dup_gpr(1, 0b10000, 1, 1),             # DUP V1.2D, X1 → [3, 3]
        neon_3reg_same(1, 1, 0b11, 1, 0b10000, 0, 2),  # SUB V2.2D, V0.2D, V1.2D
    ])
    lo = state.vreg[2] & 0xFFFF_FFFF_FFFF_FFFF
    hi = (state.vreg[2] >> 64) & 0xFFFF_FFFF_FFFF_FFFF
    assert lo == 7
    assert hi == 7


def test_neon_add_4s() -> None:
    """ADD Vd.4S, Vn.4S, Vm.4S adds four 32-bit element lanes."""
    # DUP into 32-bit lanes (imm5=01000 for S, Q=1 for 4S)
    state = run([
        movwide(1, 0b10, 0, 5, 0),
        neon_dup_gpr(1, 0b01000, 0, 0),             # DUP V0.4S, W0 → [5,5,5,5]
        movwide(1, 0b10, 0, 3, 1),
        neon_dup_gpr(1, 0b01000, 1, 1),             # DUP V1.4S, W1 → [3,3,3,3]
        neon_3reg_same(1, 0, 0b10, 1, 0b10000, 0, 2),  # ADD V2.4S, V0.4S, V1.4S
    ])
    # Each 32-bit lane should be 8
    v2 = state.vreg[2]
    for lane in range(4):
        lane_val = (v2 >> (lane * 32)) & 0xFFFFFFFF
        assert lane_val == 8, f"Lane {lane} = {lane_val}"


def test_neon_mul_4s() -> None:
    """MUL Vd.4S, Vn.4S, Vm.4S multiplies four 32-bit lanes."""
    state = run([
        movwide(1, 0b10, 0, 3, 0),
        neon_dup_gpr(1, 0b01000, 0, 0),             # DUP V0.4S, W0 → [3,3,3,3]
        movwide(1, 0b10, 0, 4, 1),
        neon_dup_gpr(1, 0b01000, 1, 1),             # DUP V1.4S, W1 → [4,4,4,4]
        neon_3reg_same(1, 0, 0b10, 1, 0b10011, 0, 2),  # MUL V2.4S, V0.4S, V1.4S
    ])
    v2 = state.vreg[2]
    for lane in range(4):
        lane_val = (v2 >> (lane * 32)) & 0xFFFFFFFF
        assert lane_val == 12, f"Lane {lane} = {lane_val}"


# ─────────────────────────────────────────────────────────────────────────────
# NEON Vector FP
# ─────────────────────────────────────────────────────────────────────────────


def test_neon_fadd_2d() -> None:
    """FADD Vd.2D, Vn.2D, Vm.2D adds two 2×double vectors."""
    # DUP V0.2D with 1.5, DUP V1.2D with 0.5 using raw bit-patterns via DUP from GPR
    state = run(
        _f64_into_reg(1.5, 0, 0) + [
            neon_dup_gpr(1, 0b10000, 0, 0),         # DUP V0.2D, X0 (using X0's bits as int)
        ] +
        _f64_into_reg(0.5, 1, 1) + [
            neon_dup_gpr(1, 0b10000, 1, 1),
        ] + [
            # FADD V2.2D, V0.2D, V1.2D
            # neon_3reg_same with Q=1, U=0, size=xx, opcode=11010
            # For FP: bit[23]=0, sz(bit[22])=1 (double)
            neon_3reg_same(1, 0, 0b01, 1, 0b11010, 0, 2),
        ]
    )
    lo_bits = state.vreg[2] & 0xFFFF_FFFF_FFFF_FFFF
    hi_bits = (state.vreg[2] >> 64) & 0xFFFF_FFFF_FFFF_FFFF
    lo_f = struct.unpack(">d", struct.pack(">Q", lo_bits))[0]
    hi_f = struct.unpack(">d", struct.pack(">Q", hi_bits))[0]
    # DUP duplicates X0's integer representation — we need to load the doubles properly
    # Actually DUP from GPR uses the raw integer, not the float value.
    # The test uses _f64_into_reg which sets D register, then DUP from GPR uses X0's int bits.
    # After _f64_into_reg(1.5, 0, 0) X0 has bits of 1.5 as integer.
    # Then neon_dup_gpr loads from X0 (the integer == float bits) into all lanes.
    # So each lane has the raw bits of 1.5. When decoded as f64: both lanes = 1.5.
    assert abs(lo_f - 2.0) < 1e-10
    assert abs(hi_f - 2.0) < 1e-10


def test_neon_fadd_4s() -> None:
    """FADD Vd.4S, Vn.4S, Vm.4S adds four single-precision floats."""
    bits32_3 = struct.unpack(">I", struct.pack(">f", 3.0))[0]
    bits32_1 = struct.unpack(">I", struct.pack(">f", 1.0))[0]
    state = run([
        # Load 3.0 into W0 as int, DUP V0.4S
        movwide(0, 0b10, 0, bits32_3 & 0xFFFF, 0),
        movwide(0, 0b11, 1, (bits32_3 >> 16) & 0xFFFF, 0),
        neon_dup_gpr(1, 0b01000, 0, 0),             # DUP V0.4S
        # Load 1.0 into W1, DUP V1.4S
        movwide(0, 0b10, 0, bits32_1 & 0xFFFF, 1),
        movwide(0, 0b11, 1, (bits32_1 >> 16) & 0xFFFF, 1),
        neon_dup_gpr(1, 0b01000, 1, 1),             # DUP V1.4S
        # FADD V2.4S: Q=1, U=0, bit23=0, sz(bit22)=0 (f32), opcode=11010
        neon_3reg_same(1, 0, 0b00, 1, 0b11010, 0, 2),
    ])
    v2 = state.vreg[2]
    for lane in range(4):
        bits_lane = (v2 >> (lane * 32)) & 0xFFFFFFFF
        f_lane = struct.unpack(">f", struct.pack(">I", bits_lane))[0]
        assert abs(f_lane - 4.0) < 1e-5, f"Lane {lane} = {f_lane}"


# ─────────────────────────────────────────────────────────────────────────────
# DUP from GPR
# ─────────────────────────────────────────────────────────────────────────────


def test_dup_2d_from_gpr() -> None:
    """DUP Vd.2D, Xn broadcasts X register to both 64-bit lanes."""
    state = run([
        movwide(1, 0b10, 0, 0x1234, 0),
        neon_dup_gpr(1, 0b10000, 0, 0),
    ])
    lo = state.vreg[0] & 0xFFFF_FFFF_FFFF_FFFF
    hi = (state.vreg[0] >> 64) & 0xFFFF_FFFF_FFFF_FFFF
    assert lo == 0x1234
    assert hi == 0x1234


def test_dup_4s_from_gpr() -> None:
    """DUP Vd.4S, Wn broadcasts W register to all four 32-bit lanes."""
    state = run([
        movwide(1, 0b10, 0, 0xABCD, 0),
        neon_dup_gpr(1, 0b01000, 0, 0),
    ])
    v = state.vreg[0]
    for lane in range(4):
        lane_val = (v >> (lane * 32)) & 0xFFFFFFFF
        assert lane_val == 0xABCD


# ─────────────────────────────────────────────────────────────────────────────
# FMLA (Fused Multiply-Accumulate)
# ─────────────────────────────────────────────────────────────────────────────


def test_fmla_2d() -> None:
    """FMLA Vd.2D, Vn.2D, Vm.2D: Vd = Vd + Vn × Vm."""
    # V0 = [2.0, 2.0] (accum), V1 = [3.0, 3.0], V2 = [4.0, 4.0]
    # Expected: V0 = [2+3*4, 2+3*4] = [14, 14]
    bits_2 = struct.unpack(">Q", struct.pack(">d", 2.0))[0]
    bits_3 = struct.unpack(">Q", struct.pack(">d", 3.0))[0]
    bits_4 = struct.unpack(">Q", struct.pack(">d", 4.0))[0]
    state = run([
        movwide(1, 0b10, 0, bits_2 & 0xFFFF, 0),
        movwide(1, 0b11, 1, (bits_2 >> 16) & 0xFFFF, 0),
        movwide(1, 0b11, 2, (bits_2 >> 32) & 0xFFFF, 0),
        movwide(1, 0b11, 3, (bits_2 >> 48) & 0xFFFF, 0),
        neon_dup_gpr(1, 0b10000, 0, 0),             # DUP V0.2D, X0 → [2.0, 2.0]
        movwide(1, 0b10, 0, bits_3 & 0xFFFF, 1),
        movwide(1, 0b11, 1, (bits_3 >> 16) & 0xFFFF, 1),
        movwide(1, 0b11, 2, (bits_3 >> 32) & 0xFFFF, 1),
        movwide(1, 0b11, 3, (bits_3 >> 48) & 0xFFFF, 1),
        neon_dup_gpr(1, 0b10000, 1, 1),             # DUP V1.2D, X1 → [3.0, 3.0]
        movwide(1, 0b10, 0, bits_4 & 0xFFFF, 2),
        movwide(1, 0b11, 1, (bits_4 >> 16) & 0xFFFF, 2),
        movwide(1, 0b11, 2, (bits_4 >> 32) & 0xFFFF, 2),
        movwide(1, 0b11, 3, (bits_4 >> 48) & 0xFFFF, 2),
        neon_dup_gpr(1, 0b10000, 2, 2),             # DUP V2.2D, X2 → [4.0, 4.0]
        # FMLA V0.2D, V1.2D, V2.2D
        # opcode=11001, Q=1, U=0, bit23=0, sz=1
        neon_3reg_same(1, 0, 0b01, 2, 0b11001, 1, 0),
    ])
    lo_bits = state.vreg[0] & 0xFFFF_FFFF_FFFF_FFFF
    hi_bits = (state.vreg[0] >> 64) & 0xFFFF_FFFF_FFFF_FFFF
    lo_f = struct.unpack(">d", struct.pack(">Q", lo_bits))[0]
    hi_f = struct.unpack(">d", struct.pack(">Q", hi_bits))[0]
    assert abs(lo_f - 14.0) < 1e-10
    assert abs(hi_f - 14.0) < 1e-10


# ─────────────────────────────────────────────────────────────────────────────
# FP-heavy program: Euclidean distance sqrt(dx*dx + dy*dy)
# ─────────────────────────────────────────────────────────────────────────────


def test_fp_euclidean_distance() -> None:
    """
    Compute distance = sqrt(dx*dx + dy*dy) using scalar FP instructions.

    This exercises: FMOV (GPR→FP), FMUL, FADD, FSQRT, FMOV (FP→GPR).

    The program:
      D0 = 3.0, D1 = 4.0
      D2 = D0 * D0 = 9.0  (dx^2)
      D3 = D1 * D1 = 16.0 (dy^2)
      D4 = D2 + D3 = 25.0
      D5 = sqrt(D4) = 5.0  (distance)
    """
    state = run(
        _f64_into_reg(3.0, 0, 0) +          # D0 = 3.0
        _f64_into_reg(4.0, 1, 1) + [        # D1 = 4.0
            fp_dp2src(0b01, 0, 0b0000, 0, 2),   # FMUL D2, D0, D0  (3^2=9)
            fp_dp2src(0b01, 1, 0b0000, 1, 3),   # FMUL D3, D1, D1  (4^2=16)
            fp_dp2src(0b01, 3, 0b0010, 2, 4),   # FADD D4, D2, D3  (9+16=25)
            fp_dp1src(0b01, 0b000011, 4, 5),    # FSQRT D5, D4     (sqrt(25)=5)
        ]
    )
    assert abs(state.d5 - 5.0) < 1e-10


def test_fp_program_roundtrip() -> None:
    """A double that is stored to memory and loaded back is unchanged."""
    state = run(
        _f64_into_reg(math.pi, 0, 0) + [
            dp_imm(1, 0, 0, 0x300, 0, 31, 1),
            fp_ldst_uoff(0b11, 0b00, 0, 1, 0),     # STR D0, [X1]
            fp_ldst_uoff(0b11, 0b01, 0, 1, 7),     # LDR D7, [X1]
        ]
    )
    assert abs(state.d7 - math.pi) < 1e-14


def test_fcvtzs_nan_returns_zero() -> None:
    """FCVTZS of NaN returns 0 (saturated conversion per spec)."""
    # Load NaN into D0
    nan_bits = struct.unpack(">Q", struct.pack(">d", float("nan")))[0]
    state = run([
        movwide(1, 0b10, 0, nan_bits & 0xFFFF, 0),
        movwide(1, 0b11, 1, (nan_bits >> 16) & 0xFFFF, 0),
        movwide(1, 0b11, 2, (nan_bits >> 32) & 0xFFFF, 0),
        movwide(1, 0b11, 3, (nan_bits >> 48) & 0xFFFF, 0),
        fmov_gpr_to_fp_d(0, 0),
        fcvtzs(1, 0b01, 0, 1),                      # FCVTZS X1, D0
    ])
    assert state.x1 == 0


def test_svc_is_nop() -> None:
    """SVC instruction is treated as a NOP (doesn't halt or error)."""
    # SVC encoding: 11010100 000 | imm16 | 00001
    svc_raw = (0b11010100_000 << 21) | (0 << 5) | 0b00001
    state = run([
        struct.pack(">I", svc_raw),
        dp_imm(1, 0, 0, 1, 0, 31, 0),
    ])
    assert state.x0 == 1


def test_unknown_opcode_halts_with_error() -> None:
    """An unknown opcode produces an ERROR trace and halts the simulator."""
    sim = AppleM1Simulator()
    # Use an encoding that doesn't match any known instruction
    # Bit pattern that falls through all checks: 0x7FC00000
    sim.load(struct.pack(">I", 0x7FC00000) + HALT)
    trace = sim.step()
    assert trace.mnemonic.startswith("ERROR:")
    assert sim.get_state().halted


# ─────────────────────────────────────────────────────────────────────────────
# Additional coverage tests for logical/arithmetic shifted-register operations
# ─────────────────────────────────────────────────────────────────────────────


def test_bic_reg() -> None:
    """BIC Xd, Xn, Xm clears bits in Xn that are set in Xm (logic_reg with N=1)."""
    state = run([
        movwide(1, 0b10, 0, 0xFF, 0),               # MOVZ X0, #0xFF
        movwide(1, 0b10, 0, 0x0F, 1),               # MOVZ X1, #0x0F
        logic_reg(1, 0b00, 0, 1, 1, 0, 0, 2),       # BIC X2, X0, X1 (AND X0, ~X1)
    ])
    assert state.x2 == 0xF0


def test_orn_reg() -> None:
    """ORN Xd, Xn, Xm — ORR with inverted Rm (logic_reg N=1, opc=01)."""
    state = run([
        movwide(1, 0b10, 0, 0x00, 0),               # MOVZ X0, #0 (all zeros)
        movwide(1, 0b10, 0, 0xFF, 1),               # MOVZ X1, #0xFF
        logic_reg(1, 0b01, 0, 1, 1, 0, 0, 2),       # ORN X2, X0, X1 → X0 | ~X1
    ])
    # ~0xFF in 64-bit = 0xFFFFFFFFFFFFFF00; ORR with 0 gives the same
    assert (state.x2 & 0xFF) == 0x00
    assert (state.x2 >> 8) == 0xFFFFFFFFFFFFFF


def test_eon_reg() -> None:
    """EON Xd, Xn, Xm — EOR with inverted Rm (logic_reg N=1, opc=10)."""
    state = run([
        movwide(1, 0b10, 0, 0xFF, 0),               # MOVZ X0, #0xFF
        movwide(1, 0b10, 0, 0xFF, 1),               # MOVZ X1, #0xFF
        logic_reg(1, 0b10, 0, 1, 1, 0, 0, 2),       # EON X2, X0, X1 → X0 ^ ~X1
    ])
    # ~0xFF = 0xFFFF_FFFF_FFFF_FF00; X0 XOR ~X1 = 0xFF ^ 0xFF...FF00 = 0xFF...FFFF
    assert state.x2 == 0xFFFF_FFFF_FFFF_FFFF


def test_dp_reg_add_shifted() -> None:
    """ADD Xd, Xn, Xm<<shift — arithmetic shifted-register."""
    state = run([
        movwide(1, 0b10, 0, 5, 0),                  # MOVZ X0, #5
        movwide(1, 0b10, 0, 3, 1),                  # MOVZ X1, #3
        dp_reg(1, 0, 0, 0, 1, 0, 0, 2),            # ADD X2, X0, X1 (shift=0, imm6=0)
    ])
    assert state.x2 == 8


def test_dp_reg_sub_shifted() -> None:
    """SUB Xd, Xn, Xm<<shift — arithmetic shifted-register subtraction."""
    state = run([
        movwide(1, 0b10, 0, 10, 0),
        movwide(1, 0b10, 0, 3, 1),
        dp_reg(1, 1, 0, 0, 1, 0, 0, 2),            # SUB X2, X0, X1
    ])
    assert state.x2 == 7


def test_dp_reg_adds_sets_flags() -> None:
    """ADDS Xd, Xn, Xm sets NZCV flags (S=1)."""
    state = run([
        movwide(1, 0b10, 0, 0, 0),                  # MOVZ X0, #0
        movwide(1, 0b10, 0, 0, 1),                  # MOVZ X1, #0
        dp_reg(1, 0, 1, 0, 1, 0, 0, 31),           # ADDS XZR, X0, X1 → Z=1
    ])
    assert state.z is True


def test_logic_imm_and() -> None:
    """AND Xd, Xn, #imm — logical immediate."""
    state = run([
        movwide(1, 0b10, 0, 0xFF, 0),               # MOVZ X0, #0xFF
        # AND X1, X0, #0x0F  (N=1, immr=0, imms=3 → bitmask=0x0F for 64-bit)
        logic_imm(1, 0b00, 1, 0, 3, 0, 1),
    ])
    assert state.x1 == 0x0F


def test_logic_imm_orr() -> None:
    """ORR Xd, Xn, #imm — logical immediate OR."""
    state = run([
        movwide(1, 0b10, 0, 0xF0, 0),
        # ORR X1, X0, #0x0F
        logic_imm(1, 0b01, 1, 0, 3, 0, 1),
    ])
    assert state.x1 == 0xFF


def test_logic_imm_eor() -> None:
    """EOR Xd, Xn, #imm — logical immediate XOR."""
    state = run([
        movwide(1, 0b10, 0, 0xFF, 0),
        # EOR X1, X0, #0x0F  → 0xFF ^ 0x0F = 0xF0
        logic_imm(1, 0b10, 1, 0, 3, 0, 1),
    ])
    assert state.x1 == 0xF0


def test_csel_condition_false() -> None:
    """CSEL Xd, Xn, Xm, cond selects Xm when condition is false."""
    state = run([
        movwide(1, 0b10, 0, 10, 0),                 # MOVZ X0, #10 (Rn)
        movwide(1, 0b10, 0, 20, 1),                 # MOVZ X1, #20 (Rm)
        # SUBS to set NE (X0 != X0+1 is always NE; use 0 != 1)
        movwide(1, 0b10, 0, 1, 2),
        dp_imm(1, 1, 1, 0, 0, 2, 31),              # SUBS XZR, X2, #0 → Z=0 (NE)
        csel_enc(1, 0, 0, 1, COND_EQ, 0b00, 0, 3), # CSEL X3, X0, X1, EQ → X3=X1=20
    ])
    assert state.x3 == 20


def test_csinc_condition_false() -> None:
    """CSINC Xd, Xn, Xm, cond: when false, Xd = Xm + 1."""
    state = run([
        movwide(1, 0b10, 0, 10, 0),                 # X0 = 10 (Rn)
        movwide(1, 0b10, 0, 4, 1),                  # X1 = 4 (Rm)
        movwide(1, 0b10, 0, 1, 2),
        dp_imm(1, 1, 1, 0, 0, 2, 31),              # SUBS XZR, X2, #0 → Z=0
        csel_enc(1, 0, 0, 1, COND_EQ, 0b01, 0, 3), # CSINC X3, X0, X1, EQ → X3=X1+1=5
    ])
    assert state.x3 == 5


def test_smulh() -> None:
    """SMULH Xd, Xn, Xm: signed multiply high (upper 64 bits of 128-bit product)."""
    # 2^32 * 2^32 = 2^64; upper 64 bits = 1
    state = run([
        movwide(1, 0b10, 0, 0, 0),                  # MOVZ X0, #0
        movwide(1, 0b11, 2, 1, 0),                  # MOVK X0, #1, lsl 32 → X0 = 2^32
        movwide(1, 0b10, 0, 0, 1),
        movwide(1, 0b11, 2, 1, 1),                  # X1 = 2^32
        madd_msub(1, 0b001, 1, 0, 31, 0, 2),        # SMULH X2, X0, X1
    ])
    assert state.x2 == 1  # (2^32 * 2^32) >> 64 = 1


def test_umulh() -> None:
    """UMULH Xd, Xn, Xm: unsigned multiply high."""
    state = run([
        movwide(1, 0b10, 0, 0, 0),
        movwide(1, 0b11, 2, 1, 0),                  # X0 = 2^32
        movwide(1, 0b10, 0, 0, 1),
        movwide(1, 0b11, 2, 1, 1),                  # X1 = 2^32
        madd_msub(1, 0b010, 1, 0, 31, 0, 2),        # UMULH X2, X0, X1
    ])
    assert state.x2 == 1


def test_neon_fsub_2d() -> None:
    """FSUB Vd.2D, Vn.2D, Vm.2D (U=1, opcode=11010)."""
    state = run(
        _f64_into_reg(5.0, 0, 0) + [
            neon_dup_gpr(1, 0b10000, 0, 0),         # V0 = [5.0, 5.0]
        ] +
        _f64_into_reg(2.0, 1, 1) + [
            neon_dup_gpr(1, 0b10000, 1, 1),         # V1 = [2.0, 2.0]
            # FSUB V2.2D, V0.2D, V1.2D: Q=1, U=1, sz=1, opcode=11010
            neon_3reg_same(1, 1, 0b01, 1, 0b11010, 0, 2),
        ]
    )
    lo_bits = state.vreg[2] & 0xFFFF_FFFF_FFFF_FFFF
    lo_f = struct.unpack(">d", struct.pack(">Q", lo_bits))[0]
    assert abs(lo_f - 3.0) < 1e-10


def test_neon_fmul_4s() -> None:
    """FMUL Vd.4S, Vn.4S, Vm.4S (opcode=11011)."""
    bits_2 = struct.unpack(">I", struct.pack(">f", 2.0))[0]
    bits_3 = struct.unpack(">I", struct.pack(">f", 3.0))[0]
    state = run([
        movwide(0, 0b10, 0, bits_2 & 0xFFFF, 0),
        movwide(0, 0b11, 1, (bits_2 >> 16) & 0xFFFF, 0),
        neon_dup_gpr(1, 0b01000, 0, 0),             # V0.4S = [2.0, 2.0, 2.0, 2.0]
        movwide(0, 0b10, 0, bits_3 & 0xFFFF, 1),
        movwide(0, 0b11, 1, (bits_3 >> 16) & 0xFFFF, 1),
        neon_dup_gpr(1, 0b01000, 1, 1),             # V1.4S = [3.0, 3.0, 3.0, 3.0]
        # FMUL V2.4S: Q=1, U=1, size=00 (sz=0→f32), opcode=11011
        neon_3reg_same(1, 1, 0b00, 1, 0b11011, 0, 2),
    ])
    v2 = state.vreg[2]
    for lane in range(4):
        bits_lane = (v2 >> (lane * 32)) & 0xFFFFFFFF
        f_lane = struct.unpack(">f", struct.pack(">I", bits_lane))[0]
        assert abs(f_lane - 6.0) < 1e-5, f"Lane {lane} = {f_lane}"
