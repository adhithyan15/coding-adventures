"""test_programs.py — Multi-instruction AArch64 gate-level simulator program tests.

Tests that the gate-level simulator correctly executes complete programs
including loops, function calls, memory operations, and arithmetic.

Uses the same instruction encoding helpers as the behavioral simulator tests.
"""

import struct
import pytest
from aarch64_gatelevel.simulator import AArch64GateLevelSimulator


HALT = b"\x00\x00\x00\x00"


def _u32be(v: int) -> bytes:
    return struct.pack(">I", v & 0xFFFFFFFF)


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


def logic_imm(sf, opc, N, immr, imms, Rn, Rd):
    v = ((sf & 1) << 31) | ((opc & 3) << 29) | (0 << 28) | (0b100100 << 22)
    v |= ((N & 1) << 22) | ((immr & 0x3F) << 16) | ((imms & 0x3F) << 10)
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


def sim():
    return AArch64GateLevelSimulator()


# ── Basic Arithmetic ──────────────────────────────────────────────────────────


def test_add_immediate():
    """ADD X0, X0, #42 → X0 = 42."""
    prog = dp_imm(1, 0, 0, 42, 0, 0, 0) + HALT
    s = sim()
    r = s.execute(prog)
    assert r.final_state.gpr[0] == 42


def test_sub_immediate():
    """MOVZ X0, #100; SUB X0, X0, #58 → X0 = 42."""
    prog = (
        movwide(1, 0b10, 0, 100, 0) +   # MOVZ X0, #100
        dp_imm(1, 1, 0, 58, 0, 0, 0) +  # SUB X0, X0, #58
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 42


def test_adds_flags_zero():
    """ADDS XZR, X0, X0 should set Z flag when X0=0."""
    prog = (
        dp_imm(1, 0, 1, 0, 0, 31, 31) +   # ADDS XZR, XZR, #0  (XZR=0)
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.z  # zero flag should be set


def test_subs_flags_carry():
    """MOVZ X0, #5; SUBS X0, X0, #3 → X0=2, C=1 (no borrow), Z=0."""
    prog = (
        movwide(1, 0b10, 0, 5, 0) +       # MOVZ X0, #5
        dp_imm(1, 1, 1, 3, 0, 0, 0) +    # SUBS X0, X0, #3
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 2
    assert r.final_state.c    # carry=1 means no borrow
    assert not r.final_state.z


def test_movz_basic():
    """MOVZ X3, #0xABCD → X3 = 0xABCD."""
    prog = movwide(1, 0b10, 0, 0xABCD, 3) + HALT
    r = sim().execute(prog)
    assert r.final_state.gpr[3] == 0xABCD


def test_movn_basic():
    """MOVN X0, #0 → X0 = 0xFFFFFFFFFFFFFFFF (all ones = -1)."""
    prog = movwide(1, 0b00, 0, 0, 0) + HALT
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 0xFFFFFFFFFFFFFFFF


def test_movk_combines():
    """MOVZ X0, #0x1234; MOVK X0, #0x5678, LSL#16 → X0 = 0x56781234."""
    prog = (
        movwide(1, 0b10, 0, 0x1234, 0) +   # MOVZ X0, #0x1234
        movwide(1, 0b11, 1, 0x5678, 0) +   # MOVK X0, #0x5678, LSL#16
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 0x56781234


def test_add_register():
    """MOVZ X1, #3; MOVZ X2, #4; ADD X0, X1, X2 → X0 = 7."""
    prog = (
        movwide(1, 0b10, 0, 3, 1) +
        movwide(1, 0b10, 0, 4, 2) +
        dp_reg(1, 0, 0, 0, 2, 0, 1, 0) +  # ADD X0, X1, X2
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 7


def test_sub_register():
    """MOVZ X1, #10; MOVZ X2, #3; SUB X0, X1, X2 → X0 = 7."""
    prog = (
        movwide(1, 0b10, 0, 10, 1) +
        movwide(1, 0b10, 0, 3, 2) +
        dp_reg(1, 1, 0, 0, 2, 0, 1, 0) +  # SUB X0, X1, X2
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 7


# ── Logic Instructions ─────────────────────────────────────────────────────────


def test_and_register():
    """MOVZ X1, #0b1100; MOVZ X2, #0b1010; AND X0, X1, X2 → X0 = 8."""
    prog = (
        movwide(1, 0b10, 0, 0b1100, 1) +
        movwide(1, 0b10, 0, 0b1010, 2) +
        logic_reg(1, 0b00, 0, 0, 2, 0, 1, 0) +  # AND
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 0b1000


def test_orr_register():
    """ORR X0, XZR, X1 = X1 (MOV alias)."""
    prog = (
        movwide(1, 0b10, 0, 42, 1) +
        logic_reg(1, 0b01, 0, 0, 1, 0, 31, 0) +  # ORR X0, XZR, X1
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 42


def test_xor_register():
    """X0 = X0 XOR X0 = 0."""
    prog = (
        movwide(1, 0b10, 0, 0xABCD, 0) +
        logic_reg(1, 0b10, 0, 0, 0, 0, 0, 0) +   # EOR X0, X0, X0
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 0


def test_bic_register():
    """BIC X0, X1, X2 = AND(X1, NOT(X2))."""
    prog = (
        movwide(1, 0b10, 0, 0b1111, 1) +
        movwide(1, 0b10, 0, 0b1010, 2) +
        logic_reg(1, 0b00, 0, 1, 2, 0, 1, 0) +   # BIC
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 0b0101


# ── Branches and Loops ─────────────────────────────────────────────────────────


def test_b_skip():
    """B skips over an instruction."""
    prog = (
        branch_imm_b(0, 2) +                  # B #+8 (skip next 1 instr)
        movwide(1, 0b10, 0, 99, 0) +          # MOVZ X0, #99 (should be skipped)
        movwide(1, 0b10, 0, 42, 1) +          # MOVZ X1, #42
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 0    # skipped
    assert r.final_state.gpr[1] == 42


def test_bl_and_ret():
    """BL saves return address; RET returns to caller."""
    # prog layout (4 bytes each):
    # 0x00: BL +2 (jump to 0x08)
    # 0x04: MOVZ X1, #42  (should execute after RET)
    # 0x08: MOVZ X0, #99   (function body)
    # 0x0C: RET
    # 0x10: HALT
    prog = (
        branch_imm_b(1, 2) +                  # BL #+8 → 0x08, X30=0x04
        movwide(1, 0b10, 0, 42, 1) +          # MOVZ X1, #42
        HALT +                                  # 0x0C — but RET returns to 0x04+4=0x08? No...
        # Wait: BL at 0x00 saves next_pc=0x04 in X30, then jumps to 0x08
        # At 0x08: execute some code, then RET → X30=0x04
        # At 0x04: MOVZ X1, #42
        # At 0x08: nop would work, but we need RET at 0x0C
        # Let me restructure:
        b""
    )
    # Restructured:
    # 0x00: BL +3  → PC=0x0C; X30=0x04
    # 0x04: MOVZ X1, #42  (return site)
    # 0x08: HALT  (after return site, end)
    # 0x0C: MOVZ X0, #99  (function)
    # 0x10: RET   (returns to X30=0x04)
    prog2 = (
        branch_imm_b(1, 3) +       # BL #+12, saves X30=0x04
        movwide(1, 0b10, 0, 42, 1) +  # MOVZ X1, #42
        HALT +
        movwide(1, 0b10, 0, 99, 0) +  # MOVZ X0, #99 (callee)
        branch_reg_b(0b010, 30)        # RET (branch to X30=0x04)
    )
    r = sim().execute(prog2)
    assert r.final_state.gpr[0] == 99
    assert r.final_state.gpr[1] == 42


def test_cbz_loop():
    """Sum 1+2+...+5 using CBZ loop."""
    # X0 = sum (accumulator), X1 = counter (starts at 5)
    # Loop: ADD X0, X0, X1; SUB X1, X1, #1; CBNZ X1, -2 instructions
    #
    # 0x00: MOVZ X0, #0   (sum = 0)
    # 0x04: MOVZ X1, #5   (counter = 5)
    # 0x08: ADD X0, X0, X1  (sum += counter)
    # 0x0C: SUB X1, X1, #1  (counter--)
    # 0x10: CBNZ X1, -2   (if counter != 0: jump back 2 instr = 0x08)
    # 0x14: HALT
    prog = (
        movwide(1, 0b10, 0, 0, 0) +           # MOVZ X0, #0
        movwide(1, 0b10, 0, 5, 1) +           # MOVZ X1, #5
        dp_reg(1, 0, 0, 0, 1, 0, 0, 0) +     # ADD X0, X0, X1
        dp_imm(1, 1, 0, 1, 0, 1, 1) +        # SUB X1, X1, #1
        cbz_cbnz_b(1, 1, -2, 1) +            # CBNZ X1, -2 (back to ADD)
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 15   # 1+2+3+4+5


def test_b_cond_loop():
    """Conditional branch: count down from 3 to 0."""
    # X0 = 3; loop: SUB X0, X0, #1; SUBS X1, X0, #0 (flags); B.NE -2
    # 0x00: MOVZ X0, #3
    # 0x04: SUBS X0, X0, #1    (also sets flags)
    # 0x08: B.NE -1  (jump back 1 instruction if X0 != 0)
    # 0x0C: HALT
    prog = (
        movwide(1, 0b10, 0, 3, 0) +
        dp_imm(1, 1, 1, 1, 0, 0, 0) +    # SUBS X0, X0, #1 (sets Z when 0)
        branch_cond_b(-1, 0b0001) +        # B.NE #+(-4)
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 0
    assert r.final_state.z


# ── Memory ────────────────────────────────────────────────────────────────────


def test_str_ldr_64():
    """Store X0 to memory, load into X1."""
    # 0x00: MOVZ X0, #0xABCD
    # 0x04: MOVZ X2, #0x100   (address)
    # 0x08: STR X0, [X2]
    # 0x0C: LDR X1, [X2]
    # 0x10: HALT
    prog = (
        movwide(1, 0b10, 0, 0xABCD, 0) +
        movwide(1, 0b10, 0, 0x100, 2) +   # X2 = address 0x100
        ldst_uoff_b(3, 0, 0b00, 0, 2, 0) + # STR X0, [X2]
        ldst_uoff_b(3, 0, 0b01, 0, 2, 1) + # LDR X1, [X2]
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[1] == 0xABCD


def test_strb_ldrb():
    """Store byte, load byte (zero-extended)."""
    prog = (
        movwide(1, 0b10, 0, 0xFF, 0) +    # X0 = 0xFF
        movwide(1, 0b10, 0, 0x200, 2) +
        ldst_uoff_b(0, 0, 0b00, 0, 2, 0) + # STRB W0, [X2]
        ldst_uoff_b(0, 0, 0b01, 0, 2, 1) + # LDRB W1, [X2]
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[1] == 0xFF


# ── Multiply / Divide ─────────────────────────────────────────────────────────


def test_mul():
    """MUL X0, X1, X2 = X1 * X2."""
    prog = (
        movwide(1, 0b10, 0, 6, 1) +
        movwide(1, 0b10, 0, 7, 2) +
        madd_msub_b(1, 0, 2, 0, 31, 1, 0) +  # MADD X0, X1, X2, XZR
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 42


def test_madd():
    """MADD X0, X1, X2, X3 = X3 + X1*X2."""
    prog = (
        movwide(1, 0b10, 0, 3, 1) +    # X1 = 3
        movwide(1, 0b10, 0, 4, 2) +    # X2 = 4
        movwide(1, 0b10, 0, 10, 3) +   # X3 = 10
        madd_msub_b(1, 0, 2, 0, 3, 1, 0) +   # MADD X0, X1, X2, X3
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 22   # 10 + 3*4


def test_udiv():
    """UDIV X0, X1, X2 = X1 / X2."""
    prog = (
        movwide(1, 0b10, 0, 100, 1) +
        movwide(1, 0b10, 0, 7, 2) +
        dp2src_b(1, 2, 0b000010, 1, 0) +   # UDIV X0, X1, X2
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 14


# ── Shift Instructions ────────────────────────────────────────────────────────


def test_lsl_register():
    """LSLV X0, X1, X2: X1 << X2."""
    prog = (
        movwide(1, 0b10, 0, 1, 1) +    # X1 = 1
        movwide(1, 0b10, 0, 4, 2) +    # X2 = 4
        dp2src_b(1, 2, 0b001000, 1, 0) +  # LSLV X0, X1, X2
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 16


def test_shifted_add_register():
    """ADD X0, X1, X2, LSL #2 = X1 + (X2 << 2)."""
    prog = (
        movwide(1, 0b10, 0, 5, 1) +    # X1 = 5
        movwide(1, 0b10, 0, 3, 2) +    # X2 = 3
        dp_reg(1, 0, 0, 0, 2, 2, 1, 0) +  # ADD X0, X1, X2, LSL #2 → 5 + 12 = 17
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 17


# ── Conditional Select ─────────────────────────────────────────────────────────


def test_csel_eq_taken():
    """CSEL X0, X1, X2, EQ: Z=1 → X0 = X1."""
    prog = (
        movwide(1, 0b10, 0, 0, 0) +       # X0 = 0 (to trigger Z flag via SUBS)
        dp_imm(1, 1, 1, 0, 0, 0, 31) +   # SUBS XZR, X0, #0 → Z=1
        movwide(1, 0b10, 0, 10, 1) +      # X1 = 10
        movwide(1, 0b10, 0, 20, 2) +      # X2 = 20
        csel_enc_b(1, 0, 0, 2, 0b0000, 0b00, 1, 0) +  # CSEL X0, X1, X2, EQ
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 10   # EQ → takes X1


def test_csel_eq_not_taken():
    """CSEL X0, X1, X2, EQ: Z=0 → X0 = X2."""
    prog = (
        movwide(1, 0b10, 0, 5, 0) +
        dp_imm(1, 1, 1, 3, 0, 0, 31) +   # SUBS XZR, X0, #3 → Z=0, C=1
        movwide(1, 0b10, 0, 10, 1) +
        movwide(1, 0b10, 0, 20, 2) +
        csel_enc_b(1, 0, 0, 2, 0b0000, 0b00, 1, 0) +  # CSEL X0, X1, X2, EQ
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 20   # Z=0 → takes X2


def test_csinc():
    """CSINC X0, X1, X2, EQ: not EQ → X0 = X2 + 1."""
    prog = (
        movwide(1, 0b10, 0, 5, 0) +
        dp_imm(1, 1, 1, 3, 0, 0, 31) +   # Z=0
        movwide(1, 0b10, 0, 10, 1) +
        movwide(1, 0b10, 0, 20, 2) +
        csel_enc_b(1, 0, 0, 2, 0b0000, 0b01, 1, 0) +   # CSINC X0, X1, X2, EQ
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 21   # X2 + 1 = 21


# ── CLZ ───────────────────────────────────────────────────────────────────────


def test_clz_64():
    """CLZ X0, X1 counts leading zeros."""
    prog = (
        movwide(1, 0b10, 0, 1, 1) +       # X1 = 1 (63 leading zeros)
        dp1src_b(1, 0b000100, 1, 0) +     # CLZ X0, X1
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 63


def test_clz_zero():
    """CLZ of 0 returns 64."""
    prog = (
        dp1src_b(1, 0b000100, 31, 0) +    # CLZ X0, XZR (XZR=0)
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 64


# ── Fibonacci ─────────────────────────────────────────────────────────────────


def test_fibonacci_5():
    """Compute 6th Fibonacci number (fib(5) = 8): 0,1,1,2,3,5,8."""
    # X0 = a = 0, X1 = b = 1, X2 = counter = 5
    # Loop: tmp = a + b; a = b; b = tmp; counter--; CBNZ counter, loop
    # Result in X1
    prog = (
        movwide(1, 0b10, 0, 0, 0) +       # X0 = 0
        movwide(1, 0b10, 0, 1, 1) +       # X1 = 1
        movwide(1, 0b10, 0, 5, 2) +       # X2 = 5 (loop 5 times)
        # Loop start at 0x0C:
        dp_reg(1, 0, 0, 0, 1, 0, 0, 3) + # ADD X3, X0, X1  (tmp = a + b)
        logic_reg(1, 0b01, 0, 0, 1, 0, 31, 0) + # MOV X0, X1 (a = b)
        logic_reg(1, 0b01, 0, 0, 3, 0, 31, 1) + # MOV X1, X3 (b = tmp)
        dp_imm(1, 1, 0, 1, 0, 2, 2) +    # SUB X2, X2, #1
        cbz_cbnz_b(1, 1, -4, 2) +        # CBNZ X2, loop (back 4 instructions = -16 bytes = -4 words)
        HALT
    )
    r = sim().execute(prog, max_steps=200)
    assert r.final_state.gpr[1] == 8


# ── XZR always reads 0 ───────────────────────────────────────────────────────


def test_xzr_source():
    """Reading XZR as a source register gives 0."""
    prog = (
        movwide(1, 0b10, 0, 100, 0) +    # X0 = 100
        dp_reg(1, 0, 0, 0, 31, 0, 0, 0) + # ADD X0, X0, XZR → X0 stays 100
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[0] == 100


def test_xzr_destination():
    """Writing to XZR (index 31) discards the result."""
    prog = (
        movwide(1, 0b10, 0, 42, 31) +   # MOVZ XZR, #42 → discarded
        HALT
    )
    r = sim().execute(prog)
    assert r.final_state.gpr[31] == 0


# ── 32-bit W register operations ─────────────────────────────────────────────


def test_w_register_zero_extends():
    """Writing to W register (sf=0) clears upper 32 bits.

    Build X0 = 0xFFFF_FFFF_FFFF_FFFF using MOVN (all ones), then ADD W0, W0, #0
    (sf=0) which writes only 32 bits and zero-extends.  After the W-register
    write, upper 32 bits must be 0, so X0 = 0xFFFF_FFFF.
    """
    prog = (
        movwide(1, 0b00, 0, 0, 0) +       # MOVN X0, #0 → X0 = 0xFFFFFFFFFFFFFFFF
        dp_imm(0, 0, 0, 0, 0, 0, 0) +    # ADD W0, W0, #0 (sf=0, zero-extends to 64-bit)
        HALT
    )
    r = sim().execute(prog)
    # After W-register write, upper 32 bits should be 0
    assert r.final_state.gpr[0] == 0xFFFFFFFF  # only 32-bit result, zero-extended
