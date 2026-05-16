"""test_equivalence.py — Cross-validation: gate-level vs behavioral Alpha simulator.

Each test builds a short Alpha machine-code program, runs it on both the
behavioral AlphaSimulator and the gate-level AlphaAXPGateLevelSimulator, and
asserts that the final register state is identical.

This proves that the gate-level simulation produces the exact same results as
the reference behavioral model.

Instruction encoding helpers
─────────────────────────────
  Memory:    (op<<26) | (ra<<21) | (rb<<16) | (disp & 0xFFFF)
  Branch:    (op<<26) | (ra<<21) | (disp21 & 0x1FFFFF)
  Operate:   (op<<26) | (ra<<21) | (rb<<16) | (0<<12) | (func<<5) | rc
  Operate-L: (op<<26) | (ra<<21) | (lit8<<13) | (1<<12) | (func<<5) | rc
  Jump:      (0x1A<<26) | (ra<<21) | (rb<<16) | (func<<14) | hint
  HALT:      0x00000000
"""

from __future__ import annotations

import struct

from alpha_axp_simulator import AlphaSimulator as BehavioralSim

from alpha_axp_gatelevel import AlphaAXPGateLevelSimulator as GateSim

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w(word: int) -> bytes:
    """Pack a 32-bit word as little-endian bytes."""
    return struct.pack("<I", word & 0xFFFF_FFFF)


HALT = w(0x00000000)


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


# ── Cross-validation helper ───────────────────────────────────────────────────

def cross_validate(prog: bytes, max_steps: int = 10_000) -> tuple:
    """Run prog on both simulators, return (behavioral_state, gate_state)."""
    b = BehavioralSim()
    b_result = b.execute(prog, max_steps=max_steps)
    bs = b_result.final_state

    g = GateSim()
    g_result = g.execute(prog, max_steps=max_steps)
    gs = g_result.final_state

    # Both should have halted cleanly
    assert b_result.halted, f"Behavioral not halted: {b_result.error}"
    assert g_result.halted, f"Gate-level not halted: {g_result.error}"

    return bs, gs


def assert_equiv(prog: bytes, max_steps: int = 10_000):
    """Assert that gate-level matches behavioral for the given program."""
    bs, gs = cross_validate(prog, max_steps)
    assert bs.regs == gs.regs, (
        f"Register mismatch!\n"
        f"Behavioral: {[hex(r) for r in bs.regs]}\n"
        f"Gate-level: {[hex(r) for r in gs.regs]}"
    )


# ── Test programs ─────────────────────────────────────────────────────────────

class TestEquivalenceArithmetic:
    """ADDQ, SUBQ, ADDL: arithmetic operations."""

    def test_addq_simple(self):
        # r1 = 3, r2 = 4, r3 = r1 + r2
        prog = (
            operate_lit(0x11, 31, 3, 0x20, 1)    # BIS r31, #3, r1  (r1=3)
            + operate_lit(0x11, 31, 4, 0x20, 2)  # BIS r31, #4, r2  (r2=4)
            + operate(0x10, 1, 2, 0x20, 3)        # ADDQ r1, r2, r3
            + HALT
        )
        assert_equiv(prog)

    def test_subq_simple(self):
        # r1 = 10, r2 = 3, r3 = r1 - r2 = 7
        prog = (
            operate_lit(0x11, 31, 10, 0x20, 1)
            + operate_lit(0x11, 31, 3, 0x20, 2)
            + operate(0x10, 1, 2, 0x29, 3)        # SUBQ r1, r2, r3
            + HALT
        )
        assert_equiv(prog)

    def test_addl_sign_extension(self):
        # ADDL with 32-bit overflow: r1 = 0x7FFFFFFF, r2 = 1
        # r3 = sext32(0x7FFFFFFF + 1) = sext32(0x80000000) = 0xFFFFFFFF80000000
        prog = (
            operate_lit(0x11, 31, 0x7F, 0x20, 1)  # BIS r31, #127, r1
            + operate_lit(0x12, 1, 24, 0x39, 1)   # SLL r1, #24, r1 → r1 = 0x7F000000
            + operate_lit(0x11, 31, 0xFF, 0x20, 4) # BIS r31, #255, r4
            + operate_lit(0x12, 4, 16, 0x39, 4)   # SLL r4, #16 → r4 = 0x00FF0000
            + operate(0x11, 1, 4, 0x20, 1)         # BIS r1, r4, r1 → r1 = 0x7FFF0000
            + operate_lit(0x11, 31, 0xFF, 0x20, 4) # r4 = 0xFF
            + operate_lit(0x12, 4, 8, 0x39, 4)    # SLL r4, #8 → 0xFF00
            + operate(0x11, 1, 4, 0x20, 1)         # r1 = 0x7FFFFF00
            + operate_lit(0x11, 31, 0xFF, 0x20, 4) # r4 = 0xFF
            + operate(0x11, 1, 4, 0x20, 1)         # r1 = 0x7FFFFFFF
            + operate_lit(0x11, 31, 1, 0x20, 2)    # r2 = 1
            + operate(0x10, 1, 2, 0x00, 3)          # ADDL r1, r2, r3
            + HALT
        )
        assert_equiv(prog)

    def test_subq_borrow(self):
        # r1 = 0, r2 = 1: r3 = 0 - 1 = 0xFFFF...FFFF
        prog = (
            operate_lit(0x11, 31, 0, 0x20, 1)
            + operate_lit(0x11, 31, 1, 0x20, 2)
            + operate(0x10, 1, 2, 0x29, 3)
            + HALT
        )
        assert_equiv(prog)


class TestEquivalenceBitwise:
    """AND, BIS (OR), XOR, ORNOT, BIC, EQV."""

    def test_and(self):
        prog = (
            operate_lit(0x11, 31, 0b1100, 0x20, 1)
            + operate_lit(0x11, 31, 0b1010, 0x20, 2)
            + operate(0x11, 1, 2, 0x00, 3)   # AND
            + HALT
        )
        assert_equiv(prog)

    def test_bis_or(self):
        prog = (
            operate_lit(0x11, 31, 0b0011, 0x20, 1)
            + operate_lit(0x11, 31, 0b1100, 0x20, 2)
            + operate(0x11, 1, 2, 0x20, 3)   # BIS
            + HALT
        )
        assert_equiv(prog)

    def test_xor(self):
        prog = (
            operate_lit(0x11, 31, 0xFF, 0x20, 1)
            + operate_lit(0x11, 31, 0xF0, 0x20, 2)
            + operate(0x11, 1, 2, 0x40, 3)   # XOR
            + HALT
        )
        assert_equiv(prog)

    def test_bic(self):
        prog = (
            operate_lit(0x11, 31, 0xFF, 0x20, 1)
            + operate_lit(0x11, 31, 0x0F, 0x20, 2)
            + operate(0x11, 1, 2, 0x08, 3)   # BIC
            + HALT
        )
        assert_equiv(prog)

    def test_ornot(self):
        prog = (
            operate_lit(0x11, 31, 0, 0x20, 1)
            + operate_lit(0x11, 31, 0, 0x20, 2)
            + operate(0x11, 1, 2, 0x28, 3)   # ORNOT
            + HALT
        )
        assert_equiv(prog)


class TestEquivalenceShifts:
    """SLL, SRL, SRA."""

    def test_sll(self):
        prog = (
            operate_lit(0x11, 31, 1, 0x20, 1)   # r1 = 1
            + operate_lit(0x12, 1, 4, 0x39, 2)  # SLL r1, #4, r2
            + HALT
        )
        assert_equiv(prog)

    def test_srl(self):
        prog = (
            operate_lit(0x11, 31, 16, 0x20, 1)  # r1 = 16
            + operate_lit(0x12, 1, 2, 0x34, 2)  # SRL r1, #2, r2
            + HALT
        )
        assert_equiv(prog)

    def test_sra(self):
        # SRA with sign extension — func=0x3C is the canonical Alpha SRA encoding
        # supported by both the behavioral and gate-level simulators.
        prog = (
            operate_lit(0x11, 31, 8, 0x20, 1)   # r1 = 8
            + operate_lit(0x12, 1, 1, 0x3C, 2)  # SRA r1, #1, r2  (func=0x3C)
            + HALT
        )
        assert_equiv(prog)


class TestEquivalenceCompare:
    """CMPEQ, CMPLT, CMPULT — compare writes 0 or 1 to Rc."""

    def test_cmpeq_true(self):
        prog = (
            operate_lit(0x11, 31, 5, 0x20, 1)
            + operate_lit(0x11, 31, 5, 0x20, 2)
            + operate(0x10, 1, 2, 0x2D, 3)  # CMPEQ
            + HALT
        )
        assert_equiv(prog)

    def test_cmpeq_false(self):
        prog = (
            operate_lit(0x11, 31, 5, 0x20, 1)
            + operate_lit(0x11, 31, 6, 0x20, 2)
            + operate(0x10, 1, 2, 0x2D, 3)  # CMPEQ
            + HALT
        )
        assert_equiv(prog)

    def test_cmplt(self):
        prog = (
            operate_lit(0x11, 31, 3, 0x20, 1)
            + operate_lit(0x11, 31, 5, 0x20, 2)
            + operate(0x10, 1, 2, 0x4D, 3)  # CMPLT
            + HALT
        )
        assert_equiv(prog)

    def test_cmpult(self):
        prog = (
            operate_lit(0x11, 31, 3, 0x20, 1)
            + operate_lit(0x11, 31, 5, 0x20, 2)
            + operate(0x10, 1, 2, 0x3D, 3)  # CMPULT
            + HALT
        )
        assert_equiv(prog)


class TestEquivalenceMulq:
    """MULQ: lower 64 bits of 64×64 multiply."""

    def test_small(self):
        prog = (
            operate_lit(0x11, 31, 6, 0x20, 1)
            + operate_lit(0x11, 31, 7, 0x20, 2)
            + operate(0x13, 1, 2, 0x20, 3)   # MULQ
            + HALT
        )
        assert_equiv(prog)

    def test_by_zero(self):
        prog = (
            operate_lit(0x11, 31, 0xFF, 0x20, 1)
            + operate_lit(0x11, 31, 0, 0x20, 2)
            + operate(0x13, 1, 2, 0x20, 3)
            + HALT
        )
        assert_equiv(prog)


class TestEquivalenceBranch:
    """BEQ, BNE: conditional branches."""

    def test_beq_not_taken(self):
        # r1 = 1 ≠ 0, so BEQ not taken; ADDQ executes; HALT
        prog = (
            operate_lit(0x11, 31, 1, 0x20, 1)   # r1 = 1
            + branch_op(0x39, 1, 1)              # BEQ r1, +1 (skip next)
            + operate_lit(0x11, 31, 42, 0x20, 2) # r2 = 42
            + HALT
        )
        assert_equiv(prog)

    def test_beq_taken(self):
        # r1 = 0, BEQ taken → skip ADDQ → r2 = 0
        prog = (
            operate_lit(0x11, 31, 0, 0x20, 1)   # r1 = 0
            + branch_op(0x39, 1, 1)              # BEQ r1, +1 → skip next
            + operate_lit(0x11, 31, 42, 0x20, 2) # r2 = 42 (SKIPPED)
            + HALT
        )
        assert_equiv(prog)

    def test_bne_taken(self):
        # r1 = 5 ≠ 0, BNE taken → skip
        prog = (
            operate_lit(0x11, 31, 5, 0x20, 1)
            + branch_op(0x3D, 1, 1)              # BNE r1, +1 → skip
            + operate_lit(0x11, 31, 99, 0x20, 2) # r2 = 99 (SKIPPED)
            + HALT
        )
        assert_equiv(prog)


class TestEquivalenceJSRRet:
    """JSR/RET subroutine call — r26 = link register."""

    def test_jsr_ret(self):
        # Simple: BR to skip the subroutine body, then call it via BSR
        # Layout:
        #   0x00: BR r31, +2      (skip subroutine)
        #   0x04: BIS r31,#42,r1  (subroutine: r1=42)
        #   0x08: RET r31,(r26)   (subroutine: return)
        #   0x0C: BSR r26, -2     (call subroutine at 0x04)
        #   0x10: HALT
        prog = (
            branch_op(0x30, 31, 2)               # BR r31, +2 → PC=0x0C (skip sub)
            + operate_lit(0x11, 31, 42, 0x20, 1) # subroutine body: r1=42
            + jump_op(31, 26, 2)                  # RET r31, (r26)
            + branch_op(0x34, 26, -2)             # BSR r26, -2 → call at 0x04
            + HALT
        )
        assert_equiv(prog)
