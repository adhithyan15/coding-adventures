"""Per-instruction tests for AlphaSimulator.

Covers each opcode group: memory, INTA, INTL, INTS, INTM, branch, jump.
All encodings use struct.pack("<I", ...) — little-endian, unlike the prior
big-endian simulators in this series.
"""

from __future__ import annotations

import struct

from alpha_axp_simulator import AlphaSimulator

# ── Encoding helpers ──────────────────────────────────────────────────────────

def w32(v: int) -> bytes:
    return struct.pack("<I", v & 0xFFFF_FFFF)


HALT = w32(0x0000_0000)


def opa(func: int, ra: int, rb: int, rc: int) -> bytes:
    """Operate register format: op|ra|rb|0|func|rc."""
    return w32((ra << 21) | (rb << 16) | (func << 5) | rc)


def opi(op: int, func: int, ra: int, lit: int, rc: int) -> bytes:
    """Operate immediate format: op|ra|lit8|1|func|rc."""
    return w32((op << 26) | (ra << 21) | ((lit & 0xFF) << 13) | (1 << 12) | (func << 5) | rc)


def opa_op(op: int, func: int, ra: int, rb: int, rc: int) -> bytes:
    """Operate register format with explicit opcode."""
    return w32((op << 26) | (ra << 21) | (rb << 16) | (func << 5) | rc)


def mov_i(rd: int, imm8: int) -> bytes:
    """BIS r31, imm8, rd — load 8-bit zero-extended immediate."""
    return opi(0x11, 0x20, 31, imm8, rd)


def mem_op(op: int, ra: int, rb: int, disp: int) -> bytes:
    """Memory format: op|ra|rb|disp16."""
    return w32((op << 26) | (ra << 21) | (rb << 16) | (disp & 0xFFFF))


def branch(op: int, ra: int, disp21: int) -> bytes:
    """Branch format: op|ra|disp21."""
    return w32((op << 26) | (ra << 21) | (disp21 & 0x1F_FFFF))


def jump(func: int, ra: int, rb: int) -> bytes:
    """Jump format: 0x1A|ra|rb|func|hint=0."""
    return w32((0x1A << 26) | (ra << 21) | (rb << 16) | (func << 14))


# ── INTA: arithmetic ──────────────────────────────────────────────────────────

class TestINTA:

    def _run(self, prog: bytes) -> AlphaSimulator:
        sim = AlphaSimulator()
        sim.execute(prog)
        return sim

    def test_addq_basic(self):
        prog  = mov_i(1, 10)
        prog += mov_i(2, 20)
        prog += opa_op(0x10, 0x20, 1, 2, 3)   # ADDQ r1, r2, r3
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 30

    def test_addq_64bit_wraps(self):
        # Load max 8-bit into r1, shift up so r1 = 0xFFFFFFFFFFFFFF00
        # Use ADDQ to wrap around
        sim = AlphaSimulator()
        sim._regs[1] = 0xFFFF_FFFF_FFFF_FFFE
        sim._regs[2] = 3
        prog = opa_op(0x10, 0x20, 1, 2, 3) + HALT  # ADDQ r1, r2, r3
        sim.load(prog)
        sim._regs[1] = 0xFFFF_FFFF_FFFF_FFFE
        sim._regs[2] = 3
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 1   # wraps

    def test_subq_basic(self):
        prog  = mov_i(1, 50)
        prog += mov_i(2, 13)
        prog += opa_op(0x10, 0x29, 1, 2, 3)   # SUBQ r1, r2, r3
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 37

    def test_addl_sign_extends(self):
        """ADDL of 0x7FFFFFFF + 1 → 32-bit 0x80000000 → sext64 = 0xFFFFFFFF80000000."""
        sim = AlphaSimulator()
        prog = opa_op(0x10, 0x00, 1, 2, 3) + HALT   # ADDL r1, r2, r3
        sim.load(prog)
        sim._regs[1] = 0x7FFF_FFFF
        sim._regs[2] = 1
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFFFF_FFFF_8000_0000

    def test_subl_sign_extends(self):
        """SUBL 0 - 1 = -1 in 32-bit → 0xFFFFFFFFFFFFFFFF."""
        sim = AlphaSimulator()
        prog = opa_op(0x10, 0x09, 31, 31, 3) + HALT   # SUBL r31, r31, r3 → 0
        sim.load(prog)
        sim._regs[2] = 1
        # Actually test SUBL r31, r2 where r2=1 → 0 - 1 = -1 in 32 bits
        prog = opa_op(0x10, 0x09, 31, 2, 3) + HALT
        sim.load(prog)
        sim._regs[2] = 1
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFFFF_FFFF_FFFF_FFFF

    def test_mulq_basic(self):
        prog  = mov_i(1, 7)
        prog += mov_i(2, 6)
        prog += opa_op(0x10, 0x38, 1, 2, 3)   # MULQ r1, r2, r3
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 42

    def test_cmpeq_true(self):
        prog  = mov_i(1, 42)
        prog += mov_i(2, 42)
        prog += opa_op(0x10, 0x2D, 1, 2, 3)   # CMPEQ r1, r2, r3
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 1

    def test_cmpeq_false(self):
        prog  = mov_i(1, 42)
        prog += mov_i(2, 43)
        prog += opa_op(0x10, 0x2D, 1, 2, 3)
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 0

    def test_cmplt_signed_true(self):
        """CMPLT: signed comparison — -1 < 0."""
        sim = AlphaSimulator()
        prog = opa_op(0x10, 0x4D, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0xFFFF_FFFF_FFFF_FFFF   # -1 unsigned
        sim._regs[2] = 0
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 1

    def test_cmplt_signed_false(self):
        """CMPLT: 5 < 3 is false."""
        prog  = mov_i(1, 5)
        prog += mov_i(2, 3)
        prog += opa_op(0x10, 0x4D, 1, 2, 3)
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 0

    def test_cmple_equal(self):
        prog  = mov_i(1, 7)
        prog += mov_i(2, 7)
        prog += opa_op(0x10, 0x6D, 1, 2, 3)   # CMPLE
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 1

    def test_cmpult_unsigned(self):
        """CMPULT: unsigned — a large unsigned value > small."""
        sim = AlphaSimulator()
        prog = opa_op(0x10, 0x3D, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0xFFFF_FFFF_FFFF_FFFF   # huge unsigned
        sim._regs[2] = 1
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0   # NOT less than 1

    def test_addq_immediate(self):
        """ADDQ with i-bit literal."""
        prog  = mov_i(1, 10)
        # ADDQ r1, #5, r2: op=0x10, func=0x20, ra=1, lit=5, rc=2
        prog += opi(0x10, 0x20, 1, 5, 2)
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(2) == 15

    def test_s4addq(self):
        """S4ADDQ: rc = Ra*4 + src."""
        prog  = mov_i(1, 3)
        prog += mov_i(2, 1)
        prog += opa_op(0x10, 0x22, 1, 2, 3)   # S4ADDQ r1, r2, r3
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 13   # 3*4 + 1


# ── INTL: logical and CMOV ────────────────────────────────────────────────────

class TestINTL:

    def _run(self, prog: bytes) -> AlphaSimulator:
        sim = AlphaSimulator()
        sim.execute(prog)
        return sim

    def test_and(self):
        prog  = mov_i(1, 0xFF)
        prog += mov_i(2, 0x0F)
        prog += opa_op(0x11, 0x00, 1, 2, 3)   # AND
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 0x0F

    def test_bic(self):
        prog  = mov_i(1, 0xFF)
        prog += mov_i(2, 0x0F)
        prog += opa_op(0x11, 0x08, 1, 2, 3)   # BIC (AND NOT)
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 0xF0

    def test_bis_or(self):
        prog  = mov_i(1, 0xA0)
        prog += mov_i(2, 0x0B)
        prog += opa_op(0x11, 0x20, 1, 2, 3)   # BIS
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 0xAB

    def test_xor(self):
        prog  = mov_i(1, 0xFF)
        prog += mov_i(2, 0x55)
        prog += opa_op(0x11, 0x40, 1, 2, 3)   # XOR
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 0xAA

    def test_ornot(self):
        """ORNOT: r3 = r1 | ~r2."""
        prog  = mov_i(1, 0x00)
        prog += mov_i(2, 0x00)
        prog += opa_op(0x11, 0x28, 1, 2, 3)   # ORNOT
        prog += HALT
        sim = self._run(prog)
        # r1=0, r2=0 → 0 | ~0 = 0xFFFF...FFFF
        assert sim._get_reg(3) == 0xFFFF_FFFF_FFFF_FFFF

    def test_eqv_xnor(self):
        """EQV (XNOR): r3 = r1 ^ ~r2."""
        prog  = mov_i(1, 0xFF)
        prog += mov_i(2, 0xFF)
        prog += opa_op(0x11, 0x48, 1, 2, 3)   # EQV
        prog += HALT
        sim = self._run(prog)
        # 0xFF ^ ~0xFF = 0xFF ^ 0xFFFFFFFFFFFFFF00 = 0xFFFFFFFFFFFFFFFF
        assert sim._get_reg(3) == 0xFFFF_FFFF_FFFF_FFFF

    def test_cmoveq_condition_true(self):
        """CMOVEQ: Ra==0 → move src to Rc."""
        sim = AlphaSimulator()
        prog  = mov_i(3, 99)                       # r3 = 99 (initial)
        prog += opa_op(0x11, 0x24, 31, 2, 3)      # CMOVEQ r31, r2, r3
        prog += HALT
        sim.load(prog)
        sim._regs[2] = 42
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 42   # r31==0 so condition true

    def test_cmoveq_condition_false(self):
        """CMOVEQ: Ra!=0 → Rc unchanged."""
        sim = AlphaSimulator()
        prog  = mov_i(3, 99)
        prog += mov_i(1, 5)                        # r1 = 5 (non-zero)
        prog += opa_op(0x11, 0x24, 1, 2, 3)       # CMOVEQ r1, r2, r3
        prog += HALT
        sim.load(prog)
        sim._regs[2] = 42
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 99   # r1!=0, Rc unchanged

    def test_cmovne_true(self):
        prog  = mov_i(1, 7)
        prog += mov_i(3, 0)
        prog += opa_op(0x11, 0x26, 1, 2, 3)   # CMOVNE r1, r2, r3 (r1!=0 → true)
        prog += HALT
        sim = AlphaSimulator()
        sim.load(prog)
        sim._regs[2] = 55
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 55

    def test_cmovlt_negative(self):
        """CMOVLT: signed Ra < 0."""
        sim = AlphaSimulator()
        prog  = mov_i(3, 77)
        prog += opa_op(0x11, 0x44, 1, 2, 3)   # CMOVLT r1, r2, r3
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 0xFFFF_FFFF_FFFF_FFFF   # -1 signed
        sim._regs[2] = 123
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 123

    def test_cmovlbs(self):
        """CMOVLBS: low bit of Ra is set → move src."""
        sim = AlphaSimulator()
        prog  = mov_i(3, 0)
        prog += opa_op(0x11, 0x14, 1, 2, 3)   # CMOVLBS r1, r2, r3
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 0x11   # low bit set
        sim._regs[2] = 9
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 9

    def test_cmovlbc(self):
        """CMOVLBC: low bit of Ra is clear → move src."""
        sim = AlphaSimulator()
        prog  = mov_i(3, 0)
        prog += opa_op(0x11, 0x16, 1, 2, 3)
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 0x10   # low bit clear
        sim._regs[2] = 8
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 8

    def test_bis_mov_idiom(self):
        """BIS r31, imm8, rd is the standard MOV immediate."""
        result = AlphaSimulator().execute(mov_i(5, 200) + HALT)
        assert result.final_state.regs[5] == 200


# ── INTS: shift and byte manipulation ────────────────────────────────────────

class TestINTS:

    def _run(self, prog: bytes) -> AlphaSimulator:
        sim = AlphaSimulator()
        sim.execute(prog)
        return sim

    def test_sll(self):
        prog  = mov_i(1, 1)
        prog += mov_i(2, 8)
        prog += opa_op(0x12, 0x39, 1, 2, 3)   # SLL r1, r2, r3
        prog += HALT
        sim = self._run(prog)
        assert sim._get_reg(3) == 256

    def test_srl(self):
        sim = AlphaSimulator()
        prog = opa_op(0x12, 0x34, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0xFFFF_FFFF_FFFF_FF00
        sim._regs[2] = 8
        while not sim._halted:
            sim.step()
        # SRL: zero-fill → 0x00FFFFFFFFFFFFFF
        assert sim._get_reg(3) == 0x00FF_FFFF_FFFF_FFFF

    def test_sra_arithmetic(self):
        """SRA fills with sign bit."""
        sim = AlphaSimulator()
        prog = opa_op(0x12, 0x3C, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0x8000_0000_0000_0000   # high bit set
        sim._regs[2] = 4
        while not sim._halted:
            sim.step()
        # SRA 4 → 0xF800000000000000
        assert sim._get_reg(3) == 0xF800_0000_0000_0000

    def test_extbl(self):
        """EXTBL: extract byte at offset 2 (boff=16 bits)."""
        sim = AlphaSimulator()
        prog = opa_op(0x12, 0x06, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0x0000_0000_00AB_CDEF
        sim._regs[2] = 2   # byte offset 2 → boff=16 bits
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xAB   # bytes: EF CD AB 00...

    def test_insbl(self):
        """INSBL: insert byte at offset 1."""
        sim = AlphaSimulator()
        prog = opa_op(0x12, 0x0B, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0x42   # byte to insert
        sim._regs[2] = 1      # boff = 1*8 = 8 bits
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0x4200   # byte placed at position 1

    def test_mskbl(self):
        """MSKBL: zero the byte at offset 0."""
        sim = AlphaSimulator()
        prog = opa_op(0x12, 0x02, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0xDEAD_BEEF_1234_5678
        sim._regs[2] = 0   # byte offset 0
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xDEAD_BEEF_1234_5600

    def test_zap(self):
        """ZAP: zero byte where mask bit is set."""
        sim = AlphaSimulator()
        prog = opa_op(0x12, 0x30, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0xFF_FF_FF_FF_FF_FF_FF_FF
        sim._regs[2] = 0b0000_0001   # zero byte 0 only
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFF_FF_FF_FF_FF_FF_FF_00

    def test_zapnot(self):
        """ZAPNOT: keep byte where mask bit is set."""
        sim = AlphaSimulator()
        prog = opa_op(0x12, 0x31, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0xFF_FF_FF_FF_FF_FF_FF_FF
        sim._regs[2] = 0b0000_0001   # keep only byte 0
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0x00_00_00_00_00_00_00_FF

    def test_sextb_positive(self):
        prog = opa_op(0x12, 0x00, 1, 31, 3) + HALT
        sim = AlphaSimulator()
        sim.load(prog)
        sim._regs[1] = 0x7F
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0x7F

    def test_sextb_negative(self):
        prog = opa_op(0x12, 0x00, 1, 31, 3) + HALT
        sim = AlphaSimulator()
        sim.load(prog)
        sim._regs[1] = 0x80   # -128 when signed byte
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFFFF_FFFF_FFFF_FF80

    def test_sextw_negative(self):
        prog = opa_op(0x12, 0x01, 1, 31, 3) + HALT
        sim = AlphaSimulator()
        sim.load(prog)
        sim._regs[1] = 0x8000   # -32768 signed word
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFFFF_FFFF_FFFF_8000


# ── INTM: multiply ────────────────────────────────────────────────────────────

class TestINTM:

    def test_mulq(self):
        sim = AlphaSimulator()
        prog = opa_op(0x13, 0x20, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0x1_0000_0000   # 2^32
        sim._regs[2] = 0x1_0000_0000
        while not sim._halted:
            sim.step()
        # 2^32 * 2^32 = 2^64 → lower 64 bits = 0
        assert sim._get_reg(3) == 0

    def test_umulh(self):
        """UMULH: upper 64 bits of unsigned product."""
        sim = AlphaSimulator()
        prog = opa_op(0x13, 0x30, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0x1_0000_0000   # 2^32
        sim._regs[2] = 0x1_0000_0000   # 2^32
        while not sim._halted:
            sim.step()
        # 2^32 * 2^32 = 2^64 → upper 64 bits = 1
        assert sim._get_reg(3) == 1

    def test_mull_sign_extend(self):
        """MULL wraps at 32 bits and sign-extends."""
        sim = AlphaSimulator()
        prog = opa_op(0x13, 0x00, 1, 2, 3) + HALT
        sim.load(prog)
        sim._regs[1] = 0x7FFF_FFFF
        sim._regs[2] = 2
        while not sim._halted:
            sim.step()
        # 0x7FFFFFFF * 2 = 0xFFFFFFFE → sext32 = 0xFFFFFFFFFFFFFFFE
        assert sim._get_reg(3) == 0xFFFF_FFFF_FFFF_FFFE


# ── Memory ────────────────────────────────────────────────────────────────────

class TestMemory:

    def test_ldq_stq_roundtrip(self):
        sim = AlphaSimulator()
        # r1 = base address (0x100), r2 = value, r3 = destination
        prog  = mem_op(0x2D, 2, 1, 0)               # STQ r2, 0(r1)
        prog += mem_op(0x29, 3, 1, 0)               # LDQ r3, 0(r1)
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 0x100    # aligned address
        sim._regs[2] = 0xDEAD_BEEF_CAFE_BABE
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xDEAD_BEEF_CAFE_BABE

    def test_ldl_sign_extends(self):
        """LDL sign-extends 32-bit value to 64 bits."""
        sim = AlphaSimulator()
        prog = mem_op(0x28, 3, 1, 0) + HALT   # LDL r3, 0(r1)
        sim.load(prog)
        sim._regs[1] = 0x100
        # Write 0x80000000 at address 0x100 (little-endian)
        sim._mem[0x100] = 0x00
        sim._mem[0x101] = 0x00
        sim._mem[0x102] = 0x00
        sim._mem[0x103] = 0x80
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFFFF_FFFF_8000_0000

    def test_ldl_positive_no_extend(self):
        """LDL with positive 32-bit value just zero-extends."""
        sim = AlphaSimulator()
        prog = mem_op(0x28, 3, 1, 0) + HALT
        sim.load(prog)
        sim._regs[1] = 0x100
        sim._mem[0x100] = 0x78
        sim._mem[0x101] = 0x56
        sim._mem[0x102] = 0x34
        sim._mem[0x103] = 0x12   # 0x12345678 — positive
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0x1234_5678

    def test_ldbu_zero_extends(self):
        sim = AlphaSimulator()
        prog = mem_op(0x0A, 3, 1, 0) + HALT   # LDBU r3, 0(r1)
        sim.load(prog)
        sim._regs[1] = 0x50
        sim._mem[0x50] = 0xFF
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0xFF   # zero-extended, not sign-extended

    def test_ldwu_zero_extends(self):
        sim = AlphaSimulator()
        prog = mem_op(0x0C, 3, 1, 0) + HALT   # LDWU r3, 0(r1)
        sim.load(prog)
        sim._regs[1] = 0x50
        sim._mem[0x50] = 0xFF
        sim._mem[0x51] = 0x80
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 0x80FF   # zero-extended

    def test_stl_only_32_bits(self):
        """STL writes only the low 32 bits."""
        sim = AlphaSimulator()
        prog = mem_op(0x2C, 2, 1, 0) + HALT   # STL r2, 0(r1)
        sim.load(prog)
        sim._regs[1] = 0x200
        sim._regs[2] = 0xCAFE_BABE_1234_5678
        while not sim._halted:
            sim.step()
        # Only low 32 bits stored: 0x12345678
        assert sim._mem[0x200] == 0x78
        assert sim._mem[0x201] == 0x56
        assert sim._mem[0x202] == 0x34
        assert sim._mem[0x203] == 0x12

    def test_stb_stw(self):
        sim = AlphaSimulator()
        prog  = mem_op(0x0E, 2, 1, 0)    # STB r2, 0(r1)
        prog += mem_op(0x0D, 3, 1, 2)    # STW r3, 2(r1)
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 0x300
        sim._regs[2] = 0xAB
        sim._regs[3] = 0xCDEF
        while not sim._halted:
            sim.step()
        assert sim._mem[0x300] == 0xAB
        assert sim._mem[0x302] == 0xEF
        assert sim._mem[0x303] == 0xCD

    def test_ldq_with_negative_displacement(self):
        """Negative displacement (sign-extended) addresses lower memory."""
        sim = AlphaSimulator()
        prog = mem_op(0x29, 3, 1, 0xFFF8) + HALT   # LDQ r3, -8(r1)  (0xFFF8 = -8)
        sim.load(prog)
        sim._regs[1] = 0x108   # ea = 0x108 - 8 = 0x100
        sim._mem[0x100] = 0x01
        for i in range(1, 8):
            sim._mem[0x100 + i] = 0
        while not sim._halted:
            sim.step()
        assert sim._get_reg(3) == 1

    def test_unaligned_ldq_raises(self):
        prog = mem_op(0x29, 3, 1, 0) + HALT
        sim = AlphaSimulator()
        sim.load(prog)
        sim._regs[1] = 0x101   # unaligned
        trace = sim.step()
        assert "Unaligned" in trace.mnemonic


# ── Branches ──────────────────────────────────────────────────────────────────

class TestBranches:

    def test_br_unconditional(self):
        """BR always branches."""
        # Program: BR to skip r1=1; r1=1; HALT
        # Layout: addr 0: BR disp21=1; addr 4: MOV r1,1; addr 8: HALT
        # target = (0+4) + 1*4 = 8 → skips MOV
        prog  = branch(0x30, 31, 1)   # BR r31, +1
        prog += mov_i(1, 1)           # skipped
        prog += HALT
        result = AlphaSimulator().execute(prog)
        assert result.final_state.regs[1] == 0

    def test_beq_taken(self):
        """BEQ taken when r1==0."""
        # addr 0: BEQ r1, +1; addr 4: MOV r2,99; addr 8: HALT
        prog  = branch(0x39, 1, 1)    # BEQ r1, +1 (skip if r1==0)
        prog += mov_i(2, 99)
        prog += HALT
        result = AlphaSimulator().execute(prog)
        assert result.final_state.regs[2] == 0   # skipped

    def test_beq_not_taken(self):
        """BEQ not taken when r1!=0."""
        sim = AlphaSimulator()
        prog  = branch(0x39, 1, 1)
        prog += mov_i(2, 99)
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 5
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 99   # not skipped

    def test_bne_taken(self):
        sim = AlphaSimulator()
        prog  = branch(0x3D, 1, 1)    # BNE r1, +1
        prog += mov_i(2, 77)
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 1              # non-zero → taken
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0   # skipped

    def test_blt_taken(self):
        """BLT taken when signed Ra < 0."""
        sim = AlphaSimulator()
        prog  = branch(0x3A, 1, 1)    # BLT r1, +1
        prog += mov_i(2, 55)
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 0xFFFF_FFFF_FFFF_FFFF   # -1
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0   # skipped

    def test_ble_taken_on_zero(self):
        """BLE taken when Ra==0."""
        prog  = branch(0x3B, 31, 1)   # BLE r31, +1 (r31=0 → signed ≤ 0)
        prog += mov_i(2, 44)
        prog += HALT
        result = AlphaSimulator().execute(prog)
        assert result.final_state.regs[2] == 0   # skipped

    def test_bgt_taken(self):
        """BGT taken when signed Ra > 0."""
        sim = AlphaSimulator()
        prog  = branch(0x3F, 1, 1)    # BGT r1, +1
        prog += mov_i(2, 33)
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 1              # positive
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0   # skipped

    def test_bge_taken_on_zero(self):
        prog  = branch(0x3E, 31, 1)   # BGE r31, +1 (r31=0 ≥ 0)
        prog += mov_i(2, 22)
        prog += HALT
        result = AlphaSimulator().execute(prog)
        assert result.final_state.regs[2] == 0

    def test_blbc_taken_when_even(self):
        """BLBC taken when low bit is clear (even number)."""
        sim = AlphaSimulator()
        prog  = branch(0x38, 1, 1)    # BLBC r1, +1
        prog += mov_i(2, 11)
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 4              # even
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0

    def test_blbs_taken_when_odd(self):
        """BLBS taken when low bit is set (odd number)."""
        sim = AlphaSimulator()
        prog  = branch(0x3C, 1, 1)    # BLBS r1, +1
        prog += mov_i(2, 11)
        prog += HALT
        sim.load(prog)
        sim._regs[1] = 3              # odd
        while not sim._halted:
            sim.step()
        assert sim._get_reg(2) == 0

    def test_bsr_saves_return_address(self):
        """BSR saves PC+4 (address of instruction after BSR) into Ra."""
        # Layout:
        #   addr 0: BSR r26, +1   (ra=26, disp=1 → target = (0+4) + 1*4 = 8)
        #   addr 4: (never reached)
        #   addr 8: HALT
        prog  = branch(0x34, 26, 1)   # BSR r26, +1
        prog += mov_i(1, 0xFF)         # never reached
        prog += HALT
        result = AlphaSimulator().execute(prog)
        assert result.final_state.regs[26] == 4   # saved PC+4 = 4

    def test_br_backward(self):
        """BR with negative displacement loops back."""
        # addr 0: MOV r1, 1; addr 4: BR r31, -1 (disp=-1 → back to addr 0)
        # This will hit max_steps
        prog  = mov_i(1, 1)
        prog += branch(0x30, 31, 0x1F_FFFF)   # disp21=-1 → target=(4+4)+(-4)=4 ... wait
        # target = (pc_of_instr+4) + disp*4 = (4+4) + (-1)*4 = 4. Loop at addr4.
        # Actually disp=-1 loops to addr 4 (itself). For a loop to addr 0 we need disp=-2.
        prog += HALT
        result = AlphaSimulator().execute(prog, max_steps=5)
        assert not result.ok   # never reaches HALT


# ── Jumps ─────────────────────────────────────────────────────────────────────

class TestJumps:

    def test_jmp_basic(self):
        """JMP jumps to Rb & ~3, saves PC+4 in Ra."""
        # Place a HALT at address 0x10
        sim = AlphaSimulator()
        prog = jump(0x00, 1, 2) + HALT   # JMP r1, (r2) — jump to r2
        sim.load(prog)
        sim._mem[0x10] = 0x00  # HALT at 0x10
        sim._regs[2] = 0x10
        sim.step()   # JMP
        assert sim._pc == 0x10
        assert sim._get_reg(1) == 4   # saved return address

    def test_jsr_saves_link(self):
        """JSR saves PC+4 in Ra."""
        sim = AlphaSimulator()
        prog = jump(0x01, 26, 2) + HALT   # JSR r26, (r2)
        sim.load(prog)
        sim._mem[0x20] = 0x00
        sim._regs[2] = 0x20
        sim.step()
        assert sim._get_reg(26) == 4
        assert sim._pc == 0x20

    def test_ret_returns_via_register(self):
        """RET jumps to Rb (the saved return address)."""
        # Program:
        #   addr 0: BSR r26, +1     → calls addr 8
        #   addr 4: HALT
        #   addr 8: RET r31, (r26)  → jumps back to r26=4
        prog  = branch(0x34, 26, 1)   # BSR r26, +1 → target=(0+4)+4=8; r26=4
        prog += HALT                   # addr 4
        prog += jump(0x02, 31, 26)    # RET r31, (r26) → jumps to r26=4
        result = AlphaSimulator().execute(prog)
        assert result.ok
        assert result.final_state.pc == 4 + 4   # PC after HALT step
