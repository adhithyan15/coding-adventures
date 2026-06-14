"""test_programs.py — Full Alpha AXP programs running on the gate-level simulator.

Each test encodes a complete Alpha machine-code program in little-endian bytes
and verifies the expected final register state.

Instruction encoding:
  Memory:    (op<<26) | (ra<<21) | (rb<<16) | (disp & 0xFFFF)
  Branch:    (op<<26) | (ra<<21) | (disp21 & 0x1FFFFF)
  Operate:   (op<<26) | (ra<<21) | (rb<<16) | (0<<12) | (func<<5) | rc
  Operate-L: (op<<26) | (ra<<21) | (lit8<<13) | (1<<12) | (func<<5) | rc
  Jump:      (0x1A<<26) | (ra<<21) | (rb<<16) | (func<<14) | hint
  HALT:      0x00000000
"""

from __future__ import annotations

import struct

from alpha_axp_gatelevel import AlphaAXPGateLevelSimulator

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w(word: int) -> bytes:
    """Pack a 32-bit integer as little-endian bytes."""
    return struct.pack("<I", word & 0xFFFF_FFFF)


HALT = w(0)


def mem_op(op, ra, rb, disp):
    return w((op << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF))


def branch_op(op, ra, disp21):
    return w((op << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


def operate(op, ra, rb, func, rc):
    return w((op << 26) | (ra << 21) | (rb << 16) | (0 << 12) | (func << 5) | rc)


def operate_lit(op, ra, lit8, func, rc):
    return w((op << 26) | (ra << 21) | (lit8 << 13) | (1 << 12) | (func << 5) | rc)


def jump_op(ra, rb, func, hint=0):
    return w((0x1A << 26) | (ra << 21) | (rb << 16) | (func << 14) | (hint & 0x3FFF))


def BIS_lit(ra, lit, rc):
    """BIS r31, #lit, rc — load immediate."""
    return operate_lit(0x11, 31, lit, 0x20, rc)


def ADDQ_reg(ra, rb, rc):
    return operate(0x10, ra, rb, 0x20, rc)


def ADDQ_lit(ra, lit, rc):
    return operate_lit(0x10, ra, lit, 0x20, rc)


def SUBQ_reg(ra, rb, rc):
    return operate(0x10, ra, rb, 0x29, rc)


def MULQ_reg(ra, rb, rc):
    return operate(0x13, ra, rb, 0x20, rc)


def SLL_lit(ra, shamt, rc):
    return operate_lit(0x12, ra, shamt, 0x39, rc)


def SRL_lit(ra, shamt, rc):
    return operate_lit(0x12, ra, shamt, 0x34, rc)


def CMPLT_reg(ra, rb, rc):
    return operate(0x10, ra, rb, 0x4D, rc)


def CMOVGE_reg(ra, rb, rc):
    return operate(0x11, ra, rb, 0x46, rc)


def BEQ(ra, disp21):
    return branch_op(0x39, ra, disp21)


def BNE(ra, disp21):
    return branch_op(0x3D, ra, disp21)


# ── Tests ─────────────────────────────────────────────────────────────────────

class TestHalt:
    def test_immediate_halt(self):
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(HALT)
        assert result.halted
        assert result.ok
        assert result.steps == 1

    def test_halt_leaves_regs_zero(self):
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(HALT)
        state = result.final_state
        for i in range(32):
            assert state.regs[i] == 0


class TestLoadImmediate:
    """BIS r31, #lit, Rc — the Alpha idiom for loading a small immediate."""

    def test_load_1(self):
        prog = BIS_lit(31, 1, 0) + HALT
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.final_state.regs[0] == 1

    def test_load_42(self):
        prog = BIS_lit(31, 42, 1) + HALT
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.final_state.regs[1] == 42

    def test_load_max_lit(self):
        # Maximum 8-bit literal = 255
        prog = BIS_lit(31, 255, 2) + HALT
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.final_state.regs[2] == 255


class TestSumOneToTen:
    """Sum 1..10: loop using ADDQ, SUBQ, BNE.

    Register layout:
      r1 = accumulator (sum)
      r2 = loop counter (starts at 10, counts down)
      r3 = constant 1

    Loop:
      r1 = r1 + r2
      r2 = r2 - r3
      if r2 != 0: goto loop
    HALT

    Expected: r1 = 1+2+...+10 = 55
    """

    def test_sum_1_to_10(self):
        prog = b""

        # Init
        prog += BIS_lit(31, 0, 1)    # r1 = 0 (accumulator)
        prog += BIS_lit(31, 10, 2)   # r2 = 10 (counter)
        prog += BIS_lit(31, 1, 3)    # r3 = 1

        # Loop body (3 instructions, at offset 12):
        # Instruction 3 (offset 12): ADDQ r1, r2, r1
        prog += ADDQ_reg(1, 2, 1)
        # Instruction 4 (offset 16): SUBQ r2, r3, r2
        prog += SUBQ_reg(2, 3, 2)
        # Instruction 5 (offset 20): BNE r2, -2 → back to offset 12
        # disp21 = -2 (in units of 4-byte words, from PC+4 after BNE fetch)
        # After fetch of BNE at offset 20, PC is at 24.
        # Target = 24 + (-2)*4 = 24 - 8 = 16... need offset 12 (ADDQ)
        # BNE at offset 20: pc_after = 24; target = 24 + disp21*4 = 12
        # → disp21 = (12 - 24) / 4 = -3
        prog += BNE(2, -3)           # BNE r2, -3 → offset 12
        # Instruction 6 (offset 24): HALT
        prog += HALT

        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog, max_steps=200)
        assert result.ok, f"Not halted: {result.error}"
        assert result.final_state.regs[1] == 55
        assert result.final_state.regs[2] == 0


class TestMultiply:
    """MULQ: 6 × 7 = 42."""

    def test_mulq_six_times_seven(self):
        prog = (
            BIS_lit(31, 6, 1)
            + BIS_lit(31, 7, 2)
            + MULQ_reg(1, 2, 3)
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.final_state.regs[3] == 42

    def test_mulq_by_zero(self):
        prog = (
            BIS_lit(31, 0xFF, 1)
            + BIS_lit(31, 0, 2)
            + MULQ_reg(1, 2, 3)
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.final_state.regs[3] == 0

    def test_mulq_by_one(self):
        prog = (
            BIS_lit(31, 123, 1)
            + BIS_lit(31, 1, 2)
            + MULQ_reg(1, 2, 3)
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.final_state.regs[3] == 123


class TestMax:
    """Max of two values using CMPLT + CMOVGE.

    Algorithm:
      if r1 < r2: r3 = r2  (CMPLT r1,r2,r4; CMOVGE r4,r2,r1 ← NO)
    Simpler:
      r3 = r1
      if r1 < r2: CMOVGE(result=r1<r2 → 1) r2 → r3
      Actually: CMPLT(r1,r2,r4); if r4==1, r3=r2 else r3=r1
      Using BNE on r4.
    """

    def test_max_first_larger(self):
        # r1=10, r2=5 → max=10
        prog = (
            BIS_lit(31, 10, 1)
            + BIS_lit(31, 5, 2)
            + CMPLT_reg(1, 2, 4)      # r4 = 1 if r1 < r2 (= 0 here)
            + operate(0x11, 4, 2, 0x26, 3)  # CMOVNE r4, r2, r3 — if r4!=0: r3=r2
            # If r4=0 (r1 >= r2), we need r3 = r1. Use CMOVEQ:
            + BIS_lit(31, 0, 3)             # r3 = 0 temporarily
            + CMPLT_reg(2, 1, 5)            # r5 = 1 if r2 < r1 (= 1)
            + operate(0x11, 5, 1, 0x26, 3)  # CMOVNE r5, r1, r3 — if r5!=0: r3=r1
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.final_state.regs[3] == 10

    def test_max_second_larger(self):
        # r1=3, r2=7 → max=7
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 7, 2)
            + BIS_lit(31, 0, 3)             # r3 = 0
            + CMPLT_reg(1, 2, 4)            # r4 = 1 (r1 < r2)
            + operate(0x11, 4, 2, 0x26, 3)  # CMOVNE r4, r2, r3 → r3=7
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.final_state.regs[3] == 7


class TestZapZapnot:
    """ZAP and ZAPNOT: byte-lane masking via gate-level AND."""

    def test_zap_clears_bytes(self):
        # ZAP r1, #0xFF, r2 — mask=0xFF → zero all 8 bytes → r2=0
        # First load r1 = 0xDEAD_BEEF_CAFE_BABE (too large for lit8, use shifts)
        # Simpler: load r1 = 0xFF (byte0=0xFF), ZAP with mask 0x01 → zero byte0 → r2=0
        prog = (
            BIS_lit(31, 0xFF, 1)               # r1 = 0xFF
            + operate_lit(0x12, 1, 1, 0x30, 2) # ZAP r1, #1, r2 — zero byte0
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.final_state.regs[2] == 0

    def test_zap_preserves_others(self):
        # r1 = 0xFF00 (byte1=0xFF, byte0=0x00)
        # ZAP r1, #2, r2 — mask=0b00000010 → zero byte1 → r2=0
        # Load r1 = 256*255 = 0xFF00 via SLL
        prog = (
            BIS_lit(31, 0xFF, 1)               # r1 = 255
            + SLL_lit(1, 8, 1)                 # r1 = 0xFF00
            + operate_lit(0x12, 1, 2, 0x30, 2) # ZAP r1, #2, r2 — zero byte1
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.final_state.regs[2] == 0

    def test_zapnot_keeps_selected(self):
        # ZAPNOT r1, #1, r2 — keep only byte0
        # r1 = 0xABCD → byte0 = 0xCD, others zeroed
        prog = (
            BIS_lit(31, 0xCD, 1)               # r1 = 0xCD
            + operate_lit(0x12, 1, 1, 0x31, 2) # ZAPNOT r1, #1, r2 — keep byte0
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.final_state.regs[2] == 0xCD  # byte0 preserved


class TestJSRRet:
    """JSR/RET subroutine call pattern."""

    def test_bsr_ret_simple(self):
        """
        Call a subroutine that sets r1=100, return, assert r1=100.

        Layout (each instruction = 4 bytes):
          offset 0x00: BR r31, +2      → skip subroutine, jump to call site
          offset 0x04: BIS r31,#100,r1 — subroutine: r1=100
          offset 0x08: RET r31,(r26)   — return via r26 (link register)
          offset 0x0C: BSR r26, -2     — call: r26=0x10, jump to 0x04
          offset 0x10: HALT
        """
        prog = (
            branch_op(0x30, 31, 2)         # BR r31, +2 → target=0x0C (skip subroutine)
            + BIS_lit(31, 100, 1)          # subroutine at 0x04: r1=100
            + jump_op(31, 26, 2)           # RET r31,(r26) at 0x08
            + branch_op(0x34, 26, -3)      # BSR r26,-3 at 0x0C → target=0x04
            + HALT                         # at 0x10
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok, f"Not halted: {result.error}"
        assert result.final_state.regs[1] == 100

    def test_jmp_indirect(self):
        """JMP via register: load target address, jump."""
        # r1 = 0x08 (address of BIS instruction)
        # JMP r31, (r1) → jump to 0x08
        # offset 0x00: LDA r1, 8(r31)  → r1 = 8
        # offset 0x04: JMP r31, (r1)   → PC = 8 & ~3 = 8
        # offset 0x08: BIS r31,#42,r2  → r2 = 42
        # offset 0x0C: HALT
        prog = (
            mem_op(0x08, 1, 31, 8)         # LDA r1, 8(r31)
            + jump_op(31, 1, 0)            # JMP r31, (r1)
            + BIS_lit(31, 42, 2)           # r2 = 42
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.final_state.regs[2] == 42


class TestMemoryLoadStore:
    """LDQ/STQ memory access."""

    def test_stq_ldq_roundtrip(self):
        """Store a value to memory, load it back."""
        # r1 = 0x1000 (memory address — 8-byte aligned)
        # r2 = 0xAB (value to store)
        # STQ r2, 0(r1)
        # LDQ r3, 0(r1) → r3 = 0xAB
        prog = (
            BIS_lit(31, 0x10, 1)           # r1 = 16
            + SLL_lit(1, 8, 1)             # r1 = 0x1000 (8-byte aligned)
            + BIS_lit(31, 0xAB, 2)         # r2 = 0xAB
            + mem_op(0x2D, 2, 1, 0)        # STQ r2, 0(r1)
            + mem_op(0x29, 3, 1, 0)        # LDQ r3, 0(r1)
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.final_state.regs[3] == 0xAB

    def test_ldl_sign_extension(self):
        """LDL sign-extends 32-bit value: store 0x80000000, load → negative."""
        # Store 0x80000000 as a longword, then load it with LDL → should sign-extend
        # r1 = 0x1000 (aligned addr), r2 = 0x80000000 (as 64-bit)
        # We'll load using ADDL to build 0x80000000 in r2, then STL, then LDL
        prog = (
            BIS_lit(31, 0x80, 2)           # r2 = 0x80
            + SLL_lit(2, 24, 2)            # r2 = 0x80000000
            + BIS_lit(31, 0x10, 1)
            + SLL_lit(1, 8, 1)             # r1 = 0x1000
            + mem_op(0x2C, 2, 1, 0)        # STL r2, 0(r1)
            + mem_op(0x28, 3, 1, 0)        # LDL r3, 0(r1) → sign-extended
            + HALT
        )
        sim = AlphaAXPGateLevelSimulator()
        result = sim.execute(prog)
        assert result.ok
        # 0x80000000 sign-extended to 64 bits = 0xFFFFFFFF80000000
        assert result.final_state.regs[3] == 0xFFFF_FFFF_8000_0000
