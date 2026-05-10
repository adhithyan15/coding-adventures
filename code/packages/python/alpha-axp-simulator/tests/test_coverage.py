"""Edge-case and coverage tests for AlphaSimulator.

Focuses on corners not covered by the per-instruction tests:
  - r31 hardwired zero (reads 0, writes discarded)
  - Little-endian byte layout in memory
  - ADDL sign-extension boundary
  - CMOV false-branch preserves Rc exactly
  - SRA sign-fill
  - max_steps termination
  - Unknown opcode / PALcode raises cleanly
  - Memory alignment errors
  - execute() resets before running
  - Operate literal is zero-extended (not sign-extended)
"""

from __future__ import annotations

import struct

import pytest

from alpha_axp_simulator import AlphaSimulator


def w32(v: int) -> bytes:
    return struct.pack("<I", v & 0xFFFF_FFFF)


HALT = w32(0x0000_0000)


def mov_i(rd: int, imm8: int) -> bytes:
    """BIS r31, imm8, rd."""
    return w32((0x11 << 26) | (31 << 21) | ((imm8 & 0xFF) << 13) | (1 << 12) | (0x20 << 5) | rd)


def opa_op(op: int, func: int, ra: int, rb: int, rc: int) -> bytes:
    return w32((op << 26) | (ra << 21) | (rb << 16) | (func << 5) | rc)


def mem_op(op: int, ra: int, rb: int, disp: int) -> bytes:
    return w32((op << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF))


def branch(op: int, ra: int, disp21: int) -> bytes:
    return w32((op << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


# ── r31 hardwired zero ────────────────────────────────────────────────────────

class TestR31HardwiredZero:

    def test_read_r31_always_zero(self):
        sim = AlphaSimulator()
        sim._regs[31] = 0xDEAD_BEEF   # force a non-zero value internally
        assert sim._get_reg(31) == 0   # protocol read returns 0

    def test_write_r31_discarded(self):
        sim = AlphaSimulator()
        sim._set_reg(31, 0xDEAD_BEEF)
        assert sim._regs[31] == 0   # write was discarded

    def test_addq_to_r31_discarded(self):
        # ADDQ r1, r2, r31 — result discarded
        prog  = mov_i(1, 5)
        prog += mov_i(2, 3)
        prog += opa_op(0x10, 0x20, 1, 2, 31)   # ADDQ r1, r2, r31
        prog += HALT
        result = AlphaSimulator().execute(prog)
        assert result.final_state.regs[31] == 0

    def test_mov_imm_to_r31_discarded(self):
        result = AlphaSimulator().execute(mov_i(31, 42) + HALT)
        assert result.final_state.regs[31] == 0

    def test_r31_as_source_is_zero(self):
        # BIS r31, r31, r1 → r1 = 0 | 0 = 0
        prog = opa_op(0x11, 0x20, 31, 31, 1) + HALT
        result = AlphaSimulator().execute(prog)
        assert result.final_state.regs[1] == 0


# ── Little-endian byte layout ─────────────────────────────────────────────────

class TestLittleEndian:

    def test_instruction_bytes_little_endian(self):
        """Instruction word 0x43FF_FFFF stored as FF FF FF 43 in memory."""
        sim = AlphaSimulator()
        # Pick a recognisable word — HALT=0x00000000 isn't interesting here.
        # Use NOP = (0x11 << 26) | (31 << 21) | (31 << 16) | (0x20 << 5) | 31
        nop_word = (0x11 << 26) | (31 << 21) | (31 << 16) | (0x20 << 5) | 31
        sim.load(struct.pack("<I", nop_word) + HALT)
        # Byte 0 = LSB of nop_word
        assert sim._mem[0] == nop_word & 0xFF
        assert sim._mem[1] == (nop_word >> 8) & 0xFF
        assert sim._mem[2] == (nop_word >> 16) & 0xFF
        assert sim._mem[3] == (nop_word >> 24) & 0xFF

    def test_stq_little_endian_layout(self):
        """STQ stores bytes in little-endian order."""
        sim = AlphaSimulator()
        prog = mem_op(0x2D, 1, 2, 0) + HALT   # STQ r1, 0(r2)
        sim.load(prog)
        sim._regs[1] = 0x0102_0304_0506_0708
        sim._regs[2] = 0x100
        while not sim._halted:
            sim.step()
        assert sim._mem[0x100] == 0x08   # least significant byte first
        assert sim._mem[0x101] == 0x07
        assert sim._mem[0x102] == 0x06
        assert sim._mem[0x103] == 0x05
        assert sim._mem[0x104] == 0x04
        assert sim._mem[0x105] == 0x03
        assert sim._mem[0x106] == 0x02
        assert sim._mem[0x107] == 0x01

    def test_ldq_little_endian_reassembly(self):
        """LDQ reassembles bytes from little-endian order."""
        sim = AlphaSimulator()
        prog = mem_op(0x29, 3, 1, 0) + HALT   # LDQ r3, 0(r1)
        sim.load(prog)
        sim._regs[1] = 0x200
        # Write 0xDEADBEEF_CAFEBABE little-endian
        val = 0xDEAD_BEEF_CAFE_BABE
        for i in range(8):
            sim._mem[0x200 + i] = (val >> (i * 8)) & 0xFF
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == val


# ── ADDL sign-extension boundary ─────────────────────────────────────────────

class TestADDLSignExtension:

    def test_addl_positive_no_extension(self):
        """ADDL result fits in 31 bits → no sign extension."""
        sim = AlphaSimulator()
        prog = opa_op(0x10, 0x00, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 1
        sim._regs[2] = 1
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 2

    def test_addl_overflow_sign_extended(self):
        """ADDL 0x7FFFFFFF + 1 = 0x80000000 → sext = 0xFFFFFFFF80000000."""
        sim = AlphaSimulator()
        prog = opa_op(0x10, 0x00, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0x7FFF_FFFF
        sim._regs[2] = 1
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFFFF_FFFF_8000_0000

    def test_addl_all_ones_plus_one(self):
        """ADDL 0xFFFFFFFF + 1 = 0x100000000 → low 32 = 0 → sext = 0."""
        sim = AlphaSimulator()
        prog = opa_op(0x10, 0x00, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0xFFFF_FFFF
        sim._regs[2] = 1
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0   # low 32 = 0, sext(0) = 0

    def test_subl_zero_minus_one(self):
        """SUBL 0 - 1 = 0xFFFFFFFF → sext = 0xFFFFFFFFFFFFFFFF."""
        sim = AlphaSimulator()
        prog = opa_op(0x10, 0x09, 31, 2, 3) + HALT
        sim.load(prog)
        sim._regs[2] = 1
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFFFF_FFFF_FFFF_FFFF


# ── CMOV false-branch preserves Rc ───────────────────────────────────────────

class TestCMOVFalseBranch:

    def _cmov_false(self, func: int, ra_val: int) -> int:
        """Run CMOV with condition false; Rc starts at 0xABCD; should stay."""
        sim = AlphaSimulator()
        prog  = opa_op(0x11, func, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = ra_val
        sim._regs[2] = 0xDEAD
        sim._regs[3] = 0xABCD   # initial Rc
        while not sim._halted:
            sim.step()
        return sim._get_reg(3)

    def test_cmoveq_false_preserves_rc(self):
        # Ra=5 (non-zero) → CMOVEQ false
        assert self._cmov_false(0x24, 5) == 0xABCD

    def test_cmovne_false_preserves_rc(self):
        # Ra=0 → CMOVNE false
        assert self._cmov_false(0x26, 0) == 0xABCD

    def test_cmovlt_false_preserves_rc(self):
        # Ra=1 (positive) → CMOVLT false
        assert self._cmov_false(0x44, 1) == 0xABCD

    def test_cmovge_false_preserves_rc(self):
        # Ra=0xFFFF_FFFF_FFFF_FFFF (-1 signed) → CMOVGE false
        assert self._cmov_false(0x46, 0xFFFF_FFFF_FFFF_FFFF) == 0xABCD

    def test_cmovlbs_false_preserves_rc(self):
        # Ra=4 (even) → CMOVLBS false
        assert self._cmov_false(0x14, 4) == 0xABCD

    def test_cmovlbc_false_preserves_rc(self):
        # Ra=3 (odd) → CMOVLBC false
        assert self._cmov_false(0x16, 3) == 0xABCD


# ── Operate literal is zero-extended ─────────────────────────────────────────

class TestOperateLiteral:

    def test_literal_255_zero_extended(self):
        """Literal 0xFF = 255 (not -1 — it's zero-extended, not sign-extended)."""
        # ADDQ r31, #255, r1 → r1 = 0 + 255 = 255
        prog = w32((0x10 << 26) | (31 << 21) | (0xFF << 13) | (1 << 12) | (0x20 << 5) | 1) + HALT
        result = AlphaSimulator().execute(prog)
        assert result.final_state.regs[1] == 255   # NOT 0xFFFFFFFFFFFFFFFF

    def test_literal_max_is_255(self):
        """8-bit literal can represent at most 255."""
        prog = w32((0x10 << 26) | (31 << 21) | (0xFF << 13) | (1 << 12) | (0x20 << 5) | 1) + HALT
        result = AlphaSimulator().execute(prog)
        assert result.final_state.regs[1] == 255


# ── Error paths ───────────────────────────────────────────────────────────────

class TestErrorPaths:

    def test_unknown_opcode_halts_with_error(self):
        """An unknown opcode stops execution with an ERROR trace."""
        # Opcode 0x01 is unused on Alpha
        bad_instr = w32(0x01 << 26)
        result = AlphaSimulator().execute(bad_instr + HALT)
        assert not result.ok
        last_trace = result.traces[-1]
        assert last_trace.mnemonic.startswith("ERROR")

    def test_unknown_palcode_halts_with_error(self):
        """call_pal with non-zero palcode is unsupported."""
        # PALcode 0x1 (not HALT)
        bad_pal = w32(0x0000_0001)   # op=0, palcode=1
        result = AlphaSimulator().execute(bad_pal + HALT)
        assert not result.ok

    def test_unaligned_stq_halts_with_error(self):
        sim = AlphaSimulator()
        prog = mem_op(0x2D, 1, 2, 0) + HALT
        sim.load(prog)
        sim._regs[1] = 0xABCD
        sim._regs[2] = 0x101   # unaligned
        trace = sim.step()
        assert "Unaligned" in trace.mnemonic

    def test_unaligned_ldl_halts_with_error(self):
        sim = AlphaSimulator()
        prog = mem_op(0x28, 3, 1, 0) + HALT
        sim.load(prog)
        sim._regs[1] = 0x003   # unaligned (not 4-byte aligned)
        trace = sim.step()
        assert "Unaligned" in trace.mnemonic

    def test_unknown_inta_func(self):
        """Unknown INTA function code → ERROR."""
        bad = w32((0x10 << 26) | (31 << 21) | (31 << 16) | (0x99 << 5) | 1)
        result = AlphaSimulator().execute(bad + HALT)
        assert not result.ok

    def test_unknown_intl_func(self):
        bad = w32((0x11 << 26) | (31 << 21) | (31 << 16) | (0x99 << 5) | 1)
        result = AlphaSimulator().execute(bad + HALT)
        assert not result.ok

    def test_unknown_ints_func(self):
        bad = w32((0x12 << 26) | (31 << 21) | (31 << 16) | (0x99 << 5) | 1)
        result = AlphaSimulator().execute(bad + HALT)
        assert not result.ok


# ── max_steps termination ─────────────────────────────────────────────────────

class TestMaxSteps:

    def test_max_steps_exceeded(self):
        """BR self-loop never halts; max_steps terminates cleanly."""
        loop = w32((0x30 << 26) | (31 << 21) | 0x1F_FFFF)   # BR r31, -1
        result = AlphaSimulator().execute(loop + HALT, max_steps=7)
        assert not result.ok
        assert result.steps == 7
        assert "max_steps" in result.error

    def test_max_steps_1_on_nop_then_halt(self):
        """max_steps=1 stops before HALT (only NOP executes)."""
        nop = w32((0x11 << 26) | (31 << 21) | (31 << 16) | (0x20 << 5) | 31)
        result = AlphaSimulator().execute(nop + HALT, max_steps=1)
        assert not result.ok
        assert result.steps == 1


# ── execute() resets state ────────────────────────────────────────────────────

class TestExecuteResets:

    def test_dirty_regs_cleared(self):
        sim = AlphaSimulator()
        sim._regs[7]  = 0xDEAD
        sim._regs[15] = 0xBEEF
        result = sim.execute(HALT)
        assert result.final_state.regs[7]  == 0
        assert result.final_state.regs[15] == 0

    def test_dirty_memory_cleared(self):
        sim = AlphaSimulator()
        sim._mem[0x500] = 0xAB
        result = sim.execute(HALT)
        assert result.final_state.memory[0x500] == 0

    def test_halted_flag_cleared_before_run(self):
        sim = AlphaSimulator()
        sim._halted = True
        result = sim.execute(HALT)
        # After execute the CPU is halted normally (by HALT instruction)
        assert result.ok
        assert result.halted


# ── SRA sign behaviour ────────────────────────────────────────────────────────

class TestSRABehaviour:

    def test_sra_positive_fills_with_zeros(self):
        sim = AlphaSimulator()
        prog = opa_op(0x12, 0x3C, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0x0000_FFFF_0000_0000
        sim._regs[2] = 16
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0x0000_0000_FFFF_0000

    def test_sra_negative_fills_with_ones(self):
        sim = AlphaSimulator()
        prog = opa_op(0x12, 0x3C, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0xFFFF_0000_0000_0000   # negative (high bit set)
        sim._regs[2] = 16
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFFFF_FFFF_0000_0000


# ── AlphaState immutability ───────────────────────────────────────────────────

class TestAlphaStateImmutability:

    def test_state_regs_is_tuple(self):
        sim = AlphaSimulator()
        state = sim.get_state()
        assert isinstance(state.regs, tuple)
        assert len(state.regs) == 32

    def test_state_memory_is_tuple(self):
        sim = AlphaSimulator()
        state = sim.get_state()
        assert isinstance(state.memory, tuple)
        assert len(state.memory) == 65536

    def test_state_frozen(self):
        import dataclasses
        sim = AlphaSimulator()
        state = sim.get_state()
        with pytest.raises((dataclasses.FrozenInstanceError, TypeError)):
            state.pc = 99  # type: ignore[misc]

    def test_mutating_sim_does_not_change_snapshot(self):
        sim = AlphaSimulator()
        before = sim.get_state()
        sim._regs[0] = 0xDEAD
        after = sim.get_state()
        assert before.regs[0] == 0
        assert after.regs[0] == 0xDEAD

    def test_state_convenience_properties(self):
        sim = AlphaSimulator()
        sim._regs[26] = 0x1234   # r26 = ra
        sim._regs[30] = 0x5678   # r30 = sp
        state = sim.get_state()
        assert state.ra == 0x1234
        assert state.sp == 0x5678
        assert state.zero == 0
        assert state.r31 == 0
