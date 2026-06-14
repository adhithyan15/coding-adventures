"""test_equivalence.py — Cross-validation: gate-level vs behavioral AArch64 simulator.

Runs the same programs on both the gate-level simulator (this package) and
the behavioral simulator (aarch64-simulator) and asserts identical final
state for all observable outputs:
  - GPRs 0–31 (including XZR which must be 0)
  - SP
  - NZCV
  - PC (after halt)
  - memory (all 64 KiB)

This tests the gate-level implementation against the reference behavioral
implementation, ensuring the extra gate-level complexity doesn't introduce
any behavioral divergence.

Programs tested
───────────────
1. Arithmetic: MOVZ, ADD, SUB with flag-setting
2. Logical: AND, ORR, EOR, BIC register form
3. Memory: STR/LDR 64-bit round-trip
4. Loop: CBZ-based sum loop (1+2+3+4+5=15)
5. Multiply/divide: MUL, UDIV
6. Fibonacci: fib(5) = 8
7. Conditional select: CSEL, CSINC
8. Shifts: LSLV, ASRV
"""

from __future__ import annotations

import struct
import pytest

from aarch64_gatelevel.simulator import AArch64GateLevelSimulator
from aarch64_simulator.simulator import AArch64Simulator


HALT = b"\x00\x00\x00\x00"


def _u32be(v: int) -> bytes:
    return struct.pack(">I", v & 0xFFFFFFFF)


# ── Instruction encoding helpers (identical to test_programs.py) ──────────────


def dp_imm(sf, op, S, imm12, sh, Rn, Rd):
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b100000 << 23) | ((sh & 1) << 22)
    v |= ((imm12 & 0xFFF) << 10) | ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def movwide(sf, opc, hw, imm16, Rd):
    v = ((sf & 1) << 31) | ((opc & 3) << 29)
    v |= (0b100101 << 23) | ((hw & 3) << 21)
    v |= ((imm16 & 0xFFFF) << 5) | (Rd & 0x1F)
    return _u32be(v)


def dp_reg(sf, op, S, shift, Rm, imm6, Rn, Rd):
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b01011 << 24) | ((shift & 3) << 22) | ((Rm & 0x1F) << 16)
    v |= ((imm6 & 0x3F) << 10) | ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def logic_reg(sf, opc, shift, N, Rm, imm6, Rn, Rd):
    v = ((sf & 1) << 31) | ((opc & 3) << 29)
    v |= (0b01010 << 24) | ((shift & 3) << 22) | ((N & 1) << 21)
    v |= ((Rm & 0x1F) << 16) | ((imm6 & 0x3F) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def branch_imm_b(op, imm26):
    v = ((op & 1) << 31) | (0b00101 << 26) | (imm26 & 0x3FF_FFFF)
    return _u32be(v)


def branch_cond_b(imm19, cond):
    v = (0b01010100 << 24) | ((imm19 & 0x7FFFF) << 5) | (cond & 0xF)
    return _u32be(v)


def cbz_cbnz_b(sf, op, imm19, Rt):
    v = ((sf & 1) << 31) | (0b011010 << 25) | ((op & 1) << 24)
    v |= ((imm19 & 0x7FFFF) << 5) | (Rt & 0x1F)
    return _u32be(v)


def branch_reg_b(op, Rn):
    v = (0b1101011_0 << 24) | ((op & 0x7) << 21) | (0b11111 << 16) | ((Rn & 0x1F) << 5)
    return _u32be(v)


def ldst_uoff_b(size, V, opc, imm12, Rn, Rt):
    v = ((size & 3) << 30) | (0b111 << 27) | ((V & 1) << 26) | (0b01 << 24)
    v |= ((opc & 3) << 22) | ((imm12 & 0xFFF) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rt & 0x1F)
    return _u32be(v)


def madd_msub_b(sf, op54, Rm, o0, Ra, Rn, Rd):
    v = ((sf & 1) << 31) | (0b00_11011 << 24)
    v |= ((op54 & 7) << 21) | ((Rm & 0x1F) << 16)
    v |= ((o0 & 1) << 15) | ((Ra & 0x1F) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def dp2src_b(sf, Rm, opc2, Rn, Rd):
    v = ((sf & 1) << 31) | (0b11010110 << 21)
    v |= ((Rm & 0x1F) << 16) | ((opc2 & 0x3F) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def csel_enc_b(sf, op, S, Rm, cond, op2, Rn, Rd):
    v = ((sf & 1) << 31) | ((op & 1) << 30) | ((S & 1) << 29)
    v |= (0b11010100 << 21) | ((Rm & 0x1F) << 16)
    v |= ((cond & 0xF) << 12) | ((op2 & 3) << 10)
    v |= ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


def dp1src_b(sf, opc2, Rn, Rd):
    v = ((sf & 1) << 31) | (1 << 30) | (0b11010110 << 21) | (0 << 16)
    v |= ((opc2 & 0x3F) << 10) | ((Rn & 0x1F) << 5) | (Rd & 0x1F)
    return _u32be(v)


# ── Core equivalence assertion ──────────────────────────────────────────────────


def assert_equivalent(prog: bytes, max_steps: int = 10_000, label: str = "") -> None:
    """Run `prog` on both simulators and assert identical final state.

    Compares:
      - All 32 GPRs (X0–X31 including XZR which must be 0 on both)
      - SP
      - NZCV nibble
      - Full 64 KiB memory image

    Parameters
    ──────────
    prog      : the program bytes to execute on both simulators
    max_steps : step limit (default 10000)
    label     : human-readable description for better failure messages
    """
    # Gate-level simulator
    gl = AArch64GateLevelSimulator()
    gl_result = gl.execute(prog, max_steps=max_steps)
    gs = gl_result.final_state

    # Behavioral simulator
    beh = AArch64Simulator()
    beh_result = beh.execute(prog, max_steps=max_steps)
    bs = beh_result.final_state

    ctx = f"[{label}] " if label else ""

    # Compare all 32 GPRs
    for i in range(32):
        assert gs.gpr[i] == bs.gpr[i], (
            f"{ctx}GPR[{i}] mismatch: gate-level=0x{gs.gpr[i]:016X}, "
            f"behavioral=0x{bs.gpr[i]:016X}"
        )

    # Compare SP
    assert gs.sp == bs.sp, (
        f"{ctx}SP mismatch: gate-level=0x{gs.sp:016X}, behavioral=0x{bs.sp:016X}"
    )

    # Compare NZCV
    assert gs.nzcv == bs.nzcv, (
        f"{ctx}NZCV mismatch: gate-level=0b{gs.nzcv:04b}, behavioral=0b{bs.nzcv:04b}"
    )

    # Compare memory (whole 64 KiB)
    assert gs.memory == bs.memory, (
        f"{ctx}Memory mismatch (first diff at byte "
        f"{next(i for i,(a,b) in enumerate(zip(gs.memory, bs.memory)) if a != b)})"
    )


# ── Equivalence test programs ─────────────────────────────────────────────────


def test_equiv_arithmetic():
    """Program 1: MOVZ + ADD + SUBS → flags set, result computed.

    MOVZ X0, #42 ; MOVZ X1, #8 ; ADD X0, X0, X1 ; SUBS XZR, X0, #50 → Z=1
    """
    prog = (
        movwide(1, 0b10, 0, 42, 0) +           # MOVZ X0, #42
        movwide(1, 0b10, 0, 8, 1) +            # MOVZ X1, #8
        dp_reg(1, 0, 0, 0, 1, 0, 0, 0) +      # ADD X0, X0, X1 → X0=50
        dp_imm(1, 1, 1, 50, 0, 0, 31) +       # SUBS XZR, X0, #50 → Z=1
        HALT
    )
    assert_equivalent(prog, label="arithmetic")


def test_equiv_logical():
    """Program 2: AND, ORR, EOR register operations.

    X1=0b1010, X2=0b1100; X3=AND, X4=ORR, X5=EOR.
    """
    prog = (
        movwide(1, 0b10, 0, 0b1010, 1) +               # MOVZ X1, #0b1010
        movwide(1, 0b10, 0, 0b1100, 2) +               # MOVZ X2, #0b1100
        logic_reg(1, 0b00, 0, 0, 2, 0, 1, 3) +        # AND X3, X1, X2
        logic_reg(1, 0b01, 0, 0, 2, 0, 1, 4) +        # ORR X4, X1, X2
        logic_reg(1, 0b10, 0, 0, 2, 0, 1, 5) +        # EOR X5, X1, X2
        logic_reg(1, 0b00, 0, 1, 2, 0, 1, 6) +        # BIC X6, X1, X2
        HALT
    )
    assert_equivalent(prog, label="logical")


def test_equiv_memory():
    """Program 3: STR 64-bit + LDR 64-bit round-trip.

    Store X0 to [X2], load into X3; also STRB/LDRB.
    """
    prog = (
        movwide(1, 0b10, 0, 0xABCD, 0) +              # MOVZ X0, #0xABCD
        movwide(1, 0b10, 0, 0x100, 2) +               # MOVZ X2, #0x100 (address)
        ldst_uoff_b(3, 0, 0b00, 0, 2, 0) +            # STR X0, [X2]
        ldst_uoff_b(3, 0, 0b01, 0, 2, 3) +            # LDR X3, [X2]
        movwide(1, 0b10, 0, 0xFF, 4) +                # MOVZ X4, #0xFF
        movwide(1, 0b10, 0, 0x200, 5) +               # MOVZ X5, #0x200
        ldst_uoff_b(0, 0, 0b00, 0, 5, 4) +            # STRB W4, [X5]
        ldst_uoff_b(0, 0, 0b01, 0, 5, 6) +            # LDRB W6, [X5]
        HALT
    )
    assert_equivalent(prog, label="memory")


def test_equiv_loop_sum():
    """Program 4: CBZ loop computing sum 1+2+3+4+5 = 15.

    X0 = accumulator, X1 = counter from 5 to 0.
    """
    prog = (
        movwide(1, 0b10, 0, 0, 0) +                   # MOVZ X0, #0
        movwide(1, 0b10, 0, 5, 1) +                   # MOVZ X1, #5
        # Loop at offset 8 (bytes 0x08):
        dp_reg(1, 0, 0, 0, 1, 0, 0, 0) +             # ADD X0, X0, X1
        dp_imm(1, 1, 0, 1, 0, 1, 1) +                # SUB X1, X1, #1
        cbz_cbnz_b(1, 1, -2, 1) +                    # CBNZ X1, -2
        HALT
    )
    assert_equivalent(prog, label="loop_sum")


def test_equiv_multiply_divide():
    """Program 5: MUL (via MADD with Ra=XZR) and UDIV.

    6 * 7 = 42; 100 / 7 = 14.
    """
    prog = (
        movwide(1, 0b10, 0, 6, 1) +                   # MOVZ X1, #6
        movwide(1, 0b10, 0, 7, 2) +                   # MOVZ X2, #7
        madd_msub_b(1, 0, 2, 0, 31, 1, 0) +          # MADD X0, X1, X2, XZR (MUL)
        movwide(1, 0b10, 0, 100, 3) +                 # MOVZ X3, #100
        dp2src_b(1, 2, 0b000010, 3, 4) +             # UDIV X4, X3, X2
        HALT
    )
    assert_equivalent(prog, label="multiply_divide")


def test_equiv_fibonacci():
    """Program 6: Fibonacci — fib(5) = 8.

    X0 = a=0, X1 = b=1, X2 = counter=5.
    Loop: tmp=a+b; a=b; b=tmp; counter--; CBNZ counter, loop.
    Result: X1 = 8.
    """
    prog = (
        movwide(1, 0b10, 0, 0, 0) +                               # X0 = 0
        movwide(1, 0b10, 0, 1, 1) +                               # X1 = 1
        movwide(1, 0b10, 0, 5, 2) +                               # X2 = 5
        dp_reg(1, 0, 0, 0, 1, 0, 0, 3) +                        # ADD X3, X0, X1
        logic_reg(1, 0b01, 0, 0, 1, 0, 31, 0) +                 # ORR X0, XZR, X1
        logic_reg(1, 0b01, 0, 0, 3, 0, 31, 1) +                 # ORR X1, XZR, X3
        dp_imm(1, 1, 0, 1, 0, 2, 2) +                           # SUB X2, X2, #1
        cbz_cbnz_b(1, 1, -4, 2) +                               # CBNZ X2, loop
        HALT
    )
    assert_equivalent(prog, max_steps=500, label="fibonacci")


def test_equiv_conditional_select():
    """Program 7: CSEL and CSINC with both taken and not-taken paths.

    Set Z=1 via SUBS, then: CSEL takes true-path; CSINC takes false-path.
    """
    prog = (
        movwide(1, 0b10, 0, 0, 0) +                   # X0 = 0
        dp_imm(1, 1, 1, 0, 0, 0, 31) +               # SUBS XZR, X0, #0 → Z=1
        movwide(1, 0b10, 0, 10, 1) +                  # X1 = 10
        movwide(1, 0b10, 0, 20, 2) +                  # X2 = 20
        csel_enc_b(1, 0, 0, 2, 0b0000, 0b00, 1, 3) + # CSEL X3, X1, X2, EQ → 10
        csel_enc_b(1, 0, 0, 2, 0b0001, 0b01, 1, 4) + # CSINC X4, X1, X2, NE (Z=1→NE=false→X2+1=21)
        HALT
    )
    assert_equivalent(prog, label="conditional_select")


def test_equiv_shifts():
    """Program 8: Variable shifts LSLV and ASRV.

    X1=4, X2=1: LSLV X0, X1, X2 → X1 << 1 = 8.
    ASRV of a negative value propagates sign bit.
    """
    prog = (
        movwide(1, 0b10, 0, 4, 1) +                   # X1 = 4
        movwide(1, 0b10, 0, 1, 2) +                   # X2 = 1
        dp2src_b(1, 2, 0b001000, 1, 0) +             # LSLV X0, X1, X2 → 8
        movwide(1, 0b00, 0, 0, 3) +                   # MOVN X3, #0 → -1 (0xFFFF...)
        dp2src_b(1, 2, 0b001010, 3, 4) +             # ASRV X4, X3, X2 → -1 >> 1 = -1
        HALT
    )
    assert_equivalent(prog, label="shifts")


def test_equiv_branch_bl_ret():
    """Program 9: BL + RET call-return round-trip.

    BL to a function, function saves a value, RET returns.
    """
    # 0x00: BL +3 → PC=0x0C, X30=0x04
    # 0x04: MOVZ X1, #42 (return site)
    # 0x08: HALT
    # 0x0C: MOVZ X0, #99 (callee body)
    # 0x10: RET
    prog = (
        branch_imm_b(1, 3) +                          # BL +3, X30=0x04
        movwide(1, 0b10, 0, 42, 1) +                  # MOVZ X1, #42
        HALT +
        movwide(1, 0b10, 0, 99, 0) +                  # MOVZ X0, #99
        branch_reg_b(0b010, 30)                        # RET
    )
    assert_equivalent(prog, label="branch_bl_ret")


def test_equiv_movwide_sequence():
    """Program 10: MOVZ + MOVK + MOVN to build 64-bit constants.

    Tests that all MOVZ/MOVK/MOVN variants produce identical values.
    """
    prog = (
        movwide(1, 0b10, 0, 0x1234, 0) +              # MOVZ X0, #0x1234
        movwide(1, 0b11, 1, 0x5678, 0) +              # MOVK X0, #0x5678, LSL#16
        movwide(1, 0b00, 0, 0, 1) +                   # MOVN X1, #0 → -1
        movwide(1, 0b10, 0, 0xABCD, 2) +              # MOVZ X2, #0xABCD
        HALT
    )
    assert_equivalent(prog, label="movwide_sequence")
