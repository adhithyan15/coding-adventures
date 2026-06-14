"""Edge-case and coverage tests for the SPARC V8 simulator.

These tests target specific hardware behaviors, error conditions, and corner
cases not exercised by the normal instruction flow tests.
"""

from __future__ import annotations

import struct

from sparc_v8_simulator import SPARCSimulator

# ── Helpers ───────────────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    return struct.pack(">I", v & 0xFFFF_FFFF)


HALT = w32(0x91D0_2000)
NOP  = w32(0x0100_0000)


def add_i(rd: int, rs1: int, simm13: int) -> bytes:
    return w32((0x2 << 30) | (rd << 25) | (0x00 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def addcc_i(rd: int, rs1: int, simm13: int) -> bytes:
    return w32((0x2 << 30) | (rd << 25) | (0x10 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def subcc_i(rd: int, rs1: int, simm13: int) -> bytes:
    return w32((0x2 << 30) | (rd << 25) | (0x14 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def or_i(rd: int, rs1: int, simm13: int) -> bytes:
    return w32((0x2 << 30) | (rd << 25) | (0x02 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def sethi(rd: int, imm22: int) -> bytes:
    return w32((rd << 25) | (0x4 << 22) | (imm22 & 0x3FFFFF))


def mk_mem_i(op3: int, rd: int, rs1: int, simm13: int) -> bytes:
    return w32((0x3 << 30) | (rd << 25) | (op3 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def save_i(rd: int, rs1: int, simm13: int) -> bytes:
    return w32((0x2 << 30) | (rd << 25) | (0x3C << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def restore_i(rd: int, rs1: int, simm13: int) -> bytes:
    return w32((0x2 << 30) | (rd << 25) | (0x3D << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


# ── %g0 always zero ───────────────────────────────────────────────────────────

class TestG0Immutable:
    """Writing to %g0 must have no effect."""

    def test_add_to_g0_discarded(self):
        """ADD %g0, %g0, 42 — result discarded, %g0 stays 0."""
        prog = add_i(0, 0, 42) + HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._regs[0] == 0

    def test_sethi_to_g0_discarded(self):
        """SETHI 0xDEADB, %g0 — discarded."""
        prog = sethi(0, 0xDEADB) + HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._regs[0] == 0

    def test_subcc_to_g0_discarded(self):
        """SUBcc %g0, %g1, %g0 — rd=0 discarded, but flags still set."""
        prog  = add_i(1, 0, 5)
        prog += subcc_i(0, 1, 5)    # SUBcc %g0, %g1, 5 → 0 (Z=1)
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._regs[0] == 0
        assert sim._psr_z   # flags were updated despite rd=0


# ── Condition code edge cases ─────────────────────────────────────────────────

class TestConditionCodes:

    def test_addcc_zero_result(self):
        """ADDcc 0 + 0 → Z=1, N=0, V=0, C=0."""
        prog = addcc_i(1, 0, 0) + HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._psr_z
        assert not sim._psr_n
        assert not sim._psr_v
        assert not sim._psr_c

    def test_addcc_max_plus_1_overflow(self):
        """ADDcc 0x7FFFFFFF + 1 → V=1, N=1 (signed overflow)."""
        prog  = sethi(1, 0x1FFFFF)
        prog += or_i(1, 1, 0xFFF)       # %g1 = 0x7FFFFFFF
        prog += addcc_i(2, 1, 1)
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._psr_v
        assert sim._psr_n

    def test_subcc_equal_operands_z(self):
        """SUBcc 5 - 5 = 0 → Z=1."""
        prog  = add_i(1, 0, 5)
        prog += subcc_i(2, 1, 5)
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._psr_z
        assert not sim._psr_n
        assert not sim._psr_c

    def test_subcc_borrow_sets_c(self):
        """SUBcc 0 - 1 → C=1 (borrow)."""
        prog = subcc_i(1, 0, 1) + HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._psr_c
        assert sim._psr_n

    def test_logic_clears_vc(self):
        """ANDcc always sets V=0, C=0."""
        sim = SPARCSimulator()
        sim._psr_v = True
        sim._psr_c = True
        prog = add_i(1, 0, 0xF) + w32((0x2 << 30) | (2 << 25) | (0x11 << 19) | (1 << 14) | (1 << 13) | 0xF) + HALT
        sim.execute(prog)
        assert not sim._psr_v
        assert not sim._psr_c


# ── Misaligned memory ─────────────────────────────────────────────────────────

class TestMisalignment:

    def test_ld_misaligned_raises(self):
        """LD from odd address raises ValueError."""
        prog  = add_i(1, 0, 1)
        prog += mk_mem_i(0x00, 2, 1, 0)   # LD %g2, [%g1] — misaligned
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok
        assert result.error is not None

    def test_st_misaligned_raises(self):
        """ST to odd address raises ValueError."""
        prog  = add_i(1, 0, 3)
        prog += mk_mem_i(0x04, 2, 1, 0)   # ST %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_lduh_misaligned_raises(self):
        """LDUH from odd address raises ValueError."""
        prog  = add_i(1, 0, 1)
        prog += mk_mem_i(0x02, 2, 1, 0)   # LDUH %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_sth_misaligned_raises(self):
        """STH to odd address raises ValueError."""
        prog  = add_i(1, 0, 1)
        prog += mk_mem_i(0x06, 2, 1, 0)   # STH %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_ldsb_any_addr_ok(self):
        """LDSB works at any byte address."""
        prog  = add_i(1, 0, 1)
        prog += mk_mem_i(0x09, 2, 1, 0)   # LDSB %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.ok

    def test_stb_any_addr_ok(self):
        """STB works at any byte address."""
        prog  = add_i(1, 0, 3)
        prog += mk_mem_i(0x05, 2, 1, 0)   # STB %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.ok


# ── Division by zero ──────────────────────────────────────────────────────────

class TestDivisionErrors:

    def test_udiv_by_zero_raises(self):
        """UDIV by zero raises ValueError."""
        prog  = add_i(1, 0, 10)
        prog += w32((0x2 << 30) | (3 << 25) | (0x0E << 19) | (1 << 14) | 0)  # UDIV %g3, %g1, %g0
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok
        assert result.error is not None

    def test_sdiv_by_zero_raises(self):
        """SDIV by zero raises ValueError."""
        prog  = add_i(1, 0, 10)
        prog += w32((0x2 << 30) | (3 << 25) | (0x0F << 19) | (1 << 14) | 0)  # SDIV %g3, %g1, %g0
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok


# ── Unknown opcodes ───────────────────────────────────────────────────────────

class TestUnknownOpcodes:

    def test_unknown_alu_op3_raises(self):
        """Unknown ALU op3 raises ValueError."""
        # op=2, op3=0x3B (unassigned)
        prog = w32((0x2 << 30) | (0x3B << 19)) + HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_unknown_mem_op3_raises(self):
        """Unknown memory op3 raises ValueError."""
        # op=3, op3=0x10 (unassigned)
        prog = w32((0x3 << 30) | (0x10 << 19)) + HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_unknown_fmt2_op2_raises(self):
        """Unknown Format-2 op2 raises ValueError."""
        # op=0, op2=0x7 (unassigned)
        prog = w32((0x0 << 30) | (0x7 << 22)) + HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_ticc_non_always_raises(self):
        """Ticc with condition other than 'always' raises ValueError."""
        # op=2, op3=0x3A, cond=1 (TE — trap if equal), i=1, simm13=0
        prog = w32((0x2 << 30) | (0x1 << 25) | (0x3A << 19) | (1 << 13)) + HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok


# ── Window overflow ───────────────────────────────────────────────────────────

class TestWindowOverflow:

    def test_too_many_saves_raises(self):
        """Exhausting all windows with SAVE raises ValueError (window overflow)."""
        # With NWINDOWS=3: CWP goes 0→2→1, next SAVE would wrap to 0 = overflow
        prog  = save_i(0, 0, 0)    # CWP 0 → 2
        prog += save_i(0, 0, 0)    # CWP 2 → 1
        prog += save_i(0, 0, 0)    # CWP 1 → 0 = overflow (same as caller)
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert not result.ok


# ── max_steps guard ───────────────────────────────────────────────────────────

class TestMaxSteps:

    def test_max_steps_terminates(self):
        """Infinite BA loop terminated by max_steps."""
        # BA 0: branch always to self (disp22=0 → target = PC + 0*4 = PC)
        loop = w32((0x0 << 30) | (0x8 << 25) | (0x2 << 22) | 0)   # BA 0 (self-loop)
        result = SPARCSimulator().execute(loop + HALT, max_steps=5)
        assert not result.ok
        assert result.steps == 5
        assert "max_steps" in result.error

    def test_max_steps_default_is_100000(self):
        """Default max_steps is 100,000."""
        import inspect
        sig = inspect.signature(SPARCSimulator.execute)
        assert sig.parameters["max_steps"].default == 100_000


# ── Big-endian memory ─────────────────────────────────────────────────────────

class TestBigEndian:

    def test_st_stores_big_endian(self):
        """ST 0x12345678 at addr 0x100 → bytes [0x12, 0x34, 0x56, 0x78]."""
        sim = SPARCSimulator()
        sim.load(HALT)
        sim._store_word(0x100, 0x1234_5678)
        assert sim._mem[0x100] == 0x12
        assert sim._mem[0x101] == 0x34
        assert sim._mem[0x102] == 0x56
        assert sim._mem[0x103] == 0x78

    def test_ld_reads_big_endian(self):
        """LD from [0xDE, 0xAD, 0xBE, 0xEF] = 0xDEADBEEF."""
        sim = SPARCSimulator()
        sim.load(HALT)
        sim._mem[0x200] = 0xDE
        sim._mem[0x201] = 0xAD
        sim._mem[0x202] = 0xBE
        sim._mem[0x203] = 0xEF
        assert sim._load_word(0x200) == 0xDEAD_BEEF


# ── Instruction encoding tests ────────────────────────────────────────────────

class TestInstructionEncoding:

    def test_halt_mnemonic(self):
        """ta 0 reports mnemonic 'HALT'."""
        sim = SPARCSimulator()
        sim.load(HALT)
        trace = sim.step()
        assert trace.mnemonic == "HALT"

    def test_nop_mnemonic(self):
        """NOP (0x01000000) reports mnemonic 'NOP'."""
        sim = SPARCSimulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert trace.mnemonic == "NOP"

    def test_sethi_then_or_constant_0xdeadbeef(self):
        """SETHI + OR idiom loads 0xDEADBEEF."""
        prog  = sethi(1, 0x37AB6C)        # %g1 = 0xDEADB000  (0x37AB6C << 10)
        prog += or_i(1, 1, 0xEEF)         # %g1 |= 0xEEF → 0xDEADBEEF
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(1) == 0xDEAD_BEEF

    def test_srl_no_sign_fill(self):
        """SRL on 0x80000000 by 1 → 0x40000000 (zero fill)."""
        prog  = sethi(1, 0x200000)        # %g1 = 0x80000000  (0x200000 << 10)
        prog += w32((0x2 << 30) | (2 << 25) | (0x26 << 19) | (1 << 14) | (1 << 13) | 1)  # SRL 1
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 0x4000_0000

    def test_sra_preserves_sign(self):
        """SRA on 0x80000000 by 1 → 0xC0000000 (sign-fill)."""
        prog  = sethi(1, 0x200000)
        prog += w32((0x2 << 30) | (2 << 25) | (0x27 << 19) | (1 << 14) | (1 << 13) | 1)  # SRA 1
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 0xC000_0000

    def test_addx_uses_carry(self):
        """ADDX includes carry from previous operation.

        execute() calls reset() internally, so carry must be set AFTER load().
        """
        sim = SPARCSimulator()
        prog  = add_i(1, 0, 5)
        prog += w32((0x2 << 30) | (2 << 25) | (0x08 << 19) | (1 << 14) | (1 << 13) | 3)  # ADDX %g2,%g1,3
        prog += HALT
        sim.load(prog)
        sim._psr_c = True          # set carry AFTER load() (which calls reset())
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 9   # 5 + 3 + 1 (carry)
