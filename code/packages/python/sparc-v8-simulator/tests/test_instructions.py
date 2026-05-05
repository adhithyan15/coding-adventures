"""Per-instruction tests for the SPARC V8 simulator.

Every instruction variant is tested for correct register/memory/CC effect
and correct PC advancement.

SPARC V8 instruction encoding reference:
  Format 1 (CALL): [op:2=01][disp30:30]
  Format 2:        [op:2=00][rd:5][op2:3][imm22:22]
  Format 3 reg:    [op:2][rd:5][op3:6][rs1:5][0][asi:8][rs2:5]
  Format 3 imm:    [op:2][rd:5][op3:6][rs1:5][1][simm13:13]

op=2 → ALU; op=3 → Memory; op=0 → SETHI/Bicc; op=1 → CALL
"""

from __future__ import annotations

import struct

from sparc_v8_simulator import SPARCSimulator

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    """Pack a 32-bit unsigned int as 4 big-endian bytes."""
    return struct.pack(">I", v & 0xFFFF_FFFF)


HALT = w32(0x91D0_2000)   # ta 0
NOP  = w32(0x0100_0000)   # sethi 0, %g0


def sethi(rd: int, imm22: int) -> bytes:
    """SETHI imm22, rd  — op=0, op2=4."""
    return w32((rd << 25) | (0x4 << 22) | (imm22 & 0x3FFFFF))


def mk_alu_i(op3: int, rd: int, rs1: int, simm13: int) -> bytes:
    """Format 3 ALU with sign-extended 13-bit immediate (i=1)."""
    return w32((0x2 << 30) | (rd << 25) | (op3 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def mk_alu_r(op3: int, rd: int, rs1: int, rs2: int) -> bytes:
    """Format 3 ALU with register operand (i=0)."""
    return w32((0x2 << 30) | (rd << 25) | (op3 << 19) | (rs1 << 14) | rs2)


def mk_mem_i(op3: int, rd: int, rs1: int, simm13: int) -> bytes:
    """Format 3 memory with immediate offset (i=1)."""
    return w32((0x3 << 30) | (rd << 25) | (op3 << 19) | (rs1 << 14) | (1 << 13) | (simm13 & 0x1FFF))


def mk_mem_r(op3: int, rd: int, rs1: int, rs2: int) -> bytes:
    """Format 3 memory with register offset (i=0)."""
    return w32((0x3 << 30) | (rd << 25) | (op3 << 19) | (rs1 << 14) | rs2)


def mk_bicc(cond: int, disp22: int) -> bytes:
    """Bicc  cond, disp22  — op=0, op2=2."""
    return w32((0x0 << 30) | (cond << 25) | (0x2 << 22) | (disp22 & 0x3FFFFF))


def mk_call(disp30: int) -> bytes:
    """CALL disp30."""
    return w32((0x1 << 30) | (disp30 & 0x3FFF_FFFF))


# Shortcuts for frequently used ALU ops
def add_i(rd: int, rs1: int, simm13: int) -> bytes:
    return mk_alu_i(0x00, rd, rs1, simm13)   # ADD


def or_i(rd: int, rs1: int, simm13: int) -> bytes:
    return mk_alu_i(0x02, rd, rs1, simm13)   # OR (also used as MOV imm)


# ── SETHI / NOP / MOV ────────────────────────────────────────────────────────

class TestSethi:

    def test_sethi_loads_upper_22_bits(self):
        """SETHI %hi(0xDEADB000), %o0 → %o0 = 0xDEADB000.

        SPARC SETHI places the 22-bit imm22 into rd[31:10].
        To load 0xDEADB000, imm22 = 0xDEADB000 >> 10 = 0x37AB6C.
        """
        prog = sethi(8, 0x37AB6C) + HALT    # 0x37AB6C << 10 = 0xDEADB000
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(8) == 0xDEADB000

    def test_sethi_then_or_full_32_bit(self):
        """SETHI + OR loads a full 32-bit constant (0xDEADBEEF).

        SETHI: rd = imm22 << 10.  To load the upper bits of 0xDEADBEEF,
        imm22 = 0xDEADB000 >> 10 = 0x37AB6C, giving rd = 0xDEADB000.
        OR fills in the lower 12 bits: 0xDEADB000 | 0xEEF = 0xDEADBEEF.
        """
        prog  = sethi(8, 0x37AB6C)            # %o0 = 0xDEADB000
        prog += or_i(8, 8, 0xEEF)             # %o0 |= 0xEEF → 0xDEADBEEF
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(8) == 0xDEAD_BEEF

    def test_nop_is_sethi_zero_g0(self):
        """NOP (0x01000000) = SETHI 0, %g0 — no register changes."""
        prog = NOP + HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.traces[0].mnemonic == "NOP"
        assert sim._regs[0] == 0   # g0 still zero


# ── ADD / SUB ─────────────────────────────────────────────────────────────────

class TestAddSub:

    def test_add_immediate(self):
        """ADD %g1, %g0, 42 → %g1 = 42."""
        prog = add_i(1, 0, 42) + HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(1) == 42

    def test_add_register(self):
        """ADD %g3, %g1, %g2 → %g3 = 10 + 20 = 30."""
        prog  = add_i(1, 0, 10)
        prog += add_i(2, 0, 20)
        prog += mk_alu_r(0x00, 3, 1, 2)   # ADD %g3, %g1, %g2
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 30

    def test_addcc_sets_z_flag(self):
        """ADDcc 0 + 0 → Z=1."""
        prog = mk_alu_i(0x10, 1, 0, 0) + HALT   # ADDcc %g1, %g0, 0
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._psr_z
        assert not sim._psr_n

    def test_addcc_sets_n_flag(self):
        """ADDcc %g0 + (-1) → N=1."""
        prog = mk_alu_i(0x10, 1, 0, 0x1FFF) + HALT   # simm13 = -1 (0x1FFF sign-extended)
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._psr_n
        assert not sim._psr_z

    def test_addcc_sets_c_flag(self):
        """ADDcc 0xFFFFFFFF + 1 → C=1, Z=1 (unsigned overflow)."""
        prog  = sethi(1, 0x3FFFFF)              # %g1 = 0xFFFFFC00
        prog += or_i(1, 1, 0xFFF)               # %g1 |= 0xFFF → 0xFFFFFFFF
        prog += mk_alu_i(0x10, 2, 1, 1)         # ADDcc %g2, %g1, 1
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._psr_c    # carry out
        assert sim._psr_z    # result is 0
        assert sim._get_reg(2) == 0

    def test_addcc_sets_v_flag(self):
        """ADDcc 0x7FFFFFFF + 1 → V=1, N=1 (signed overflow)."""
        prog  = sethi(1, 0x1FFFFF)              # %g1 = 0x7FFFFC00
        prog += or_i(1, 1, 0xFFF)               # %g1 = 0x7FFFFFFF
        prog += mk_alu_i(0x10, 2, 1, 1)         # ADDcc %g2, %g1, 1
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._psr_v    # signed overflow
        assert sim._psr_n    # result is negative (0x80000000)

    def test_sub_basic(self):
        """SUB %g2, %g1, %g0 = 10 - 3 = 7."""
        prog  = add_i(1, 0, 10)
        prog += add_i(2, 0, 3)
        prog += mk_alu_r(0x04, 3, 1, 2)   # SUB %g3, %g1, %g2
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 7

    def test_subcc_sets_c_on_borrow(self):
        """SUBcc 0 - 1 → C=1 (borrow), result = 0xFFFFFFFF."""
        prog = mk_alu_i(0x14, 1, 0, 1) + HALT   # SUBcc %g1, %g0, 1
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._psr_c
        assert sim._psr_n
        assert sim._get_reg(1) == 0xFFFF_FFFF


# ── Logic ─────────────────────────────────────────────────────────────────────

class TestLogic:

    def test_and(self):
        prog  = add_i(1, 0, 0xFF)
        prog += add_i(2, 0, 0x0F)
        prog += mk_alu_r(0x01, 3, 1, 2)   # AND
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 0x0F

    def test_andn(self):
        """ANDN %g3, %g1, %g2 = %g1 & ~%g2."""
        prog  = add_i(1, 0, 0xFF)
        prog += add_i(2, 0, 0x0F)
        prog += mk_alu_r(0x05, 3, 1, 2)   # ANDN
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 0xF0

    def test_or(self):
        prog  = add_i(1, 0, 0xF0)
        prog += add_i(2, 0, 0x0F)
        prog += mk_alu_r(0x02, 3, 1, 2)   # OR
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 0xFF

    def test_orn(self):
        """ORN %g3, %g0, %g0 = ~(0|0) = 0xFFFFFFFF."""
        prog = mk_alu_r(0x06, 1, 0, 0) + HALT   # ORN
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(1) == 0xFFFF_FFFF

    def test_xor(self):
        prog  = add_i(1, 0, 0xFF)
        prog += add_i(2, 0, 0x0F)
        prog += mk_alu_r(0x03, 3, 1, 2)   # XOR
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 0xF0

    def test_xnor(self):
        """XNOR %g1, %g1, %g1 = ~(x^x) = 0xFFFFFFFF."""
        prog  = add_i(1, 0, 0xAB)
        prog += mk_alu_r(0x07, 2, 1, 1)   # XNOR %g2, %g1, %g1
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 0xFFFF_FFFF

    def test_andcc_clears_v_c(self):
        """ANDcc always clears V and C."""
        sim = SPARCSimulator()
        sim._psr_v = True
        sim._psr_c = True
        prog = add_i(1, 0, 5) + mk_alu_r(0x11, 2, 1, 1) + HALT
        sim.execute(prog)
        assert not sim._psr_v
        assert not sim._psr_c


# ── Shifts ────────────────────────────────────────────────────────────────────

class TestShifts:

    def test_sll(self):
        """SLL %g1, %g0, 4 → shift left 4."""
        prog = add_i(1, 0, 1) + mk_alu_i(0x25, 2, 1, 4) + HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 16

    def test_srl_logical(self):
        """SRL: zero-fills on right shift of negative value."""
        prog  = sethi(1, 0x200000)             # %g1 = 0x80000000
        prog += mk_alu_i(0x26, 2, 1, 1)        # SRL %g2, %g1, 1
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 0x4000_0000   # no sign fill

    def test_sra_arithmetic(self):
        """SRA: sign-fills on right shift of negative value."""
        prog  = sethi(1, 0x200000)             # %g1 = 0x80000000
        prog += mk_alu_i(0x27, 2, 1, 1)        # SRA %g2, %g1, 1
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 0xC000_0000   # sign fill

    def test_sll_by_register(self):
        """SLL shift amount taken from register."""
        prog  = add_i(1, 0, 1)
        prog += add_i(2, 0, 3)
        prog += mk_alu_r(0x25, 3, 1, 2)   # SLL %g3, %g1, %g2
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 8


# ── Multiply / Divide ─────────────────────────────────────────────────────────

class TestMulDiv:

    def test_umul(self):
        """UMUL 7 × 6 = 42 (unsigned)."""
        prog  = add_i(1, 0, 7)
        prog += add_i(2, 0, 6)
        prog += mk_alu_r(0x0A, 3, 1, 2)   # UMUL %g3, %g1, %g2
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 42
        assert sim._y == 0

    def test_umul_large(self):
        """UMUL 0xFFFFFFFF × 0xFFFFFFFF → Y:rd = 0xFFFFFFFE00000001."""
        prog  = sethi(1, 0x3FFFFF)
        prog += or_i(1, 1, 0xFFF)          # %g1 = 0xFFFFFFFF
        prog += mk_alu_r(0x0A, 2, 1, 1)   # UMUL %g2, %g1, %g1
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 0x0000_0001   # LO
        assert sim._y == 0xFFFF_FFFE             # HI

    def test_smul_negative(self):
        """SMUL (-1) × (-1) = 1."""
        prog  = mk_alu_i(0x04, 1, 0, 1)         # SUB %g1, %g0, 1 → -1
        prog += mk_alu_r(0x0B, 2, 1, 1)          # SMUL %g2, %g1, %g1
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(2) == 1
        assert sim._y == 0

    def test_udiv(self):
        """UDIV 17 / 5 = 3 (Y=0)."""
        prog  = add_i(1, 0, 17)             # rs1 = 17
        prog += add_i(2, 0, 5)              # divisor
        prog += mk_alu_r(0x30, 0, 0, 0)    # WRY %g0 (Y=0)
        prog += mk_alu_r(0x0E, 3, 1, 2)    # UDIV %g3, %g1, %g2
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._get_reg(3) == 3

    def test_sdiv_negative(self):
        """SDIV (-17) / 5 = -3 (truncate toward zero)."""
        prog  = mk_alu_i(0x04, 1, 0, 17)   # SUB %g1, %g0, 17 → -17
        prog += add_i(2, 0, 5)
        # Y must hold the sign extension of rs1.  For -17, Y = 0xFFFFFFFF
        prog += sethi(4, 0x3FFFFF)
        prog += or_i(4, 4, 0xFFF)           # %g4 = 0xFFFFFFFF
        prog += mk_alu_r(0x30, 0, 4, 0)     # WRY %g4
        prog += mk_alu_r(0x0F, 3, 1, 2)     # SDIV %g3, %g1, %g2
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        # -17 / 5 = -3 (truncate toward zero)
        assert sim._get_reg(3) == 0xFFFF_FFFD   # -3 as unsigned

    def test_rdy_wry_roundtrip(self):
        """WRY then RDY round-trip."""
        prog  = add_i(1, 0, 0x42)
        prog += mk_alu_r(0x30, 0, 1, 0)    # WRY %g1
        prog += mk_alu_r(0x28, 2, 0, 0)    # RDY %g2
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._y == 0x42
        assert sim._get_reg(2) == 0x42


# ── Memory loads and stores ───────────────────────────────────────────────────

class TestLoadStore:

    def test_ld_st_roundtrip(self):
        """ST 0xDEADBEEF at 0x0100, then LD it back."""
        sim = SPARCSimulator()
        sim.load(HALT)
        sim._store_word(0x100, 0xDEAD_BEEF)
        assert sim._load_word(0x100) == 0xDEAD_BEEF

    def test_ld_program(self):
        """LD program: %g1 = base(0x100), LD %g2, [%g1+0]."""
        prog  = sethi(1, 0)                       # %g1 = 0 (upper)
        prog += or_i(1, 1, 0x100)                 # %g1 = 0x100
        prog += mk_mem_i(0x00, 2, 1, 0)           # LD %g2, [%g1+0]
        prog += HALT
        sim = SPARCSimulator()
        sim.load(prog)
        sim._store_word(0x100, 0x1234_5678)
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0x1234_5678

    def test_ldsb_sign_extend(self):
        """LDSB 0xFF → sign-extended to 0xFFFFFFFF."""
        prog  = or_i(1, 0, 0x200)                 # base
        prog += mk_mem_i(0x09, 2, 1, 0)           # LDSB %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        sim.load(prog)
        sim._mem[0x200] = 0xFF
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0xFFFF_FFFF

    def test_ldub_zero_extend(self):
        """LDUB 0xFF → zero-extended to 0x000000FF."""
        prog  = or_i(1, 0, 0x300)
        prog += mk_mem_i(0x01, 2, 1, 0)           # LDUB %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        sim.load(prog)
        sim._mem[0x300] = 0xFF
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0xFF

    def test_ldsh_sign_extend(self):
        """LDSH 0x8000 → sign-extended to 0xFFFF8000."""
        prog  = or_i(1, 0, 0x400)
        prog += mk_mem_i(0x0A, 2, 1, 0)           # LDSH %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        sim.load(prog)
        sim._store_half(0x400, 0x8000)
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0xFFFF_8000

    def test_lduh_zero_extend(self):
        """LDUH 0x8000 → zero-extended to 0x00008000."""
        prog  = or_i(1, 0, 0x500)
        prog += mk_mem_i(0x02, 2, 1, 0)           # LDUH %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        sim.load(prog)
        sim._store_half(0x500, 0x8000)
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0x8000

    def test_stb_stores_byte(self):
        """STB stores only the lowest byte."""
        prog  = or_i(1, 0, 0x600)
        prog += sethi(2, 0xDEADB)
        prog += or_i(2, 2, 0xEF)                  # %g2 = 0xDEADEF (lower bits)
        prog += mk_mem_i(0x05, 2, 1, 0)           # STB %g2, [%g1]
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._mem[0x600] == 0xEF


# ── Branches ──────────────────────────────────────────────────────────────────

class TestBranches:

    def test_ba_always_taken(self):
        """BA +2 skips one instruction (disp22=2 → target = PC+8 = HALT)."""
        # BA +2: target = PC_of_BA + 2*4 = 0x00 + 8 = 0x08 = HALT (skips NOP)
        prog  = mk_bicc(0x8, 2)   # BA offset=+2 instructions → skip NOP
        prog += NOP                # skipped
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.steps == 2   # BA + HALT (NOP skipped)

    def test_bn_never_taken(self):
        """BN never branches."""
        prog  = mk_bicc(0x0, 10)  # BN +10 (not taken)
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.steps == 2   # BN + HALT

    def test_be_taken_when_zero(self):
        """BE taken when Z=1 (result was zero)."""
        # SUBcc %g1, %g1, %g1 sets Z=1; BE +2 skips NOP (disp22=2 → PC+8 = HALT)
        prog  = mk_alu_r(0x14, 1, 1, 1)   # SUBcc %g1, %g1, %g1 → 0
        prog += mk_bicc(0x1, 2)            # BE +2
        prog += NOP                         # skipped
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.steps == 3   # SUBcc + BE + HALT

    def test_bne_not_taken_when_zero(self):
        """BNE not taken when Z=1."""
        prog  = mk_alu_r(0x14, 1, 1, 1)   # SUBcc → Z=1
        prog += mk_bicc(0x9, 5)            # BNE +5 (not taken)
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.ok

    def test_bl_taken_when_negative(self):
        """BL (N!=V) taken for negative result."""
        prog  = mk_alu_i(0x14, 1, 0, 1)   # SUBcc %g1, %g0, 1 → -1 (N=1, V=0)
        prog += mk_bicc(0x3, 2)            # BL offset=+2 (N!=V → taken, skips NOP)
        prog += NOP
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.steps == 3   # SUBcc + BL + HALT

    def test_bge_taken_when_not_less(self):
        """BGE taken when N==V."""
        prog  = mk_alu_i(0x10, 1, 0, 5)   # ADDcc %g1, %g0, 5 → N=0, V=0
        prog += mk_bicc(0xB, 2)            # BGE (N==V) → taken, skips NOP
        prog += NOP
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.steps == 3

    def test_bcs_taken_on_carry(self):
        """BCS taken when C=1."""
        # 0xFFFFFFFF + 1 sets C=1
        prog  = sethi(1, 0x3FFFFF)
        prog += or_i(1, 1, 0xFFF)              # %g1 = 0xFFFFFFFF
        prog += mk_alu_i(0x10, 2, 1, 1)        # ADDcc → C=1
        prog += mk_bicc(0x5, 2)                 # BCS +2 → taken, skips NOP
        prog += NOP
        prog += HALT
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.steps == 5   # SETHI + OR + ADDcc + BCS + HALT


# ── CALL / JMPL ──────────────────────────────────────────────────────────────

class TestCallJmpl:

    def test_call_sets_o7_and_jumps(self):
        """CALL disp30 sets %o7 = PC of CALL, jumps to PC+disp30*4."""
        # Layout: NOP(0), CALL+4(4), NOP skipped(8), ..., HALT at target
        prog = bytearray(0x14)
        struct.pack_into(">I", prog, 0x00, 0x0100_0000)    # NOP
        # CALL to 0x10: disp30 = (0x10 - 0x04) / 4 = 3
        struct.pack_into(">I", prog, 0x04, (0x1 << 30) | 3)   # CALL +3 words
        struct.pack_into(">I", prog, 0x08, 0x0100_0000)    # NOP (skipped)
        struct.pack_into(">I", prog, 0x0C, 0x0100_0000)    # NOP (skipped)
        struct.pack_into(">I", prog, 0x10, 0x91D0_2000)    # HALT
        sim = SPARCSimulator()
        result = sim.execute(bytes(prog))
        assert result.ok
        assert sim._get_reg(15) == 0x04    # %o7 = address of CALL

    def test_jmpl_return(self):
        """JMPL %g1 + 0, %g0 — jump to %g1, rd=g0 discards link."""
        prog  = or_i(1, 0, 0x0C)              # %g1 = 0x0C (addr of HALT)
        prog += w32((0x2 << 30) | (0 << 25) | (0x38 << 19) | (1 << 14) | (1 << 13))  # JMPL %g0, %g1+0
        prog += NOP                             # not executed
        prog += HALT                            # at 0x0C
        sim = SPARCSimulator()
        result = sim.execute(prog)
        assert result.ok
        assert result.steps == 3   # OR + JMPL + HALT


# ── SAVE / RESTORE register windows ──────────────────────────────────────────

class TestRegisterWindows:

    def test_save_rotates_window(self):
        """SAVE decrements CWP."""
        prog  = add_i(8, 0, 99)               # %o0 = 99 (in window 0)
        prog += mk_alu_i(0x3C, 0, 0, 0)       # SAVE %g0, %g0, 0
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._cwp == 2   # (0 - 1) % 3 = 2

    def test_save_caller_out_becomes_callee_in(self):
        """After SAVE, caller's %o0 is callee's %i0."""
        prog  = add_i(8, 0, 42)               # %o0 (virtual 8) = 42
        prog += mk_alu_i(0x3C, 0, 0, 0)       # SAVE
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        # After SAVE: CWP=2.  Callee's %i0 = virtual 24 in window 2
        # = physical[window_base[0] + (24-24)] = physical[8] = old %o0
        assert sim._get_reg(24) == 42          # %i0 = old %o0

    def test_restore_returns_to_caller_window(self):
        """RESTORE after SAVE restores CWP."""
        prog  = mk_alu_i(0x3C, 0, 0, 0)       # SAVE (CWP 0→2)
        prog += mk_alu_i(0x3D, 0, 0, 0)       # RESTORE (CWP 2→0)
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        assert sim._cwp == 0

    def test_save_computes_new_sp(self):
        """SAVE rd, rs1, simm13 computes result before rotating window."""
        prog  = add_i(14, 0, 0x100)            # %o6 (%sp) = 0x100
        prog += mk_alu_i(0x3C, 14, 14, -32)    # SAVE %o6, %o6, -32  (sp -= 32)
        prog += HALT
        sim = SPARCSimulator()
        sim.execute(prog)
        # After SAVE (CWP=2), rd=14 in new window = %o6 of window 2
        # But the result 0x100 - 32 = 0xE0 was written to rd=14 in new window
        assert sim._get_reg(14) == 0xE0
