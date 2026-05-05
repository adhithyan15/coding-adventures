"""Edge-case and coverage tests for the MIPS R2000 simulator.

These tests target specific hardware behaviors, error conditions, and corner
cases not exercised by the normal instruction flow tests.
"""

from __future__ import annotations

import struct

from mips_r2000_simulator import MIPSSimulator

# ── Helpers ───────────────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    """Pack a 32-bit unsigned int as 4 big-endian bytes."""
    return struct.pack(">I", v & 0xFFFF_FFFF)


HALT = w32(0x0000_000C)   # SYSCALL
NOP  = w32(0x0000_0000)   # SLL $zero,$zero,0


def addiu(rt: int, rs: int, imm: int) -> bytes:
    return w32((0x09 << 26) | (rs << 21) | (rt << 16) | (imm & 0xFFFF))


def R(rs: int, rt: int, rd: int, shamt: int, funct: int) -> bytes:
    return w32((rs << 21) | (rt << 16) | (rd << 11) | (shamt << 6) | funct)


def I_instr(op: int, rs: int, rt: int, imm: int) -> bytes:
    return w32((op << 26) | (rs << 21) | (rt << 16) | (imm & 0xFFFF))


# ── R0 always zero ────────────────────────────────────────────────────────────

class TestR0Immutable:
    """Writing to R0 ($zero) must have no effect."""

    def test_addiu_to_r0_discarded(self):
        """ADDIU $zero, $zero, 42 — result discarded, R0 stays 0."""
        prog = I_instr(0x09, 0, 0, 42) + HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[0] == 0

    def test_addu_to_r0_discarded(self):
        """ADDU $zero, $t0, $t1 — rd=0 discards the write."""
        prog  = addiu(8, 0, 5)
        prog += addiu(9, 0, 3)
        prog += R(8, 9, 0, 0, 0x21)     # ADDU $zero, $t0, $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[0] == 0

    def test_lui_to_r0_discarded(self):
        """LUI $zero, 0xDEAD — discarded."""
        prog = w32((0x0F << 26) | (0 << 21) | (0 << 16) | 0xDEAD) + HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[0] == 0


# ── Signed overflow exceptions ────────────────────────────────────────────────

class TestOverflow:

    def test_add_overflow_raises(self):
        """ADD raises ValueError on signed overflow (0x7FFFFFFF + 1)."""
        # Load 0x7FFFFFFF into $t0 using LUI + ORI
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0x7FFF)   # LUI $t0, 0x7FFF
        prog += w32((0x0D << 26) | (8 << 21) | (8 << 16) | 0xFFFF)   # ORI $t0,$t0,0xFFFF
        prog += addiu(9, 0, 1)                                          # $t1 = 1
        prog += R(8, 9, 10, 0, 0x20)    # ADD $t2, $t0, $t1 — OVERFLOW
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok
        assert result.error is not None

    def test_addi_overflow_raises(self):
        """ADDI raises ValueError on signed overflow."""
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0x7FFF)
        prog += w32((0x0D << 26) | (8 << 21) | (8 << 16) | 0xFFFF)   # $t0 = 0x7FFFFFFF
        prog += I_instr(0x08, 8, 9, 1)  # ADDI $t1, $t0, 1 — overflow
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_sub_overflow_raises(self):
        """SUB raises ValueError: -0x80000000 - 1 overflows."""
        # Load 0x80000000 (most negative 32-bit int) via LUI
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0x8000)   # LUI $t0, 0x8000
        prog += addiu(9, 0, 1)                                          # $t1 = 1
        prog += R(8, 9, 10, 0, 0x22)    # SUB $t2, $t0, $t1 — overflow
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_addu_does_not_raise_on_overflow(self):
        """ADDU silently wraps — no ValueError."""
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0x7FFF)
        prog += w32((0x0D << 26) | (8 << 21) | (8 << 16) | 0xFFFF)   # 0x7FFFFFFF
        prog += addiu(9, 0, 1)
        prog += R(8, 9, 10, 0, 0x21)    # ADDU — wraps
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert sim._regs[10] == 0x8000_0000

    def test_subu_does_not_raise(self):
        """SUBU 0 - 1 wraps to 0xFFFFFFFF — no ValueError."""
        prog  = addiu(9, 0, 1)
        prog += R(0, 9, 10, 0, 0x23)    # SUBU $t2, $zero, $t1
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert sim._regs[10] == 0xFFFF_FFFF


# ── Division by zero ──────────────────────────────────────────────────────────

class TestDivisionErrors:

    def test_div_by_zero_raises(self):
        """DIV by zero raises ValueError."""
        prog  = addiu(8, 0, 10)
        prog += R(8, 0, 0, 0, 0x1A)     # DIV $t0, $zero
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok
        assert result.error is not None

    def test_divu_by_zero_raises(self):
        """DIVU by zero raises ValueError."""
        prog  = addiu(8, 0, 10)
        prog += R(8, 0, 0, 0, 0x1B)     # DIVU $t0, $zero
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok


# ── Misaligned memory access ──────────────────────────────────────────────────

class TestMisalignment:

    def test_lw_misaligned_raises(self):
        """LW from odd address raises ValueError."""
        prog  = addiu(8, 0, 1)          # $t0 = 1 (odd address)
        prog += I_instr(0x23, 8, 9, 0)  # LW $t1, 0($t0) — misaligned
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok
        assert result.error is not None

    def test_sw_misaligned_raises(self):
        """SW to odd address raises ValueError."""
        prog  = addiu(8, 0, 3)          # addr 3 — misaligned
        prog += addiu(9, 0, 42)
        prog += I_instr(0x2B, 8, 9, 0)  # SW $t1, 0($t0)
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_lh_misaligned_raises(self):
        """LH from odd address raises ValueError."""
        prog  = addiu(8, 0, 1)
        prog += I_instr(0x21, 8, 9, 0)  # LH $t1, 0($t0)
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_sh_misaligned_raises(self):
        """SH to odd address raises ValueError."""
        prog  = addiu(8, 0, 1)
        prog += addiu(9, 0, 0xAB)
        prog += I_instr(0x29, 8, 9, 0)  # SH $t1, 0($t0)
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_lb_any_addr_ok(self):
        """LB works on any byte address (no alignment required)."""
        prog  = addiu(8, 0, 1)          # odd address
        prog += I_instr(0x20, 8, 9, 0)  # LB $t1, 0($t0)
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.ok

    def test_sb_any_addr_ok(self):
        """SB works on any byte address."""
        prog  = addiu(8, 0, 1)
        prog += addiu(9, 0, 0xAB)
        prog += I_instr(0x28, 8, 9, 0)  # SB $t1, 0($t0)
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.ok


# ── BREAK instruction ─────────────────────────────────────────────────────────

class TestBreak:

    def test_break_raises_value_error(self):
        """BREAK (funct=0x0D) raises ValueError — software breakpoint."""
        prog = w32(0x0000_000D) + HALT   # BREAK 0
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok
        assert result.error is not None

    def test_break_not_same_as_halt(self):
        """BREAK stops execution with error, not clean HALT."""
        prog = w32(0x0000_000D) + HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.halted    # HALT sentinel not set


# ── Unknown opcode ────────────────────────────────────────────────────────────

class TestUnknownOpcode:

    def test_unknown_op_raises(self):
        """An unimplemented opcode raises ValueError via execute()."""
        # op=0x3F is unassigned in MIPS
        prog = w32(0xFC00_0000) + HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_unknown_funct_raises(self):
        """An unknown R-type funct raises ValueError."""
        # op=0, funct=0x01 (unassigned)
        prog = w32(0x0000_0001) + HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok

    def test_unknown_regimm_raises(self):
        """An unknown REGIMM rt field raises ValueError."""
        # op=1, rt=0x05 (unassigned)
        prog = w32((0x01 << 26) | (0 << 21) | (0x05 << 16)) + HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert not result.ok


# ── Specific signed/unsigned semantics ───────────────────────────────────────

class TestSignedUnsigned:

    def test_sra_preserves_sign(self):
        """SRA by 1 on 0x80000000 → 0xC0000000 (sign bit preserved)."""
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0x8000)  # LUI $t0,0x8000
        prog += R(0, 8, 9, 1, 0x03)   # SRA $t1, $t0, 1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 0xC000_0000

    def test_srl_no_sign_fill(self):
        """SRL on 0x80000000 by 1 → 0x40000000 (zero fill, no sign)."""
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0x8000)
        prog += R(0, 8, 9, 1, 0x02)   # SRL $t1, $t0, 1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 0x4000_0000

    def test_lb_0x7f_positive(self):
        """LB 0x7F → 127 (positive, no sign extension change)."""
        prog  = addiu(8, 0, 0x200)
        prog += addiu(9, 0, 0x7F)
        prog += I_instr(0x28, 8, 9, 0)  # SB $t1, 0($t0)
        prog += I_instr(0x20, 8, 10, 0) # LB $t2, 0($t0)
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0x7F

    def test_lbu_0x80_no_sign_extension(self):
        """LBU 0x80 → 128 (zero-extended, not -128)."""
        prog  = addiu(8, 0, 0x300)
        prog += w32((0x0D << 26) | (0 << 21) | (9 << 16) | 0x80)  # ORI $t1,0x80
        prog += I_instr(0x28, 8, 9, 0)  # SB
        prog += I_instr(0x24, 8, 10, 0) # LBU
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0x80

    def test_sltiu_sign_extends_imm_then_compares_unsigned(self):
        """SLTIU: imm is sign-extended to 32 bits, then compared as unsigned.

        SLTIU $t1, $t0, 0xFFFF: 0xFFFF sign-extended = 0xFFFFFFFF.
        If $t0 = 5: 5 < 0xFFFFFFFF (unsigned) → 1.
        """
        prog  = addiu(8, 0, 5)
        prog += I_instr(0x0B, 8, 9, 0xFFFF)  # SLTIU $t1, $t0, -1 (0xFFFFFFFF unsigned)
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 1

    def test_slti_vs_sltu_differ_for_negative(self):
        """SLT and SLTU give different results for 0xFFFFFFFF.

        Signed:   0xFFFFFFFF = -1 < 1 → SLT gives 1
        Unsigned: 0xFFFFFFFF = large positive > 1 → SLTU gives 0
        """
        prog  = addiu(8, 0, 0xFFFF)     # $t0 = 0xFFFFFFFF (= -1 signed)
        prog += addiu(9, 0, 1)           # $t1 = 1
        prog += R(8, 9, 10, 0, 0x2A)    # SLT  $t2, $t0, $t1 → 1 (signed: -1 < 1)
        prog += R(8, 9, 11, 0, 0x2B)    # SLTU $t3, $t0, $t1 → 0 (unsigned: 0xFFFFFFFF > 1)
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 1   # SLT
        assert sim._regs[11] == 0   # SLTU

    def test_mult_negative_numbers(self):
        """MULT (-1) * (-1) = +1 (signed 32-bit × 32-bit → 64-bit result)."""
        prog  = addiu(8, 0, 0xFFFF)     # -1
        prog += addiu(9, 0, 0xFFFF)     # -1
        prog += R(8, 9, 0, 0, 0x18)     # MULT $t0, $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._lo == 1
        assert sim._hi == 0   # +1 fits in LO, HI = 0


# ── max_steps guard ───────────────────────────────────────────────────────────

class TestMaxSteps:

    def test_max_steps_terminates(self):
        """Infinite BEQ loop terminated by max_steps."""
        loop = I_instr(0x04, 0, 0, 0xFFFF)   # BEQ $zero,$zero,-1 → PC-4 = forever
        result = MIPSSimulator().execute(loop + HALT, max_steps=5)
        assert not result.ok
        assert result.steps == 5
        assert "max_steps" in result.error

    def test_max_steps_default_is_100000(self):
        """Default max_steps is 100,000."""
        import inspect
        sig = inspect.signature(MIPSSimulator.execute)
        assert sig.parameters["max_steps"].default == 100_000


# ── Big-endian memory layout ──────────────────────────────────────────────────

class TestBigEndian:

    def test_sw_stores_big_endian(self):
        """SW 0x12345678 at addr 0x100 → bytes [0x12, 0x34, 0x56, 0x78]."""
        sim = MIPSSimulator()
        sim.load(HALT)
        sim._store_word(0x100, 0x1234_5678)
        assert sim._mem[0x100] == 0x12
        assert sim._mem[0x101] == 0x34
        assert sim._mem[0x102] == 0x56
        assert sim._mem[0x103] == 0x78

    def test_lw_reads_big_endian(self):
        """LW from address with bytes [0xDE, 0xAD, 0xBE, 0xEF] = 0xDEADBEEF."""
        sim = MIPSSimulator()
        sim.load(HALT)
        sim._mem[0x200] = 0xDE
        sim._mem[0x201] = 0xAD
        sim._mem[0x202] = 0xBE
        sim._mem[0x203] = 0xEF
        assert sim._load_word(0x200) == 0xDEAD_BEEF


# ── Instruction encoding ──────────────────────────────────────────────────────

class TestInstructionEncoding:

    def test_nop_canonical_mnemonic(self):
        """NOP (0x00000000) reports mnemonic 'NOP' — the canonical MIPS disassembly."""
        sim = MIPSSimulator()
        sim.load(NOP + HALT)
        trace = sim.step()
        assert trace.mnemonic == "NOP"
        assert sim._regs[0] == 0

    def test_halt_mnemonic(self):
        """SYSCALL instruction reports mnemonic 'HALT'."""
        sim = MIPSSimulator()
        sim.load(HALT)
        trace = sim.step()
        assert trace.mnemonic == "HALT"

    def test_lui_andi_combine_for_upper_lower(self):
        """LUI + ORI idiom loads a full 32-bit constant."""
        # Load 0xDEADBEEF: LUI $t0, 0xDEAD; ORI $t0, $t0, 0xBEEF
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0xDEAD)  # LUI
        prog += w32((0x0D << 26) | (8 << 21) | (8 << 16) | 0xBEEF)  # ORI
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 0xDEAD_BEEF
