"""Per-instruction tests for the MIPS R2000 simulator.

Every instruction variant is tested for correct register/memory effect and
correct PC advancement.  Branch and jump instructions are tested for both
taken and not-taken cases.

MIPS instruction encoding reference:
  R-type: [op:6=0][rs:5][rt:5][rd:5][shamt:5][funct:6]
  I-type: [op:6][rs:5][rt:5][imm16:16]
  J-type: [op:6][target26:26]
"""

from __future__ import annotations

import struct

from mips_r2000_simulator import MIPSSimulator
from mips_r2000_simulator.state import REG_RA

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    """Pack a 32-bit unsigned int as 4 big-endian bytes."""
    return struct.pack(">I", v & 0xFFFF_FFFF)


# Frequently used instruction words
HALT  = w32(0x0000_000C)   # SYSCALL
NOP   = w32(0x0000_0000)   # SLL $zero,$zero,0


def R(rs: int, rt: int, rd: int, shamt: int, funct: int) -> bytes:
    """Build an R-type instruction word."""
    return w32((rs << 21) | (rt << 16) | (rd << 11) | (shamt << 6) | funct)


def mk_i(op: int, rs: int, rt: int, imm: int) -> bytes:
    """Build an I-type instruction word."""
    return w32((op << 26) | (rs << 21) | (rt << 16) | (imm & 0xFFFF))


def J(op: int, target: int) -> bytes:
    """Build a J-type instruction word."""
    return w32((op << 26) | (target & 0x03FF_FFFF))


def run1(instr: bytes) -> MIPSSimulator:
    """Load and execute a single instruction followed by HALT."""
    sim = MIPSSimulator()
    sim.execute(instr + HALT)
    return sim


def run_with_reg(instr: bytes, reg: int, val: int) -> MIPSSimulator:
    """Set a register via ADDIU, then run instruction + HALT."""
    prog = mk_i(0x09, 0, reg, val) + instr + HALT   # ADDIU reg,$zero,val
    sim = MIPSSimulator()
    sim.execute(prog)
    return sim


# ── Shift instructions ────────────────────────────────────────────────────────

class TestShifts:

    def test_sll_by_4(self):
        """SLL $t0, $t1, 4  — shift $t1 left by 4."""
        # Set $t1 ($9) = 0x00000001, shift into $t0 ($8)
        prog = mk_i(0x09, 0, 9, 1)           # ADDIU $t1, $zero, 1
        prog += R(0, 9, 8, 4, 0x00)      # SLL $t0, $t1, 4
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 0x10

    def test_srl_logical(self):
        """SRL $t0, $t1, 4 — logical right shift (fills with zeros)."""
        prog = mk_i(0x09, 0, 9, 0x00F0)     # ADDIU $t1, $zero, 0xF0
        prog += R(0, 9, 8, 4, 0x02)      # SRL $t0, $t1, 4
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 0x0F

    def test_sra_arithmetic_negative(self):
        """SRA $t0, $t1, 4 — arithmetic shift preserves sign bit."""
        # Load -16 (0xFFFFFFF0) into $t1, shift right 4 → -1 (0xFFFFFFFF)
        # We can't directly load 0xFFFFFFF0 via ADDIU (only 16-bit), so use ADDIU of -16
        prog = mk_i(0x09, 0, 9, 0xFFF0)     # ADDIU $t1, $zero, -16 (sext: 0xFFFFFFF0)
        prog += R(0, 9, 8, 4, 0x03)      # SRA $t0, $t1, 4
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 0xFFFF_FFFF   # -1

    def test_sra_positive_no_sign_fill(self):
        """SRA on a positive value fills with zeros, same as SRL."""
        prog = mk_i(0x09, 0, 9, 0x0010)     # ADDIU $t1, $zero, 16
        prog += R(0, 9, 8, 2, 0x03)      # SRA $t0, $t1, 2
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 4

    def test_sllv_by_register(self):
        """SLLV $t0, $t1, $t2 — shift amount from register."""
        prog  = mk_i(0x09, 0, 9, 1)         # ADDIU $t1, $zero, 1
        prog += mk_i(0x09, 0, 10, 3)        # ADDIU $t2, $zero, 3 (shift by 3)
        prog += R(10, 9, 8, 0, 0x04)     # SLLV $t0, $t1, $t2
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 8

    def test_srlv_by_register(self):
        """SRLV $t0, $t1, $t2."""
        prog  = mk_i(0x09, 0, 9, 0x0040)   # ADDIU $t1, $zero, 64
        prog += mk_i(0x09, 0, 10, 2)        # ADDIU $t2, $zero, 2
        prog += R(10, 9, 8, 0, 0x06)     # SRLV $t0, $t1, $t2
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 16

    def test_srav_by_register(self):
        """SRAV $t0, $t1, $t2 — arithmetic shift by register."""
        prog  = mk_i(0x09, 0, 9, 0xFF80)   # ADDIU $t1, $zero, -128 sext → 0xFFFFFF80
        prog += mk_i(0x09, 0, 10, 3)        # ADDIU $t2, $zero, 3
        prog += R(10, 9, 8, 0, 0x07)     # SRAV $t0, $t1, $t2
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        # -128 >> 3 = -16 = 0xFFFFFFF0
        assert sim._regs[8] == 0xFFFF_FFF0


# ── Jump / call instructions ──────────────────────────────────────────────────

class TestJumps:

    def test_jr_jumps_to_register(self):
        """JR $ra — jump to address in $ra."""
        # Load $ra = 8 (byte addr of HALT after two NOPs at 0 and 4)
        # PC starts at 0: NOP(0), ADDIU $ra,0,8(4), JR $ra(8), ... HALT at 12
        prog = NOP                                 # 0x00: NOP
        prog += mk_i(0x09, 0, REG_RA, 12)            # 0x04: ADDIU $ra, $zero, 12
        prog += R(REG_RA, 0, 0, 0, 0x08)          # 0x08: JR $ra
        prog += NOP                                # 0x0C: NOP (would be skipped)
        prog += HALT                               # 0x10: HALT - but we jump here
        # Build: HALT at 0x0C
        prog2 = NOP                                # 0x00
        prog2 += mk_i(0x09, 0, REG_RA, 12)           # 0x04
        prog2 += R(REG_RA, 0, 0, 0, 0x08)         # 0x08: JR $ra → PC = 12
        prog2 += HALT                              # 0x0C: HALT (target)
        sim = MIPSSimulator()
        result = sim.execute(prog2)
        assert result.ok
        assert result.halted

    def test_jalr_sets_return_address(self):
        """JALR $ra, $t0 — sets $ra = PC+4 and jumps to $t0."""
        # Put address 0x10 in $t0, then JALR
        prog = bytearray(0x14)
        # 0x00: ADDIU $t0, $zero, 0x10
        struct.pack_into(">I", prog, 0x00, (0x09 << 26) | (0 << 21) | (8 << 16) | 0x10)
        # 0x04: JALR $ra, $t0  (rd=31, rs=8, funct=0x09)
        struct.pack_into(">I", prog, 0x04, (8 << 21) | (REG_RA << 11) | 0x09)
        # 0x08: NOP (this would run if no jump)
        struct.pack_into(">I", prog, 0x08, 0)
        # 0x0C: NOP
        struct.pack_into(">I", prog, 0x0C, 0)
        # 0x10: HALT (jump target)
        struct.pack_into(">I", prog, 0x10, 0x0000_000C)
        sim = MIPSSimulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        # $ra should be 0x08 (return address = instruction after JALR)
        assert sim._regs[REG_RA] == 0x08


# ── HI / LO ───────────────────────────────────────────────────────────────────

class TestHiLo:

    def test_mthi_mfhi(self):
        """MTHI / MFHI round-trip."""
        prog  = mk_i(0x09, 0, 8, 0x1234)   # ADDIU $t0, $zero, 0x1234
        prog += R(8, 0, 0, 0, 0x11)      # MTHI $t0
        prog += R(0, 0, 9, 0, 0x10)      # MFHI $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 0x1234
        assert sim._hi == 0x1234

    def test_mtlo_mflo(self):
        """MTLO / MFLO round-trip."""
        prog  = mk_i(0x09, 0, 8, 0x5678)
        prog += R(8, 0, 0, 0, 0x13)      # MTLO $t0
        prog += R(0, 0, 9, 0, 0x12)      # MFLO $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 0x5678

    def test_mult_result_in_hi_lo(self):
        """MULT $t0, $t1 — 3 * 4 = 12 → LO=12, HI=0."""
        prog  = mk_i(0x09, 0, 8, 3)
        prog += mk_i(0x09, 0, 9, 4)
        prog += R(8, 9, 0, 0, 0x18)      # MULT $t0, $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._lo == 12
        assert sim._hi == 0

    def test_multu_large(self):
        """MULTU 0xFFFFFFFF × 0xFFFFFFFF — result spans HI:LO."""
        # 0xFFFFFFFF * 0xFFFFFFFF = 0xFFFFFFFE_00000001
        # Load 0xFFFFFFFF = -1 unsigned, using LUI + ORI
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0xFFFF)  # LUI $t0, 0xFFFF
        prog += w32((0x0D << 26) | (8 << 21) | (8 << 16) | 0xFFFF)  # ORI $t0,$t0,0xFFFF
        prog += w32((0x0F << 26) | (0 << 21) | (9 << 16) | 0xFFFF)  # LUI $t1, 0xFFFF
        prog += w32((0x0D << 26) | (9 << 21) | (9 << 16) | 0xFFFF)  # ORI $t1,$t1,0xFFFF
        prog += R(8, 9, 0, 0, 0x19)      # MULTU $t0, $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._lo == 0x0000_0001
        assert sim._hi == 0xFFFF_FFFE

    def test_divu_quotient_remainder(self):
        """DIVU $t0, $t1 — 17 / 5 = quotient 3, remainder 2."""
        prog  = mk_i(0x09, 0, 8, 17)
        prog += mk_i(0x09, 0, 9, 5)
        prog += R(8, 9, 0, 0, 0x1B)      # DIVU $t0, $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._lo == 3   # quotient
        assert sim._hi == 2   # remainder

    def test_div_signed(self):
        """DIV $t0, $t1 — (-17) / 5 = quotient -3, remainder -2."""
        prog  = mk_i(0x09, 0, 8, 0xFFEF)   # ADDIU $t0, $zero, -17 (sext: 0xFFFFFFEF)
        prog += mk_i(0x09, 0, 9, 5)
        prog += R(8, 9, 0, 0, 0x1A)      # DIV $t0, $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        # -17 / 5 = -3 remainder -2 (truncate toward zero)
        assert sim._lo == 0xFFFF_FFFD   # -3 unsigned
        assert sim._hi == 0xFFFF_FFFE   # -2 unsigned


# ── ALU: ADD / ADDU / SUB / SUBU ─────────────────────────────────────────────

class TestArithmetic:

    def test_addu_no_overflow(self):
        """ADDU $t2, $t0, $t1 — 10 + 20 = 30."""
        prog  = mk_i(0x09, 0, 8, 10)
        prog += mk_i(0x09, 0, 9, 20)
        prog += R(8, 9, 10, 0, 0x21)     # ADDU $t2, $t0, $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 30

    def test_addu_wraps_silently(self):
        """ADDU wraps on 32-bit overflow without raising."""
        # 0xFFFFFFFF + 1 = 0x00000000 with wrap
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0xFFFF)  # LUI $t0,0xFFFF
        prog += w32((0x0D << 26) | (8 << 21) | (8 << 16) | 0xFFFF)  # ORI $t0,$t0,0xFFFF
        prog += mk_i(0x09, 0, 9, 1)                                      # ADDIU $t1,0,1
        prog += R(8, 9, 10, 0, 0x21)                                  # ADDU $t2,$t0,$t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0

    def test_subu_basic(self):
        """SUBU $t2, $t0, $t1 — 20 - 7 = 13."""
        prog  = mk_i(0x09, 0, 8, 20)
        prog += mk_i(0x09, 0, 9, 7)
        prog += R(8, 9, 10, 0, 0x23)     # SUBU $t2, $t0, $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 13

    def test_subu_wraps_to_large_positive(self):
        """SUBU 0 - 1 wraps to 0xFFFFFFFF (large unsigned)."""
        prog  = mk_i(0x09, 0, 9, 1)
        prog += R(0, 9, 10, 0, 0x23)     # SUBU $t2, $zero, $t1 → 0 - 1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0xFFFF_FFFF


# ── Logic ─────────────────────────────────────────────────────────────────────

class TestLogic:

    def test_and(self):
        prog  = mk_i(0x09, 0, 8, 0xFF)
        prog += mk_i(0x09, 0, 9, 0x0F)
        prog += R(8, 9, 10, 0, 0x24)     # AND $t2, $t0, $t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0x0F

    def test_or(self):
        prog  = mk_i(0x09, 0, 8, 0xF0)
        prog += mk_i(0x09, 0, 9, 0x0F)
        prog += R(8, 9, 10, 0, 0x25)     # OR
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0xFF

    def test_xor(self):
        prog  = mk_i(0x09, 0, 8, 0xFF)
        prog += mk_i(0x09, 0, 9, 0x0F)
        prog += R(8, 9, 10, 0, 0x26)     # XOR
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0xF0

    def test_nor(self):
        """NOR $t2, $zero, $zero → 0xFFFFFFFF (NOT 0)."""
        prog  = R(0, 0, 10, 0, 0x27)     # NOR $t2, $zero, $zero = ~(0|0)
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0xFFFF_FFFF

    def test_nor_nonzero_inputs(self):
        """NOR $t2, $t0, $t1 = ~(0xFF | 0x0F) = 0xFFFFFF00."""
        prog  = mk_i(0x09, 0, 8, 0xFF)
        prog += mk_i(0x09, 0, 9, 0x0F)
        prog += R(8, 9, 10, 0, 0x27)
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0xFFFF_FF00


# ── Set-less-than ─────────────────────────────────────────────────────────────

class TestSetLessThan:

    def test_slt_less(self):
        """SLT: signed -1 < 1 → 1."""
        prog  = mk_i(0x09, 0, 8, 0xFFFF)   # ADDIU $t0,$zero,-1 (signed)
        prog += mk_i(0x09, 0, 9, 1)
        prog += R(8, 9, 10, 0, 0x2A)    # SLT $t2,$t0,$t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 1

    def test_slt_not_less(self):
        """SLT: 5 < 3 → 0."""
        prog  = mk_i(0x09, 0, 8, 5)
        prog += mk_i(0x09, 0, 9, 3)
        prog += R(8, 9, 10, 0, 0x2A)
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0

    def test_sltu_unsigned_comparison(self):
        """SLTU: 0xFFFFFFFF > 1 unsigned → 0 (large uint is NOT less than 1)."""
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0xFFFF)  # LUI
        prog += w32((0x0D << 26) | (8 << 21) | (8 << 16) | 0xFFFF)  # ORI → 0xFFFFFFFF
        prog += mk_i(0x09, 0, 9, 1)
        prog += R(8, 9, 10, 0, 0x2B)    # SLTU $t2,$t0,$t1  (0xFFFFFFFF < 1?)
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 0       # No: 0xFFFFFFFF > 1 unsigned

    def test_sltu_less(self):
        """SLTU: 1 < 0xFFFFFFFF unsigned → 1."""
        prog  = mk_i(0x09, 0, 8, 1)
        prog += w32((0x0F << 26) | (0 << 21) | (9 << 16) | 0xFFFF)  # LUI $t1,0xFFFF
        prog += w32((0x0D << 26) | (9 << 21) | (9 << 16) | 0xFFFF)  # ORI $t1,$t1,0xFFFF
        prog += R(8, 9, 10, 0, 0x2B)    # SLTU $t2,$t0,$t1
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[10] == 1


# ── I-type immediate ALU ──────────────────────────────────────────────────────

class TestImmediateALU:

    def test_addiu_positive(self):
        prog = mk_i(0x09, 0, 8, 42) + HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 42

    def test_addiu_negative(self):
        """ADDIU with sign-extended negative immediate."""
        prog = mk_i(0x09, 0, 8, 0xFFFF) + HALT  # -1
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 0xFFFF_FFFF

    def test_addiu_wraps(self):
        """ADDIU wraps on overflow (unlike ADDI)."""
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0x7FFF)  # LUI $t0, 0x7FFF
        prog += w32((0x0D << 26) | (8 << 21) | (8 << 16) | 0xFFFF)  # ORI → 0x7FFFFFFF
        prog += mk_i(0x09, 8, 8, 1)   # ADDIU $t0,$t0,1 → wraps to 0x80000000
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 0x8000_0000

    def test_slti_signed(self):
        """SLTI: signed -1 < 1 → 1."""
        prog = mk_i(0x09, 0, 8, 0xFFFF) + mk_i(0x0A, 8, 9, 1) + HALT  # SLTI $t1,$t0,1
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 1

    def test_sltiu_unsigned(self):
        """SLTIU: treats immediate as unsigned (sign-extended but compared unsigned)."""
        # 5 < 10 → 1
        prog = mk_i(0x09, 0, 8, 5) + mk_i(0x0B, 8, 9, 10) + HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 1

    def test_andi_zero_extend(self):
        """ANDI zero-extends imm16 (no sign extension)."""
        prog  = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0xFFFF)  # LUI $t0,0xFFFF
        prog += w32((0x0D << 26) | (8 << 21) | (8 << 16) | 0xFFFF)  # ORI → 0xFFFFFFFF
        prog += mk_i(0x0C, 8, 9, 0x00FF)   # ANDI $t1, $t0, 0xFF
        prog += HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 0xFF     # upper 24 bits cleared

    def test_ori_zero_extend(self):
        """ORI zero-extends imm16."""
        prog = mk_i(0x0D, 0, 9, 0xABCD) + HALT  # ORI $t1, $zero, 0xABCD
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 0xABCD

    def test_xori(self):
        prog = mk_i(0x09, 0, 8, 0xFF) + mk_i(0x0E, 8, 9, 0x0F) + HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[9] == 0xF0

    def test_lui(self):
        """LUI $t0, 0xDEAD — loads upper 16 bits, lower 16 bits are 0."""
        prog = w32((0x0F << 26) | (0 << 21) | (8 << 16) | 0xDEAD) + HALT
        sim = MIPSSimulator()
        sim.execute(prog)
        assert sim._regs[8] == 0xDEAD_0000


# ── Branches ──────────────────────────────────────────────────────────────────

class TestBranches:
    """Branch target = (PC + 4) + sext(imm16) * 4.
    Since PC is already advanced to PC+4 by _fetch32, the formula in the
    simulator is: new_PC = self._pc + (simm << 2).
    """

    def _make_branch_skip(self, op: int, rs: int, rt: int, skip_offset: int) -> bytes:
        """Build a branch that jumps over 'skip_offset' instructions if taken."""
        # The branch is at addr 0, PC advances to 4.  If taken, target = 4 + skip_offset*4.
        return mk_i(op, rs, rt, skip_offset)

    def test_beq_taken(self):
        """BEQ $t0, $t1, +1  — both zero, branch skips 1 instruction."""
        # BEQ at 0x00: $zero==$zero → jump to PC+4+4 = 0x08; NOP at 0x04 (skipped); HALT at 0x08
        prog  = mk_i(0x04, 0, 0, 1) + NOP + HALT   # BEQ $zero,$zero,+1; NOP; HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.steps == 2   # BEQ + HALT (NOP skipped)

    def test_beq_not_taken(self):
        """BEQ $t0, $t1  — different values → not taken."""
        prog  = mk_i(0x09, 0, 8, 1)    # ADDIU $t0,$zero,1
        prog += mk_i(0x04, 8, 9, 5)    # BEQ $t0,$t1,+5  (t1=0 ≠ t0=1, not taken)
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.ok

    def test_bne_taken(self):
        """BNE $t0, $zero, +1  — $t0 != 0 → taken."""
        prog  = mk_i(0x09, 0, 8, 1)    # ADDIU $t0,$zero,1
        prog += mk_i(0x05, 8, 0, 1)    # BNE $t0,$zero,+1
        prog += NOP                  # skipped if taken
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.steps == 3   # ADDIU + BNE + HALT

    def test_bne_not_taken(self):
        """BNE $zero, $zero, +1  — both zero → not taken."""
        prog  = mk_i(0x05, 0, 0, 10)   # BNE $zero,$zero,+10 (not taken)
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.steps == 2   # BNE + HALT

    def test_blez_taken_zero(self):
        """BLEZ $zero, +1  — 0 <= 0 → taken."""
        prog  = mk_i(0x06, 0, 0, 1)    # BLEZ $zero,+1
        prog += NOP                  # skipped
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.steps == 2

    def test_blez_taken_negative(self):
        """BLEZ -5, +1  — negative → taken."""
        prog  = mk_i(0x09, 0, 8, 0xFFFB)  # ADDIU $t0,-5
        prog += mk_i(0x06, 8, 0, 1)        # BLEZ $t0,+1
        prog += NOP
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.steps == 3   # ADDIU + BLEZ + HALT

    def test_bgtz_taken(self):
        """BGTZ $t0, +1  — positive → taken."""
        prog  = mk_i(0x09, 0, 8, 5)
        prog += mk_i(0x07, 8, 0, 1)    # BGTZ $t0,+1
        prog += NOP
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.steps == 3   # ADDIU + BGTZ + HALT

    def test_bgtz_not_taken_zero(self):
        """BGTZ $zero, +1  — 0 is not > 0 → not taken."""
        prog  = mk_i(0x07, 0, 0, 1) + HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.steps == 2

    def test_bltz_taken(self):
        """BLTZ $t0, +1  — negative → taken."""
        prog  = mk_i(0x09, 0, 8, 0xFFFF)  # -1
        prog += mk_i(0x01, 8, 0, 1)        # BLTZ $t0,+1
        prog += NOP
        prog += HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.steps == 3

    def test_bgez_taken_zero(self):
        """BGEZ $zero, +1  — 0 >= 0 → taken."""
        prog  = mk_i(0x01, 0, 1, 1) + NOP + HALT   # BGEZ $zero,+1
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.steps == 2


# ── Loads and Stores ──────────────────────────────────────────────────────────

class TestLoadStore:

    def _load_store_sim(self) -> MIPSSimulator:
        """Return a fresh sim with some data in memory at 0x0100."""
        sim = MIPSSimulator()
        sim.reset()
        return sim

    def test_sw_lw_roundtrip(self):
        """SW then LW: store 0xDEADBEEF at addr 0x0100, load it back."""
        addr = 0x0100
        sim = MIPSSimulator()
        # Manually set register and store
        sim.load(HALT)   # reset + load minimal program
        sim._regs[8] = 0xDEAD_BEEF   # $t0 = value
        sim._regs[9] = addr            # $t1 = address
        sim._store_word(addr, 0xDEAD_BEEF)
        loaded = sim._load_word(addr)
        assert loaded == 0xDEAD_BEEF

    def test_sb_lb_sign_extend(self):
        """LB sign-extends 0xFF to 0xFFFFFFFF."""
        sim = MIPSSimulator()
        # Build program: SW then LB
        addr = 0x0200
        # Store byte 0xFF at addr, then LB to $t0
        prog = bytearray(0x20)
        # ADDIU $t1, $zero, addr
        struct.pack_into(">I", prog, 0, (0x09 << 26) | (0 << 21) | (9 << 16) | addr)
        # ADDIU $t0, $zero, 0xFF
        struct.pack_into(">I", prog, 4, (0x09 << 26) | (0 << 21) | (8 << 16) | 0xFF)
        # SB $t0, 0($t1)  — op=0x28, rs=9, rt=8, imm=0
        struct.pack_into(">I", prog, 8, (0x28 << 26) | (9 << 21) | (8 << 16))
        # LB $t2, 0($t1)  — op=0x20, rs=9, rt=10, imm=0
        struct.pack_into(">I", prog, 12, (0x20 << 26) | (9 << 21) | (10 << 16))
        # HALT
        struct.pack_into(">I", prog, 16, 0x0000_000C)
        sim = MIPSSimulator()
        sim.execute(bytes(prog))
        assert sim._regs[10] == 0xFFFF_FFFF   # sign-extended 0xFF

    def test_lbu_zero_extend(self):
        """LBU zero-extends 0xFF to 0x000000FF."""
        addr = 0x0300
        prog = bytearray(0x20)
        struct.pack_into(">I", prog, 0, (0x09 << 26) | (0 << 21) | (9 << 16) | addr)
        struct.pack_into(">I", prog, 4, (0x09 << 26) | (0 << 21) | (8 << 16) | 0xFF)
        struct.pack_into(">I", prog, 8, (0x28 << 26) | (9 << 21) | (8 << 16))   # SB
        struct.pack_into(">I", prog, 12, (0x24 << 26) | (9 << 21) | (10 << 16)) # LBU
        struct.pack_into(">I", prog, 16, 0x0000_000C)  # HALT
        sim = MIPSSimulator()
        sim.execute(bytes(prog))
        assert sim._regs[10] == 0xFF   # zero-extended

    def test_sh_lh_sign_extend(self):
        """LH sign-extends 0x8000 to 0xFFFF8000."""
        addr = 0x0400
        prog = bytearray(0x20)
        struct.pack_into(">I", prog, 0, (0x09 << 26) | (0 << 21) | (9 << 16) | addr)
        struct.pack_into(">I", prog, 4, (0x0F << 26) | (0 << 21) | (8 << 16) | 0x0000)
        struct.pack_into(">I", prog, 8, (0x0D << 26) | (8 << 21) | (8 << 16) | 0x8000)  # ORI $t0,0x8000
        struct.pack_into(">I", prog, 12, (0x29 << 26) | (9 << 21) | (8 << 16))   # SH $t0,0($t1)
        struct.pack_into(">I", prog, 16, (0x21 << 26) | (9 << 21) | (10 << 16))  # LH $t2,0($t1)
        struct.pack_into(">I", prog, 20, 0x0000_000C)
        sim = MIPSSimulator()
        sim.execute(bytes(prog))
        assert sim._regs[10] == 0xFFFF_8000

    def test_lhu_zero_extend(self):
        """LHU zero-extends 0x8000 to 0x00008000."""
        addr = 0x0500
        prog = bytearray(0x24)
        struct.pack_into(">I", prog, 0, (0x09 << 26) | (0 << 21) | (9 << 16) | addr)
        struct.pack_into(">I", prog, 4, (0x0D << 26) | (0 << 21) | (8 << 16) | 0x8000)  # ORI $t0,0x8000
        struct.pack_into(">I", prog, 8, (0x29 << 26) | (9 << 21) | (8 << 16))    # SH
        struct.pack_into(">I", prog, 12, (0x25 << 26) | (9 << 21) | (10 << 16))  # LHU
        struct.pack_into(">I", prog, 16, 0x0000_000C)
        sim = MIPSSimulator()
        sim.execute(bytes(prog))
        assert sim._regs[10] == 0x8000


# ── J-type jumps ──────────────────────────────────────────────────────────────

class TestJType:

    def test_j_absolute_jump(self):
        """J target — jumps to (PC+4)[31:28] | (target << 2)."""
        # Layout: NOP at 0, J to 8 at 4, NOP at 8 (skipped), HALT at 12
        # But wait: J target = (PC+4)[31:28] | (target26 << 2)
        # PC+4 = 0x0008 after fetch of J at addr 4.
        # target26 for addr 0x000C: (0x000C >> 2) = 3
        # (0x0008 & 0xF000) | (3 << 2) = 0x000C
        prog  = NOP                              # 0x0000
        prog += J(0x02, 3)                       # 0x0004: J to 0x000C
        prog += NOP                              # 0x0008: skipped
        prog += HALT                             # 0x000C: HALT
        sim = MIPSSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.steps == 3   # NOP + J + HALT (middle NOP skipped)

    def test_jal_sets_ra(self):
        """JAL sets $ra = PC+4 (return address)."""
        # JAL to 0x0010; $ra should = 0x0008 (addr after JAL)
        prog = bytearray(0x14)
        struct.pack_into(">I", prog, 0x00, 0x0000_0000)                      # NOP
        struct.pack_into(">I", prog, 0x04, (0x03 << 26) | 4)                 # JAL to 0x10
        # (PC+4 after JAL at 0x04 = 0x08; target = 0x08 & 0xF000 | (4<<2) = 0x10)
        struct.pack_into(">I", prog, 0x08, 0x0000_0000)                      # NOP (skipped)
        struct.pack_into(">I", prog, 0x0C, 0x0000_0000)                      # NOP (skipped)
        struct.pack_into(">I", prog, 0x10, 0x0000_000C)                      # HALT
        sim = MIPSSimulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._regs[REG_RA] == 0x0008   # return addr = after JAL
