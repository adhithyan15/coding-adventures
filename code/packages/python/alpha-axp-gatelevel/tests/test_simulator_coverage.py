"""test_simulator_coverage.py — Coverage tests for uncovered simulator paths.

These tests exercise instructions and error paths not covered by the other
test files:  CMOV variants, scaled-add/sub, byte manipulation (EXT/INS/MSK),
SEXTB/SEXTW, UMULH, MULL, LDAH, LDBU, LDQ_U, STQ_U, branch variants
(BLT, BLE, BGT, BGE, BLBC, BLBS), and error/edge paths (unknown opcode,
unsupported PALcode, already-halted, max_steps exceeded).

Instruction encoding:
  Operate:   (op<<26)|(ra<<21)|(rb<<16)|(0<<12)|(func<<5)|rc
  Operate-L: (op<<26)|(ra<<21)|(lit8<<13)|(1<<12)|(func<<5)|rc
  Memory:    (op<<26)|(ra<<21)|(rb<<16)|(disp&0xFFFF)
  Branch:    (op<<26)|(ra<<21)|(disp21&0x1FFFFF)
  Jump:      (0x1A<<26)|(ra<<21)|(rb<<16)|(func<<14)|hint
"""

from __future__ import annotations

import struct

import pytest

from alpha_axp_gatelevel import AlphaAXPGateLevelSimulator

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w(word: int) -> bytes:
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
    """BIS r31, #lit, rc — load small immediate (0–255)."""
    return operate_lit(0x11, 31, lit, 0x20, rc)


def SLL_lit(ra, shamt, rc):
    return operate_lit(0x12, ra, shamt, 0x39, rc)


def SRL_lit(ra, shamt, rc):
    return operate_lit(0x12, ra, shamt, 0x34, rc)


def run(prog, max_steps=1000):
    """Helper: execute prog, return result."""
    sim = AlphaAXPGateLevelSimulator()
    return sim.execute(prog, max_steps=max_steps)


def regs(prog, max_steps=1000):
    """Helper: execute prog, return register tuple."""
    return run(prog, max_steps).final_state.regs


# ── Error / edge paths ────────────────────────────────────────────────────────

class TestErrorPaths:
    """Unknown opcode, unsupported PALcode, already-halted, max_steps."""

    def test_unknown_opcode_halts_with_error(self):
        # Opcode 0x07 is undefined in Alpha AXP.
        bad = w(0x07 << 26) + HALT
        result = run(bad)
        assert result.halted
        assert result.error is not None
        assert "Unknown" in result.error

    def test_unsupported_palcode_halts(self):
        # PALcode 1 (CFLUSH) is not HALT=0, so we treat it as unsupported.
        palcode_1 = w(0x00000001) + HALT
        result = run(palcode_1)
        assert result.halted
        assert result.error is not None

    def test_step_when_already_halted(self):
        sim = AlphaAXPGateLevelSimulator()
        sim.load(HALT)
        # First step → HALT
        t1 = sim.step()
        assert t1.mnemonic == "HALT"
        # Second step → already halted
        t2 = sim.step()
        assert t2.mnemonic == "HALT"
        assert t2.pc_before == t2.pc_after

    def test_max_steps_exceeded(self):
        # Infinite loop: BNE r31, -1 — r31 is hardwired 0, so BNE not taken,
        # so this is actually a NOP loop that just increments PC and halts when
        # PC wraps. Use an actual infinite loop: BNE r1, -1 where r1=1.
        # BNE at offset 0 with disp21=-1 → target = (0+4) + (-1)*4 = 0 (infinite).
        # r1 = 1 (non-zero) so BNE is always taken.
        prog = BIS_lit(31, 1, 1) + branch_op(0x3D, 1, 0x1FFFFF)  # disp21 = -1
        result = run(prog, max_steps=20)
        assert not result.halted
        assert result.error is not None
        assert "max_steps" in result.error
        assert result.steps == 20

    def test_program_too_large(self):
        sim = AlphaAXPGateLevelSimulator()
        with pytest.raises(ValueError, match="exceeds memory"):
            sim.load(bytes(65537))

    def test_get_set_ports_and_interrupt(self):
        # These are no-ops on Alpha — just verify they don't raise.
        sim = AlphaAXPGateLevelSimulator()
        sim.set_input_port(0, 42)
        assert sim.get_output_port(0) == 0
        sim.interrupt()
        sim.nmi()

    def test_unknown_intl_func_raises(self):
        # func=0x99 is not a valid INTL function.
        bad = operate_lit(0x11, 31, 0, 0x7F, 1) + HALT
        result = run(bad)
        assert result.halted
        assert result.error is not None

    def test_unknown_ints_func_raises(self):
        # func=0x7F is not a valid INTS function.
        bad = operate_lit(0x12, 31, 0, 0x7F, 1) + HALT
        result = run(bad)
        assert result.halted
        assert result.error is not None

    def test_unknown_inta_func_raises(self):
        # func=0x7F is not valid for INTA.
        bad = operate_lit(0x10, 31, 0, 0x7F, 1) + HALT
        result = run(bad)
        assert result.halted
        assert result.error is not None


# ── INTL CMOV variants ────────────────────────────────────────────────────────

class TestCMOV:
    """Conditional moves: CMOVLBS, CMOVLBC, CMOVEQ, CMOVNE, CMOVLT, CMOVGE,
    CMOVLE, CMOVGT."""

    def test_cmovlbs_taken(self):
        # CMOVLBS: move if Ra[0] == 1.  r1=3 (bit0=1) → r2 = 99
        prog = (
            BIS_lit(31, 3, 1)                    # r1 = 3 (odd)
            + BIS_lit(31, 99, 2)                 # r2 = 99 (to be copied to r3 if taken)
            + BIS_lit(31, 0, 3)                  # r3 = 0
            + operate(0x11, 1, 2, 0x14, 3)       # CMOVLBS r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 99

    def test_cmovlbs_not_taken(self):
        # r1=2 (bit0=0) → CMOVLBS not taken → r3 unchanged = 77
        prog = (
            BIS_lit(31, 2, 1)
            + BIS_lit(31, 99, 2)
            + BIS_lit(31, 77, 3)
            + operate(0x11, 1, 2, 0x14, 3)
            + HALT
        )
        assert regs(prog)[3] == 77

    def test_cmovlbc_taken(self):
        # CMOVLBC: move if Ra[0] == 0.  r1=4 (bit0=0) → r3 = 55
        prog = (
            BIS_lit(31, 4, 1)
            + BIS_lit(31, 55, 2)
            + BIS_lit(31, 0, 3)
            + operate(0x11, 1, 2, 0x16, 3)       # CMOVLBC r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 55

    def test_cmovlbc_not_taken(self):
        # r1=3 (bit0=1) → CMOVLBC not taken → r3 = 88
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 55, 2)
            + BIS_lit(31, 88, 3)
            + operate(0x11, 1, 2, 0x16, 3)
            + HALT
        )
        assert regs(prog)[3] == 88

    def test_cmoveq_taken(self):
        # CMOVEQ: move if Ra==0.  r1=0 → r3 = 42
        prog = (
            BIS_lit(31, 0, 1)
            + BIS_lit(31, 42, 2)
            + BIS_lit(31, 0, 3)
            + operate(0x11, 1, 2, 0x24, 3)       # CMOVEQ r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 42

    def test_cmoveq_not_taken(self):
        # r1=5 ≠ 0 → CMOVEQ not taken → r3 = 7
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 42, 2)
            + BIS_lit(31, 7, 3)
            + operate(0x11, 1, 2, 0x24, 3)
            + HALT
        )
        assert regs(prog)[3] == 7

    def test_cmovlt_taken(self):
        # CMOVLT: move if Ra < 0 (bit63=1). r1 has bit63 set via large value.
        # We can't load a negative number directly with 8-bit lit, but:
        # BIS r31, #1, r1; SLL r1, #63, r1 → r1 = 0x8000000000000000 (negative)
        prog = (
            BIS_lit(31, 1, 1)
            + SLL_lit(1, 63, 1)                  # r1 = 0x8000000000000000
            + BIS_lit(31, 99, 2)
            + BIS_lit(31, 0, 3)
            + operate(0x11, 1, 2, 0x44, 3)        # CMOVLT r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 99

    def test_cmovlt_not_taken(self):
        # r1 = 5 (positive) → CMOVLT not taken
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 99, 2)
            + BIS_lit(31, 7, 3)
            + operate(0x11, 1, 2, 0x44, 3)
            + HALT
        )
        assert regs(prog)[3] == 7

    def test_cmovge_taken(self):
        # CMOVGE: move if Ra >= 0 (bit63=0). r1=5 → taken
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 42, 2)
            + BIS_lit(31, 0, 3)
            + operate(0x11, 1, 2, 0x46, 3)        # CMOVGE r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 42

    def test_cmovge_not_taken(self):
        # r1 = negative (bit63=1) → CMOVGE not taken
        prog = (
            BIS_lit(31, 1, 1)
            + SLL_lit(1, 63, 1)
            + BIS_lit(31, 42, 2)
            + BIS_lit(31, 7, 3)
            + operate(0x11, 1, 2, 0x46, 3)
            + HALT
        )
        assert regs(prog)[3] == 7

    def test_cmovle_taken_zero(self):
        # CMOVLE: move if Ra <= 0. r1=0 → taken
        prog = (
            BIS_lit(31, 0, 1)
            + BIS_lit(31, 33, 2)
            + BIS_lit(31, 0, 3)
            + operate(0x11, 1, 2, 0x64, 3)        # CMOVLE r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 33

    def test_cmovle_taken_negative(self):
        # CMOVLE: move if Ra <= 0. r1 = negative → taken
        prog = (
            BIS_lit(31, 1, 1)
            + SLL_lit(1, 63, 1)
            + BIS_lit(31, 33, 2)
            + BIS_lit(31, 0, 3)
            + operate(0x11, 1, 2, 0x64, 3)
            + HALT
        )
        assert regs(prog)[3] == 33

    def test_cmovle_not_taken(self):
        # r1 = 1 (positive non-zero) → CMOVLE not taken
        prog = (
            BIS_lit(31, 1, 1)
            + BIS_lit(31, 33, 2)
            + BIS_lit(31, 7, 3)
            + operate(0x11, 1, 2, 0x64, 3)
            + HALT
        )
        assert regs(prog)[3] == 7

    def test_cmovgt_taken(self):
        # CMOVGT: move if Ra > 0. r1=1 → taken
        prog = (
            BIS_lit(31, 1, 1)
            + BIS_lit(31, 55, 2)
            + BIS_lit(31, 0, 3)
            + operate(0x11, 1, 2, 0x66, 3)        # CMOVGT r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 55

    def test_cmovgt_not_taken_zero(self):
        # r1=0 → CMOVGT not taken
        prog = (
            BIS_lit(31, 0, 1)
            + BIS_lit(31, 55, 2)
            + BIS_lit(31, 7, 3)
            + operate(0x11, 1, 2, 0x66, 3)
            + HALT
        )
        assert regs(prog)[3] == 7

    def test_cmovgt_not_taken_negative(self):
        # r1 = negative → CMOVGT not taken
        prog = (
            BIS_lit(31, 1, 1)
            + SLL_lit(1, 63, 1)
            + BIS_lit(31, 55, 2)
            + BIS_lit(31, 7, 3)
            + operate(0x11, 1, 2, 0x66, 3)
            + HALT
        )
        assert regs(prog)[3] == 7


# ── INTL: AMASK, IMPLVER, EQV ─────────────────────────────────────────────────

class TestAMASKIMPLVEREQV:

    def test_eqv(self):
        # EQV = XNOR: all equal bits → all 1s
        prog = (
            BIS_lit(31, 0b1010, 1)
            + BIS_lit(31, 0b1010, 2)
            + operate(0x11, 1, 2, 0x48, 3)        # EQV r1, r2, r3
            + HALT
        )
        # 0b1010 EQV 0b1010 in 64 bits = all-1s (same bits → all match)
        assert regs(prog)[3] == 0xFFFF_FFFF_FFFF_FFFF

    def test_amask(self):
        # AMASK Ra, Rb, Rc = BIC(Ra, Rb) = Ra & ~Rb
        prog = (
            BIS_lit(31, 0xFF, 1)
            + BIS_lit(31, 0x0F, 2)
            + operate(0x11, 1, 2, 0x61, 3)        # AMASK r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 0xF0

    def test_implver(self):
        # IMPLVER: always writes 0
        prog = (
            BIS_lit(31, 42, 1)
            + operate(0x11, 1, 31, 0x6C, 2)       # IMPLVER r1, r31, r2
            + HALT
        )
        assert regs(prog)[2] == 0


# ── INTA: Scaled add/sub ──────────────────────────────────────────────────────

class TestScaledAddSub:
    """S4ADDQ, S4ADDL, S4SUBQ, S4SUBL, S8ADDQ, S8ADDL, S8SUBQ, S8SUBL."""

    def test_s4addq(self):
        # S4ADDQ r1, r2, r3: r3 = (r1 << 2) + r2 = 4*3 + 10 = 22
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 10, 2)
            + operate(0x10, 1, 2, 0x22, 3)        # S4ADDQ
            + HALT
        )
        assert regs(prog)[3] == 22

    def test_s4addl(self):
        # S4ADDL: longword version: r3 = sext32(4*2 + 5) = 13
        prog = (
            BIS_lit(31, 2, 1)
            + BIS_lit(31, 5, 2)
            + operate(0x10, 1, 2, 0x02, 3)        # S4ADDL
            + HALT
        )
        assert regs(prog)[3] == 13

    def test_s4subq(self):
        # S4SUBQ: r3 = (r1 << 2) - r2 = 4*10 - 6 = 34
        prog = (
            BIS_lit(31, 10, 1)
            + BIS_lit(31, 6, 2)
            + operate(0x10, 1, 2, 0x2B, 3)        # S4SUBQ
            + HALT
        )
        assert regs(prog)[3] == 34

    def test_s4subl(self):
        # S4SUBL: sext32(4*3 - 2) = 10
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 2, 2)
            + operate(0x10, 1, 2, 0x0B, 3)        # S4SUBL
            + HALT
        )
        assert regs(prog)[3] == 10

    def test_s8addq(self):
        # S8ADDQ: r3 = (r1 << 3) + r2 = 8*4 + 7 = 39
        prog = (
            BIS_lit(31, 4, 1)
            + BIS_lit(31, 7, 2)
            + operate(0x10, 1, 2, 0x32, 3)        # S8ADDQ
            + HALT
        )
        assert regs(prog)[3] == 39

    def test_s8addl(self):
        # S8ADDL: sext32(8*2 + 1) = 17
        prog = (
            BIS_lit(31, 2, 1)
            + BIS_lit(31, 1, 2)
            + operate(0x10, 1, 2, 0x12, 3)        # S8ADDL
            + HALT
        )
        assert regs(prog)[3] == 17

    def test_s8subq(self):
        # S8SUBQ: r3 = 8*5 - 3 = 37
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 3, 2)
            + operate(0x10, 1, 2, 0x3B, 3)        # S8SUBQ
            + HALT
        )
        assert regs(prog)[3] == 37

    def test_s8subl(self):
        # S8SUBL: sext32(8*3 - 7) = 17
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 7, 2)
            + operate(0x10, 1, 2, 0x1B, 3)        # S8SUBL
            + HALT
        )
        assert regs(prog)[3] == 17


# ── INTA: CMPLE, CMPULE ───────────────────────────────────────────────────────

class TestCmple:

    def test_cmple_less(self):
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 5, 2)
            + operate(0x10, 1, 2, 0x6D, 3)       # CMPLE r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 1

    def test_cmple_equal(self):
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 5, 2)
            + operate(0x10, 1, 2, 0x6D, 3)
            + HALT
        )
        assert regs(prog)[3] == 1

    def test_cmple_greater(self):
        prog = (
            BIS_lit(31, 7, 1)
            + BIS_lit(31, 3, 2)
            + operate(0x10, 1, 2, 0x6D, 3)
            + HALT
        )
        assert regs(prog)[3] == 0

    def test_cmpule_less(self):
        prog = (
            BIS_lit(31, 1, 1)
            + BIS_lit(31, 5, 2)
            + operate(0x10, 1, 2, 0x5D, 3)       # CMPULE
            + HALT
        )
        assert regs(prog)[3] == 1

    def test_cmpule_equal(self):
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 5, 2)
            + operate(0x10, 1, 2, 0x5D, 3)
            + HALT
        )
        assert regs(prog)[3] == 1

    def test_cmpule_greater(self):
        prog = (
            BIS_lit(31, 9, 1)
            + BIS_lit(31, 3, 2)
            + operate(0x10, 1, 2, 0x5D, 3)
            + HALT
        )
        assert regs(prog)[3] == 0


# ── INTA: CMPBGE ──────────────────────────────────────────────────────────────

class TestCmpbge:

    def test_cmpbge_all_equal(self):
        # r1 = r2 = 0 → all bytes equal → all 8 bits set = 0xFF
        prog = (
            BIS_lit(31, 0, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x10, 1, 2, 0x4B, 3)       # CMPBGE
            + HALT
        )
        assert regs(prog)[3] == 0xFF

    def test_cmpbge_byte0_less(self):
        # r1 byte0 = 0, r2 byte0 = 1 → byte0: 0 >= 1 → false → bit0=0
        # all other bytes: 0 >= 0 → true
        prog = (
            BIS_lit(31, 0, 1)
            + BIS_lit(31, 1, 2)                  # r2 byte0 = 1
            + operate(0x10, 1, 2, 0x4B, 3)
            + HALT
        )
        assert regs(prog)[3] == 0xFE  # all except bit0

    def test_cmpbge_all_greater(self):
        # r1=5, r2=3: byte0 of r1 (=5) >= byte0 of r2 (=3) → bit0=1; rest 0>=0
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 3, 2)
            + operate(0x10, 1, 2, 0x4B, 3)
            + HALT
        )
        assert regs(prog)[3] == 0xFF


# ── INTM: MULL, UMULH ────────────────────────────────────────────────────────

class TestIntm:

    def test_mull_basic(self):
        # MULL: 32-bit multiply then sign-extend. 3 × 4 = 12
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 4, 2)
            + operate(0x13, 1, 2, 0x00, 3)       # MULL r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 12

    def test_umulh_basic(self):
        # UMULH: upper 64 bits of 64×64 unsigned multiply.
        # For small numbers result in upper half = 0.
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 6, 2)
            + operate(0x13, 1, 2, 0x30, 3)       # UMULH r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 0  # 5*6=30 < 2^64

    def test_umulh_large(self):
        # UMULH with 2^63 * 2 → upper 64 bits = 1
        # r1 = 2^63, r2 = 2 → product = 2^64 → upper 64 = 1
        prog = (
            BIS_lit(31, 1, 1)
            + SLL_lit(1, 63, 1)                  # r1 = 2^63
            + BIS_lit(31, 2, 2)
            + operate(0x13, 1, 2, 0x30, 3)       # UMULH
            + HALT
        )
        assert regs(prog)[3] == 1


# ── Memory: LDAH, LDBU, LDQ_U, STQ_U ─────────────────────────────────────────

class TestMemoryExtended:

    def test_ldah(self):
        # LDAH r1, 1(r31): r1 = r31 + sext16(1) * 65536 = 65536
        prog = (
            mem_op(0x09, 1, 31, 1)               # LDAH r1, 1(r31)
            + HALT
        )
        assert regs(prog)[1] == 65536

    def test_ldbu(self):
        # Store byte 0xAB via STQ then load via LDBU.
        prog = (
            BIS_lit(31, 0x10, 1)
            + SLL_lit(1, 8, 1)                   # r1 = 0x1000
            + BIS_lit(31, 0xAB, 2)               # r2 = 0xAB
            + mem_op(0x2D, 2, 1, 0)              # STQ r2, 0(r1) — store to 0x1000
            + mem_op(0x0A, 3, 1, 0)              # LDBU r3, 0(r1) — load byte
            + HALT
        )
        assert regs(prog)[3] == 0xAB

    def test_ldq_u(self):
        # LDQ_U: aligned load (ignores low 3 bits). Same as LDQ for aligned addr.
        prog = (
            BIS_lit(31, 0x10, 1)
            + SLL_lit(1, 8, 1)                   # r1 = 0x1000
            + BIS_lit(31, 0x77, 2)
            + mem_op(0x2D, 2, 1, 0)              # STQ r2, 0(r1)
            + mem_op(0x0B, 3, 1, 0)              # LDQ_U r3, 0(r1)
            + HALT
        )
        assert regs(prog)[3] == 0x77

    def test_stq_u_and_ldq_u(self):
        # STQ_U: aligned store (ignores low 3 bits).
        prog = (
            BIS_lit(31, 0x10, 1)
            + SLL_lit(1, 8, 1)                   # r1 = 0x1000
            + BIS_lit(31, 0x55, 2)
            + mem_op(0x0F, 2, 1, 0)              # STQ_U r2, 0(r1)
            + mem_op(0x0B, 3, 1, 0)              # LDQ_U r3, 0(r1)
            + HALT
        )
        assert regs(prog)[3] == 0x55

    def test_stl_c_succeeds(self):
        # STL_C: atomic store longword — always succeeds → Ra = 1
        prog = (
            BIS_lit(31, 0x10, 1)
            + SLL_lit(1, 8, 1)                   # r1 = 0x1000
            + BIS_lit(31, 42, 2)
            + mem_op(0x2E, 2, 1, 0)              # STL_C r2, 0(r1)
            + HALT
        )
        # STL_C writes 1 to Ra (r2) on success
        assert regs(prog)[2] == 1

    def test_stq_c_succeeds(self):
        # STQ_C: always succeeds → Ra = 1
        prog = (
            BIS_lit(31, 0x10, 1)
            + SLL_lit(1, 8, 1)
            + BIS_lit(31, 99, 2)
            + mem_op(0x2F, 2, 1, 0)              # STQ_C r2, 0(r1)
            + HALT
        )
        assert regs(prog)[2] == 1

    def test_unaligned_ldl_raises(self):
        # Try to load from an unaligned address (e.g., 0x1001, which is not %4)
        prog = (
            BIS_lit(31, 0x10, 1)
            + SLL_lit(1, 8, 1)                   # r1 = 0x1000
            + operate_lit(0x10, 1, 1, 0x20, 1)  # ADDQ r1, 1, r1 → 0x1001
            + mem_op(0x28, 2, 1, 0)              # LDL r2, 0(r1) — unaligned
            + HALT
        )
        result = run(prog)
        assert result.halted
        assert result.error is not None
        assert "Unaligned" in result.error

    def test_unaligned_stq_raises(self):
        # STQ at an unaligned address
        prog = (
            BIS_lit(31, 0x10, 1)
            + SLL_lit(1, 8, 1)
            + operate_lit(0x10, 1, 3, 0x20, 1)  # ADDQ r1, 3, r1 → 0x1003
            + BIS_lit(31, 42, 2)
            + mem_op(0x2D, 2, 1, 0)              # STQ r2, 0(r1) — unaligned
            + HALT
        )
        result = run(prog)
        assert result.halted
        assert result.error is not None
        assert "Unaligned" in result.error


# ── INTS: EXT/INS/MSK/SEXTB/SEXTW ────────────────────────────────────────────

class TestByteManipulation:
    """EXTBL, EXTWL, EXTLL, EXTQL, INSBL, INSWL, INSLL, INSQL,
    MSKBL, MSKWL, MSKLL, MSKQL, SEXTB, SEXTW."""

    def test_extbl(self):
        # EXTBL: extract byte at byte-offset Rb&7 from Ra (right-aligned).
        # r1 = 0xABCD, r2 = 1 (offset=1, boff=8 bits): result = 0xAB
        prog = (
            BIS_lit(31, 0xAB, 1)
            + SLL_lit(1, 8, 1)                   # r1 = 0xAB00
            + operate_lit(0x11, 31, 0xCD, 0x20, 4)  # r4 = 0xCD
            + operate(0x11, 1, 4, 0x20, 1)        # r1 = 0xABCD
            + BIS_lit(31, 1, 2)                  # r2 = 1 (byte offset 1)
            + operate(0x12, 1, 2, 0x06, 3)        # EXTBL r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 0xAB

    def test_extwl(self):
        # EXTWL: extract 16-bit word at byte-offset.
        # r1 = 0x1234ABCD, r2 = 0 (offset=0): result = 0xABCD (low 16 bits)
        prog = (
            BIS_lit(31, 0xAB, 1)
            + SLL_lit(1, 8, 1)
            + operate_lit(0x11, 31, 0xCD, 0x20, 4)
            + operate(0x11, 1, 4, 0x20, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x12, 1, 2, 0x16, 3)        # EXTWL r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 0xABCD

    def test_extll(self):
        # EXTLL: extract 32-bit longword at byte-offset 0.
        prog = (
            BIS_lit(31, 0x12, 1)
            + SLL_lit(1, 8, 1)
            + operate_lit(0x11, 31, 0x34, 0x20, 4)
            + operate(0x11, 1, 4, 0x20, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x12, 1, 2, 0x26, 3)        # EXTLL
            + HALT
        )
        assert regs(prog)[3] == 0x1234

    def test_extql(self):
        # EXTQL: extract 64-bit quad — same as full value when offset=0.
        prog = (
            BIS_lit(31, 42, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x12, 1, 2, 0x36, 3)        # EXTQL r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 42

    def test_insbl(self):
        # INSBL: insert byte (low 8 bits of Ra) at byte-offset Rb.
        # Ra = 0xAB, Rb = 1 → result = 0xAB00
        prog = (
            BIS_lit(31, 0xAB, 1)
            + BIS_lit(31, 1, 2)                  # offset 1
            + operate(0x12, 1, 2, 0x0B, 3)        # INSBL r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 0xAB00

    def test_inswl(self):
        # INSWL: insert low 16 bits at byte-offset 0 → same value
        prog = (
            BIS_lit(31, 0xCD, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x12, 1, 2, 0x1B, 3)        # INSWL r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 0xCD

    def test_insll(self):
        # INSLL: insert low 32 bits at byte-offset 0
        prog = (
            BIS_lit(31, 0x12, 1)
            + SLL_lit(1, 8, 1)
            + operate_lit(0x11, 31, 0x34, 0x20, 4)
            + operate(0x11, 1, 4, 0x20, 1)        # r1 = 0x1234
            + BIS_lit(31, 0, 2)
            + operate(0x12, 1, 2, 0x2B, 3)        # INSLL
            + HALT
        )
        assert regs(prog)[3] == 0x1234

    def test_insql(self):
        # INSQL: insert full 64 bits at byte-offset 0 → identity
        prog = (
            BIS_lit(31, 99, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x12, 1, 2, 0x3B, 3)        # INSQL
            + HALT
        )
        assert regs(prog)[3] == 99

    def test_mskbl(self):
        # MSKBL: zero out byte 0 (offset=0) of Ra.
        # r1 = 0x1234, byte0=0x34. MSKBL with offset=0 zeros byte0 → 0x1200.
        prog = (
            BIS_lit(31, 0x12, 1)
            + SLL_lit(1, 8, 1)
            + operate_lit(0x11, 31, 0x34, 0x20, 4)
            + operate(0x11, 1, 4, 0x20, 1)        # r1 = 0x1234
            + BIS_lit(31, 0, 2)                  # offset = 0
            + operate(0x12, 1, 2, 0x02, 3)        # MSKBL r1, r2, r3
            + HALT
        )
        assert regs(prog)[3] == 0x1200

    def test_mskwl(self):
        # MSKWL: zero out 16-bit word at offset 0 → low 16 bits zeroed.
        prog = (
            BIS_lit(31, 0x12, 1)
            + SLL_lit(1, 8, 1)
            + operate_lit(0x11, 31, 0x34, 0x20, 4)
            + operate(0x11, 1, 4, 0x20, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x12, 1, 2, 0x12, 3)        # MSKWL
            + HALT
        )
        assert regs(prog)[3] == 0  # 0x1234 & ~0xFFFF = 0

    def test_mskll(self):
        # MSKLL: zero out 32-bit longword at offset 0.
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x12, 1, 2, 0x22, 3)        # MSKLL
            + HALT
        )
        assert regs(prog)[3] == 0

    def test_mskql(self):
        # MSKQL: zero out 64-bit quad at offset 0 → 0.
        prog = (
            BIS_lit(31, 0xFF, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x12, 1, 2, 0x32, 3)        # MSKQL
            + HALT
        )
        assert regs(prog)[3] == 0

    def test_sextb_positive(self):
        # SEXTB: sign-extend byte. 0x7F (127) → stays 127
        prog = (
            BIS_lit(31, 0x7F, 1)
            + operate(0x12, 1, 31, 0x00, 2)       # SEXTB r1, r31, r2
            + HALT
        )
        assert regs(prog)[2] == 0x7F

    def test_sextb_negative(self):
        # SEXTB: 0x80 → sign-extend to 0xFFFFFFFFFFFFFF80
        prog = (
            BIS_lit(31, 0x80, 1)
            + operate(0x12, 1, 31, 0x00, 2)       # SEXTB
            + HALT
        )
        assert regs(prog)[2] == 0xFFFF_FFFF_FFFF_FF80

    def test_sextw_positive(self):
        # SEXTW: sign-extend 16-bit. 0x7F = 127 → 127
        prog = (
            BIS_lit(31, 0x7F, 1)
            + operate(0x12, 1, 31, 0x01, 2)       # SEXTW r1, r31, r2
            + HALT
        )
        assert regs(prog)[2] == 0x7F

    def test_sextw_negative(self):
        # SEXTW: 0x8000 → sign-extend to 0xFFFFFFFFFFFF8000
        # Build 0x8000: BIS r31, #1, r1; SLL r1, #15, r1
        prog = (
            BIS_lit(31, 1, 1)
            + SLL_lit(1, 15, 1)                  # r1 = 0x8000
            + operate(0x12, 1, 31, 0x01, 2)       # SEXTW
            + HALT
        )
        assert regs(prog)[2] == 0xFFFF_FFFF_FFFF_8000


# ── Branch variants: BLT, BLE, BGT, BGE, BLBC, BLBS ─────────────────────────

class TestBranchVariants:

    def test_blt_taken(self):
        # BLT r1, +1: r1 is negative (bit63=1) → branch taken → skip r2=42
        # r1 = 0x8000000000000000
        prog = (
            BIS_lit(31, 1, 1)
            + SLL_lit(1, 63, 1)                  # r1 = negative
            + branch_op(0x3A, 1, 1)              # BLT r1, +1 → skip next
            + BIS_lit(31, 42, 2)                 # r2 = 42 (SKIPPED)
            + HALT
        )
        assert regs(prog)[2] == 0

    def test_blt_not_taken(self):
        prog = (
            BIS_lit(31, 5, 1)                    # r1 = 5 (positive)
            + branch_op(0x3A, 1, 1)              # BLT r1, +1 → not taken
            + BIS_lit(31, 42, 2)                 # r2 = 42 (EXECUTED)
            + HALT
        )
        assert regs(prog)[2] == 42

    def test_ble_taken_zero(self):
        # BLE: branch if Ra <= 0. r1=0 → taken
        prog = (
            BIS_lit(31, 0, 1)
            + branch_op(0x3B, 1, 1)              # BLE r1, +1 → taken (skip)
            + BIS_lit(31, 77, 2)
            + HALT
        )
        assert regs(prog)[2] == 0

    def test_ble_taken_negative(self):
        prog = (
            BIS_lit(31, 1, 1)
            + SLL_lit(1, 63, 1)                  # r1 = negative
            + branch_op(0x3B, 1, 1)              # BLE r1, +1 → taken
            + BIS_lit(31, 77, 2)
            + HALT
        )
        assert regs(prog)[2] == 0

    def test_ble_not_taken(self):
        prog = (
            BIS_lit(31, 5, 1)
            + branch_op(0x3B, 1, 1)              # BLE r1, +1 → not taken
            + BIS_lit(31, 77, 2)
            + HALT
        )
        assert regs(prog)[2] == 77

    def test_bgt_taken(self):
        # BGT: branch if Ra > 0. r1=1 → taken
        prog = (
            BIS_lit(31, 1, 1)
            + branch_op(0x3F, 1, 1)              # BGT r1, +1 → taken
            + BIS_lit(31, 55, 2)
            + HALT
        )
        assert regs(prog)[2] == 0

    def test_bgt_not_taken_zero(self):
        prog = (
            BIS_lit(31, 0, 1)
            + branch_op(0x3F, 1, 1)
            + BIS_lit(31, 55, 2)
            + HALT
        )
        assert regs(prog)[2] == 55

    def test_bge_taken(self):
        # BGE: branch if Ra >= 0. r1=5 (positive) → taken (bit63=0)
        prog = (
            BIS_lit(31, 5, 1)
            + branch_op(0x3E, 1, 1)              # BGE → taken
            + BIS_lit(31, 88, 2)
            + HALT
        )
        assert regs(prog)[2] == 0

    def test_bge_not_taken(self):
        prog = (
            BIS_lit(31, 1, 1)
            + SLL_lit(1, 63, 1)                  # r1 = negative
            + branch_op(0x3E, 1, 1)
            + BIS_lit(31, 88, 2)
            + HALT
        )
        assert regs(prog)[2] == 88

    def test_blbc_taken(self):
        # BLBC: branch if Ra[0]==0. r1=2 (even) → taken
        prog = (
            BIS_lit(31, 2, 1)
            + branch_op(0x38, 1, 1)              # BLBC → taken
            + BIS_lit(31, 33, 2)
            + HALT
        )
        assert regs(prog)[2] == 0

    def test_blbc_not_taken(self):
        prog = (
            BIS_lit(31, 3, 1)                    # r1=3 (odd) → BLBC not taken
            + branch_op(0x38, 1, 1)
            + BIS_lit(31, 33, 2)
            + HALT
        )
        assert regs(prog)[2] == 33

    def test_blbs_taken(self):
        # BLBS: branch if Ra[0]==1. r1=1 (odd) → taken
        prog = (
            BIS_lit(31, 1, 1)
            + branch_op(0x3C, 1, 1)              # BLBS → taken
            + BIS_lit(31, 44, 2)
            + HALT
        )
        assert regs(prog)[2] == 0

    def test_blbs_not_taken(self):
        prog = (
            BIS_lit(31, 2, 1)                    # r1=2 (even) → BLBS not taken
            + branch_op(0x3C, 1, 1)
            + BIS_lit(31, 44, 2)
            + HALT
        )
        assert regs(prog)[2] == 44

    def test_fp_branch_nop(self):
        # Floating-point branches (0x31–0x37, except 0x34) are treated as NOP.
        # FBEQ = 0x31: not taken, so next instruction executes.
        prog = (
            BIS_lit(31, 99, 1)
            + branch_op(0x31, 1, 1)              # FBEQ (FP branch, NOP)
            + BIS_lit(31, 42, 2)                 # r2 = 42 (should execute)
            + HALT
        )
        assert regs(prog)[2] == 42


# ── Additional LDA test (gate-level only, not behavioral) ─────────────────────

class TestLDA:
    """LDA and LDAH — address computation tests (gate-level only)."""

    def test_lda_simple(self):
        prog = (
            BIS_lit(31, 0, 1)
            + mem_op(0x08, 2, 1, 100)            # LDA r2, 100(r1) → r2 = 100
            + HALT
        )
        assert regs(prog)[2] == 100

    def test_lda_negative_offset(self):
        prog = (
            operate_lit(0x11, 31, 100, 0x20, 1)  # r1 = 100
            + mem_op(0x08, 2, 1, 0xFFFC)         # LDA r2, -4(r1) → r2 = 96
            + HALT
        )
        assert regs(prog)[2] == 96

    def test_ldah_shifts_16(self):
        # LDAH r2, 2(r31): r2 = 0 + 2 * 65536 = 131072
        prog = (
            mem_op(0x09, 2, 31, 2)               # LDAH r2, 2(r31)
            + HALT
        )
        assert regs(prog)[2] == 131072


# ── JMP/JSR mnemonic variants ─────────────────────────────────────────────────

class TestJumpVariants:

    def test_jmp_func0(self):
        # JMP: func=0. Set r1=0x08, JMP r31, (r1) → PC=0x08
        prog = (
            mem_op(0x08, 1, 31, 8)               # LDA r1, 8(r31) → r1=8
            + jump_op(31, 1, 0)                  # JMP r31,(r1)  func=0
            + BIS_lit(31, 77, 4)                 # SKIPPED
            + BIS_lit(31, 99, 5)                 # at 0x08: r5=99
            + HALT
        )
        assert regs(prog)[5] == 99

    def test_jsr_func1(self):
        # JSR: func=1. r26 = PC+4 (link). Jump to r1.
        # offset 0x00: LDA r1, 0x08(r31)
        # offset 0x04: JSR r26,(r1)  → r26=0x08, PC=0x08
        # offset 0x08: HALT
        prog = (
            mem_op(0x08, 1, 31, 8)               # r1 = 8
            + jump_op(26, 1, 1)                  # JSR r26,(r1) func=1
            + HALT                               # offset 0x08 = target
        )
        result = run(prog)
        assert result.ok
        assert result.final_state.regs[26] == 8  # link = offset 0x08

    def test_jsr_coroutine_func3(self):
        # JSR_COROUTINE: func=3. Same mechanics as JMP.
        prog = (
            mem_op(0x08, 1, 31, 8)
            + jump_op(31, 1, 3)                  # JSR_COROUTINE func=3
            + BIS_lit(31, 0, 4)                  # SKIPPED
            + BIS_lit(31, 5, 5)                  # at 0x08: r5=5
            + HALT
        )
        assert regs(prog)[5] == 5


# ── INTA alternate/overflow func codes ───────────────────────────────────────

class TestIntaAlternate:
    """Cover ADDLV (0x40), SUBLV (0x49), ADDQV (0x60), SUBQV (0x69),
    MULL (0x18), MULLV (0x58), MULQ (0x38), MULQV (0x78) paths."""

    def test_addlv_func40(self):
        # ADDLV func=0x40 — same as ADDL but overflow-trap variant.
        # 2 + 3 = 5 (no trap in our model)
        prog = (
            BIS_lit(31, 2, 1)
            + BIS_lit(31, 3, 2)
            + operate(0x10, 1, 2, 0x40, 3)        # ADDLV
            + HALT
        )
        assert regs(prog)[3] == 5

    def test_sublv_func49(self):
        # SUBLV func=0x49 — same as SUBL
        prog = (
            BIS_lit(31, 10, 1)
            + BIS_lit(31, 4, 2)
            + operate(0x10, 1, 2, 0x49, 3)        # SUBLV
            + HALT
        )
        assert regs(prog)[3] == 6

    def test_addqv_func60(self):
        # ADDQV func=0x60 — same as ADDQ
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 7, 2)
            + operate(0x10, 1, 2, 0x60, 3)        # ADDQV
            + HALT
        )
        assert regs(prog)[3] == 12

    def test_subqv_func69(self):
        # SUBQV func=0x69 — same as SUBQ
        prog = (
            BIS_lit(31, 20, 1)
            + BIS_lit(31, 8, 2)
            + operate(0x10, 1, 2, 0x69, 3)        # SUBQV
            + HALT
        )
        assert regs(prog)[3] == 12

    def test_mull_via_inta(self):
        # MULL func=0x18 in INTA group (alternate dispatch)
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 4, 2)
            + operate(0x10, 1, 2, 0x18, 3)        # MULL via INTA
            + HALT
        )
        assert regs(prog)[3] == 12

    def test_mullv_via_inta(self):
        # MULLV func=0x58
        prog = (
            BIS_lit(31, 2, 1)
            + BIS_lit(31, 5, 2)
            + operate(0x10, 1, 2, 0x58, 3)        # MULLV
            + HALT
        )
        assert regs(prog)[3] == 10

    def test_mulq_via_inta(self):
        # MULQ func=0x38 in INTA group
        prog = (
            BIS_lit(31, 6, 1)
            + BIS_lit(31, 7, 2)
            + operate(0x10, 1, 2, 0x38, 3)        # MULQ via INTA
            + HALT
        )
        assert regs(prog)[3] == 42

    def test_mulqv_via_inta(self):
        # MULQV func=0x78
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 3, 2)
            + operate(0x10, 1, 2, 0x78, 3)        # MULQV
            + HALT
        )
        assert regs(prog)[3] == 9

    def test_cmpeq_via_inta(self):
        # CMPEQ func=0x2D — equal values
        prog = (
            BIS_lit(31, 9, 1)
            + BIS_lit(31, 9, 2)
            + operate(0x10, 1, 2, 0x2D, 3)        # CMPEQ
            + HALT
        )
        assert regs(prog)[3] == 1

    def test_cmplt_via_inta(self):
        # CMPLT func=0x4D
        prog = (
            BIS_lit(31, 3, 1)
            + BIS_lit(31, 9, 2)
            + operate(0x10, 1, 2, 0x4D, 3)        # CMPLT r1<r2 → 1
            + HALT
        )
        assert regs(prog)[3] == 1

    def test_cmple_via_inta(self):
        # CMPLE func=0x6D
        prog = (
            BIS_lit(31, 5, 1)
            + BIS_lit(31, 5, 2)
            + operate(0x10, 1, 2, 0x6D, 3)        # CMPLE r1<=r2 → 1
            + HALT
        )
        assert regs(prog)[3] == 1

    def test_cmpult_via_inta(self):
        # CMPULT func=0x3D
        prog = (
            BIS_lit(31, 2, 1)
            + BIS_lit(31, 8, 2)
            + operate(0x10, 1, 2, 0x3D, 3)        # CMPULT r1<r2 (unsigned) → 1
            + HALT
        )
        assert regs(prog)[3] == 1


# ── INTL: register-form AND, BIC, BIS, ORNOT, XOR, EQV ──────────────────────

class TestIntlRegForm:
    """Test INTL operations using register form (i_bit=0) to cover lines
    that use operate() rather than operate_lit()."""

    def test_and_reg(self):
        prog = (
            BIS_lit(31, 0b1111, 1)
            + BIS_lit(31, 0b1010, 2)
            + operate(0x11, 1, 2, 0x00, 3)        # AND reg form
            + HALT
        )
        assert regs(prog)[3] == 0b1010

    def test_bic_reg(self):
        prog = (
            BIS_lit(31, 0xFF, 1)
            + BIS_lit(31, 0x0F, 2)
            + operate(0x11, 1, 2, 0x08, 3)        # BIC reg form
            + HALT
        )
        assert regs(prog)[3] == 0xF0

    def test_bis_reg(self):
        prog = (
            BIS_lit(31, 0b0011, 1)
            + BIS_lit(31, 0b1100, 2)
            + operate(0x11, 1, 2, 0x20, 3)        # BIS reg form
            + HALT
        )
        assert regs(prog)[3] == 0b1111

    def test_ornot_reg(self):
        # ORNOT: r3 = r1 | ~r2 = 0 | ~0 = all-1s
        prog = (
            BIS_lit(31, 0, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x11, 1, 2, 0x28, 3)        # ORNOT reg form
            + HALT
        )
        assert regs(prog)[3] == 0xFFFF_FFFF_FFFF_FFFF

    def test_xor_reg(self):
        prog = (
            BIS_lit(31, 0b1010, 1)
            + BIS_lit(31, 0b1100, 2)
            + operate(0x11, 1, 2, 0x40, 3)        # XOR reg form
            + HALT
        )
        assert regs(prog)[3] == 0b0110

    def test_eqv_reg(self):
        # EQV = XNOR: 0 EQV 0 = all-1s
        prog = (
            BIS_lit(31, 0, 1)
            + BIS_lit(31, 0, 2)
            + operate(0x11, 1, 2, 0x48, 3)        # EQV reg form
            + HALT
        )
        assert regs(prog)[3] == 0xFFFF_FFFF_FFFF_FFFF


# ── INTS: SRA via register form (func=0x3A) ───────────────────────────────────

class TestIntsSRARegForm:

    def test_sra_register_form(self):
        # SRA r1, r2, r3 where r2=2 (shift by 2 bits). r1=8 → r3=2.
        prog = (
            BIS_lit(31, 8, 1)
            + BIS_lit(31, 2, 2)
            + operate(0x12, 1, 2, 0x3A, 3)        # SRA reg form (func=0x3A)
            + HALT
        )
        assert regs(prog)[3] == 2

    def test_sra_alternate_3c_register(self):
        # SRA alternate encoding (func=0x3C) via register form
        prog = (
            BIS_lit(31, 16, 1)
            + BIS_lit(31, 2, 2)
            + operate(0x12, 1, 2, 0x3C, 3)        # SRA alternate func=0x3C
            + HALT
        )
        assert regs(prog)[3] == 4
