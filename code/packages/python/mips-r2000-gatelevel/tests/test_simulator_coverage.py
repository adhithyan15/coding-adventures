"""Additional tests to improve simulator.py coverage.

Covers: LWL/LWR/SWL/SWR, BLEZ/BGTZ, BLTZAL/BGEZAL, ADDI overflow,
MFHI/MTHI/MFLO/MTLO, BREAK, unknown opcode/funct, I/O port stubs,
SLLV/SRLV/SRAV, SLTI/SLTIU, XORI, LH/SH/LB/SB, BLEZ/BGTZ edge cases.
"""

import struct

import pytest

from mips_r2000_gatelevel import MIPSR2000GateLevelSimulator


def w(word: int) -> bytes:
    return struct.pack(">I", word)


HALT = w(0x0000_000C)


def r_instr(rs: int, rt: int, rd: int, shamt: int, funct: int) -> bytes:
    return w((0 << 26) | (rs << 21) | (rt << 16) | (rd << 11) | (shamt << 6) | funct)


def i_instr(op: int, rs: int, rt: int, imm: int) -> bytes:
    return w((op << 26) | (rs << 21) | (rt << 16) | (imm & 0xFFFF))


def run(prog: bytes, max_steps: int = 10_000):
    sim = MIPSR2000GateLevelSimulator()
    result = sim.execute(prog, max_steps=max_steps)
    return result


def run_state(prog: bytes):
    return run(prog).final_state


# ── BLEZ / BGTZ ───────────────────────────────────────────────────────────────


class TestBLEZBGTZ:
    def test_blez_taken_negative(self):
        # $t0 < 0: BLEZ taken
        prog = (
            i_instr(0x0F, 0, 8, 0xFFFF)   # LUI $t0, 0xFFFF → negative
            + i_instr(0x06, 8, 0, 1)      # BLEZ $t0, +1
            + i_instr(0x09, 0, 2, 99)     # skipped
            + i_instr(0x09, 0, 2, 1)
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 1

    def test_blez_taken_zero(self):
        # $t0 = 0: BLEZ taken
        prog = (
            i_instr(0x06, 0, 0, 1)        # BLEZ $zero, +1
            + i_instr(0x09, 0, 2, 99)     # skipped
            + i_instr(0x09, 0, 2, 2)
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 2

    def test_blez_not_taken_positive(self):
        # $t0 > 0: BLEZ not taken
        prog = (
            i_instr(0x09, 0, 8, 1)        # $t0 = 1
            + i_instr(0x06, 8, 0, 5)      # BLEZ $t0, +5 (not taken)
            + i_instr(0x09, 0, 2, 7)
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 7

    def test_bgtz_taken_positive(self):
        prog = (
            i_instr(0x09, 0, 8, 5)        # $t0 = 5
            + i_instr(0x07, 8, 0, 1)      # BGTZ $t0, +1 (taken)
            + i_instr(0x09, 0, 2, 99)     # skipped
            + i_instr(0x09, 0, 2, 3)
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 3

    def test_bgtz_not_taken_zero(self):
        prog = (
            i_instr(0x07, 0, 0, 5)        # BGTZ $zero, +5 (not taken)
            + i_instr(0x09, 0, 2, 4)
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 4

    def test_bgtz_not_taken_negative(self):
        prog = (
            i_instr(0x0F, 0, 8, 0xFFFF)   # LUI $t0, negative
            + i_instr(0x07, 8, 0, 5)      # BGTZ $t0, +5 (not taken)
            + i_instr(0x09, 0, 2, 5)
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 5


# ── BLTZAL / BGEZAL ───────────────────────────────────────────────────────────


class TestBLTZALBGEZAL:
    def test_bltzal_taken(self):
        # $t0 < 0: branch + link
        # 0: LUI $t0, 0xFFFF (negative)
        # 4: BLTZAL $t0, +1 → branch to PC(8)+4=12; $ra=8
        # 8: ADDIU $v0, $zero, 99 (skipped by branch)
        # 12: ADDIU $v0, $zero, 1 ← land here
        # 16: SYSCALL
        prog = (
            i_instr(0x0F, 0, 8, 0xFFFF)   # $t0 = negative
            + i_instr(0x01, 8, 0x10, 1)   # BLTZAL $t0, +1 → skip to byte 12
            + i_instr(0x09, 0, 2, 99)     # skipped
            + i_instr(0x09, 0, 2, 1)      # $v0 = 1
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 1  # 99 was skipped
        assert st.regs[31] == 8  # $ra = byte 8 (instruction after BLTZAL)

    def test_bltzal_not_taken(self):
        # $t0 >= 0: no branch, but $ra is still set
        prog = (
            i_instr(0x09, 0, 8, 5)        # $t0 = 5 (positive)
            + i_instr(0x01, 8, 0x10, 5)   # BLTZAL (not taken)
            + i_instr(0x09, 0, 2, 42)     # executed
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 42
        assert st.regs[31] != 0  # $ra was set

    def test_bgezal_taken(self):
        prog = (
            i_instr(0x09, 0, 8, 5)        # $t0 = 5 (positive >= 0)
            + i_instr(0x01, 8, 0x11, 1)   # BGEZAL $t0, +1 → skip to byte 12
            + i_instr(0x09, 0, 2, 99)     # skipped
            + i_instr(0x09, 0, 2, 2)      # $v0 = 2
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 2  # 99 was skipped
        assert st.regs[31] == 8  # $ra = byte 8

    def test_bgezal_not_taken(self):
        prog = (
            i_instr(0x0F, 0, 8, 0xFFFF)   # $t0 negative
            + i_instr(0x01, 8, 0x11, 5)   # BGEZAL (not taken)
            + i_instr(0x09, 0, 2, 7)      # executed
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 7
        assert st.regs[31] != 0  # $ra still set


# ── MFHI/MTHI/MFLO/MTLO ───────────────────────────────────────────────────────


class TestHILOMoves:
    def test_mthi_mfhi(self):
        prog = (
            i_instr(0x09, 0, 8, 42)      # $t0 = 42
            + r_instr(8, 0, 0, 0, 0x11)  # MTHI $t0
            + r_instr(0, 0, 2, 0, 0x10)  # MFHI $v0
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 42
        assert st.hi == 42

    def test_mtlo_mflo(self):
        prog = (
            i_instr(0x09, 0, 8, 99)      # $t0 = 99
            + r_instr(8, 0, 0, 0, 0x13)  # MTLO $t0
            + r_instr(0, 0, 2, 0, 0x12)  # MFLO $v0
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 99
        assert st.lo == 99


# ── SLTI / SLTIU / XORI ───────────────────────────────────────────────────────


class TestITypeArith:
    def test_slti_less(self):
        prog = (
            i_instr(0x09, 0, 8, 5)
            + i_instr(0x0A, 8, 2, 10)  # SLTI $v0, $t0, 10
            + HALT
        )
        assert run_state(prog).regs[2] == 1

    def test_slti_not_less(self):
        prog = (
            i_instr(0x09, 0, 8, 10)
            + i_instr(0x0A, 8, 2, 5)   # SLTI $v0, $t0, 5 → 0
            + HALT
        )
        assert run_state(prog).regs[2] == 0

    def test_sltiu_basic(self):
        prog = (
            i_instr(0x09, 0, 8, 3)
            + i_instr(0x0B, 8, 2, 10)  # SLTIU $v0, $t0, 10
            + HALT
        )
        assert run_state(prog).regs[2] == 1

    def test_xori(self):
        prog = (
            i_instr(0x09, 0, 8, 0xFF)
            + i_instr(0x0E, 8, 2, 0x0F)  # XORI $v0, $t0, 0x0F
            + HALT
        )
        assert run_state(prog).regs[2] == 0xF0

    def test_addi_no_overflow(self):
        prog = (
            i_instr(0x09, 0, 8, 100)
            + i_instr(0x08, 8, 2, 50)   # ADDI $v0, $t0, 50
            + HALT
        )
        assert run_state(prog).regs[2] == 150

    def test_addi_overflow(self):
        prog = (
            i_instr(0x0F, 0, 8, 0x7FFF)   # MAX_INT in $t0
            + i_instr(0x0D, 8, 8, 0xFFFF)
            + i_instr(0x08, 8, 2, 1)      # ADDI — overflow
            + HALT
        )
        result = run(prog)
        assert result.error is not None
        assert "overflow" in result.error.lower()


# ── SLLV / SRLV / SRAV ────────────────────────────────────────────────────────


class TestVariableShifts:
    def test_sllv(self):
        prog = (
            i_instr(0x09, 0, 8, 1)      # $t0 = 1
            + i_instr(0x09, 0, 9, 4)    # $t1 = 4 (shift amount)
            + r_instr(9, 8, 2, 0, 0x04) # SLLV $v0, $t0, $t1 → 16
            + HALT
        )
        assert run_state(prog).regs[2] == 16

    def test_srlv(self):
        prog = (
            i_instr(0x09, 0, 8, 16)     # $t0 = 16
            + i_instr(0x09, 0, 9, 2)    # $t1 = 2
            + r_instr(9, 8, 2, 0, 0x06) # SRLV $v0, $t0, $t1 → 4
            + HALT
        )
        assert run_state(prog).regs[2] == 4

    def test_srav(self):
        prog = (
            i_instr(0x0F, 0, 8, 0xFFFF)  # $t0 = 0xFFFF0000 (negative)
            + i_instr(0x09, 0, 9, 4)     # $t1 = 4
            + r_instr(9, 8, 2, 0, 0x07)  # SRAV $v0, $t0, $t1 → sign-fill
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] & 0xF000_0000 == 0xF000_0000  # upper bits sign-filled


# ── LWL / LWR / SWL / SWR ─────────────────────────────────────────────────────


class TestUnalignedMemory:
    def test_lwl_executes(self):
        """LWL executes without error and produces some value."""
        prog = (
            i_instr(0x0F, 0, 8, 0xDEAD)   # store test word
            + i_instr(0x0D, 8, 8, 0xBEEF)
            + i_instr(0x2B, 0, 8, 0x100)   # SW $t0, 0x100
            + i_instr(0x22, 0, 9, 0x103)   # LWL $t1, 0x103 (byte offset 3 = load all)
            + HALT
        )
        result = MIPSR2000GateLevelSimulator().execute(prog)
        assert result.halted
        # LWL at byte 3 of aligned word 0x100: shift=24, mem_mask=0xFFFFFFFF, rt_mask=0
        # result = (mem_word & 0xFFFFFFFF) | (rt_val & 0) = mem_word
        assert result.final_state.regs[9] == 0xDEAD_BEEF

    def test_lwr_executes(self):
        """LWR at byte offset 0 loads the full word."""
        prog = (
            i_instr(0x0F, 0, 8, 0xDEAD)
            + i_instr(0x0D, 8, 8, 0xBEEF)
            + i_instr(0x2B, 0, 8, 0x100)   # SW $t0, 0x100
            + i_instr(0x26, 0, 9, 0x100)   # LWR $t1, 0x100 (byte offset 0 = full word)
            + HALT
        )
        result = MIPSR2000GateLevelSimulator().execute(prog)
        assert result.halted
        # LWR at byte 0: shift=0 → full word
        assert result.final_state.regs[9] == 0xDEAD_BEEF

    def test_swl_executes(self):
        """SWL stores the MSB bytes of rt into memory."""
        prog = (
            i_instr(0x0F, 0, 8, 0xDEAD)
            + i_instr(0x0D, 8, 8, 0xBEEF)
            + i_instr(0x2A, 0, 8, 0x103)   # SWL $t0, 0x103 (byte_offset=3: store all bytes)
            + i_instr(0x23, 0, 9, 0x100)   # LW $t1, 0x100
            + HALT
        )
        result = MIPSR2000GateLevelSimulator().execute(prog)
        # SWL at byte 3: shift=0 → full store of rt
        assert result.halted
        assert result.final_state.regs[9] == 0xDEAD_BEEF

    def test_swr_executes(self):
        """SWR stores the LSB bytes of rt into memory."""
        prog = (
            i_instr(0x0F, 0, 8, 0xDEAD)
            + i_instr(0x0D, 8, 8, 0xBEEF)
            + i_instr(0x2E, 0, 8, 0x100)   # SWR $t0, 0x100 (byte_offset=0: store all bytes)
            + i_instr(0x23, 0, 9, 0x100)   # LW $t1, 0x100
            + HALT
        )
        result = MIPSR2000GateLevelSimulator().execute(prog)
        # SWR at byte 0: shift=0 → full store of rt
        assert result.halted
        assert result.final_state.regs[9] == 0xDEAD_BEEF


# ── Error paths ────────────────────────────────────────────────────────────────


class TestErrorPaths:
    def test_break_raises(self):
        prog = (
            r_instr(0, 0, 0, 0, 0x0D)   # BREAK
            + HALT
        )
        result = run(prog)
        assert result.error is not None
        assert "BREAK" in result.error

    def test_unknown_opcode(self):
        prog = w(0xFC00_0000)  # op=0x3F (unknown)
        result = run(prog)
        assert result.error is not None

    def test_unknown_r_type_funct(self):
        prog = w(0x0000_003F)  # op=0, funct=0x3F (unknown)
        result = run(prog)
        assert result.error is not None

    def test_unknown_regimm_rt(self):
        prog = w(0x0400_0000 | (0x0F << 16))  # op=1, rt=0x0F (unknown REGIMM)
        result = run(prog)
        assert result.error is not None

    def test_misaligned_lw(self):
        prog = i_instr(0x23, 0, 2, 1)  # LW at addr 1 (misaligned)
        result = run(prog)
        assert result.error is not None
        assert "misaligned" in result.error.lower()

    def test_misaligned_sw(self):
        prog = i_instr(0x2B, 0, 2, 2)  # SW at addr 2 (misaligned)
        result = run(prog)
        assert result.error is not None

    def test_load_too_large(self):
        sim = MIPSR2000GateLevelSimulator()
        with pytest.raises(ValueError, match="too large"):
            sim.load(bytes(0x10001))

    def test_halted_step_is_noop(self):
        sim = MIPSR2000GateLevelSimulator()
        sim.execute(HALT)
        trace = sim.step()
        assert trace.mnemonic == "HALT"

    def test_io_port_stubs(self):
        sim = MIPSR2000GateLevelSimulator()
        sim.set_input_port(0, 42)
        assert sim.get_output_port(0) == 0
        sim.interrupt()
        sim.nmi()


# ── Additional simulator coverage ─────────────────────────────────────────────


class TestSimulatorCoverage:
    def test_sub_overflow_raises(self):
        # MIN_INT - 1 = overflow
        # MIN_INT = 0x80000000, 1 = 0x00000001
        # 0x80000000 - 0x00000001 = overflow (most negative minus positive)
        prog = (
            i_instr(0x0F, 0, 8, 0x8000)    # LUI $t0, 0x8000 → $t0 = 0x80000000
            + i_instr(0x09, 0, 9, 1)       # ADDIU $t1, $zero, 1
            + r_instr(8, 9, 2, 0, 0x22)    # SUB $v0, $t0, $t1 → overflow
            + HALT
        )
        result = run(prog)
        assert result.error is not None
        assert "overflow" in result.error.lower()

    def test_nop_instruction(self):
        prog = w(0x0000_0000) + HALT  # NOP then HALT
        state = run_state(prog)
        assert state.halted

    def test_already_halted_execute(self):
        sim = MIPSR2000GateLevelSimulator()
        result = sim.execute(HALT)
        assert result.halted
        # second execute starts fresh
        result2 = sim.execute(HALT)
        assert result2.halted

    def test_max_steps_exceeded(self):
        # Infinite loop: BEQ $zero, $zero, 0 (infinite loop)
        prog = i_instr(0x04, 0, 0, 0xFFFF)  # BEQ $zero, $zero, -1 (infinite)
        result = run(prog, max_steps=10)
        assert result.error is not None
        assert "max_steps" in result.error

    def test_div_signed_negative_numerator(self):
        # DIV: -10 / 3 via signed funct
        prog = (
            i_instr(0x0F, 0, 8, 0xFFFF)    # LUI $t0, 0xFFFF → negative
            + i_instr(0x0D, 8, 8, 0xFFF6)  # ORI → 0xFFFFFFF6 = -10
            + i_instr(0x09, 0, 9, 3)       # $t1 = 3
            + r_instr(8, 9, 0, 0, 0x1A)    # DIV $t0, $t1
            + r_instr(0, 0, 2, 0, 0x12)    # MFLO $v0 (quotient = -3)
            + HALT
        )
        st = run_state(prog)
        assert st.regs[2] == 0xFFFF_FFFD  # -3

    def test_multu_basic(self):
        prog = (
            i_instr(0x09, 0, 8, 1000)
            + i_instr(0x09, 0, 9, 1000)
            + r_instr(8, 9, 0, 0, 0x19)   # MULTU $t0, $t1
            + r_instr(0, 0, 2, 0, 0x12)   # MFLO $v0
            + HALT
        )
        assert run_state(prog).regs[2] == 1_000_000

    def test_beq_zero_offset(self):
        # BEQ with offset=0 (branch to next instruction effectively = no-op)
        prog = (
            i_instr(0x09, 0, 8, 5)
            + i_instr(0x09, 0, 9, 5)
            + i_instr(0x04, 8, 9, 0)  # BEQ $t0, $t1, 0 → branch to next instr
            + i_instr(0x09, 0, 2, 7)
            + HALT
        )
        assert run_state(prog).regs[2] == 7

    def test_lbu_lhu(self):
        prog = (
            i_instr(0x09, 0, 8, 0xFF)
            + i_instr(0x28, 0, 8, 0x200)    # SB $t0, 0x200
            + i_instr(0x24, 0, 9, 0x200)    # LBU $t1
            + i_instr(0x0D, 0, 10, 0x1234)  # ORI $t2, $zero, 0x1234
            + i_instr(0x29, 0, 10, 0x202)   # SH $t2, 0x202
            + i_instr(0x25, 0, 11, 0x202)   # LHU $t3
            + HALT
        )
        st = run_state(prog)
        assert st.regs[9] == 0xFF
        assert st.regs[11] == 0x1234

    def test_bltz_not_taken_positive(self):
        prog = (
            i_instr(0x09, 0, 8, 5)
            + i_instr(0x01, 8, 0, 5)      # BLTZ (not taken)
            + i_instr(0x09, 0, 2, 42)
            + HALT
        )
        assert run_state(prog).regs[2] == 42

    def test_bgez_not_taken_negative(self):
        prog = (
            i_instr(0x0F, 0, 8, 0xFFFF)
            + i_instr(0x01, 8, 1, 5)     # BGEZ $t0, +5 (not taken, $t0 < 0)
            + i_instr(0x09, 0, 2, 6)
            + HALT
        )
        assert run_state(prog).regs[2] == 6
