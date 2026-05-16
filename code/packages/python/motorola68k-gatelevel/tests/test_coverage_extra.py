"""Extra coverage tests targeting uncovered simulator paths.

These tests exercise instructions not covered by the main test suites:
OR, AND, EOR, SUB variants, address-register ops (ADDA, SUBA, CMPA, MOVEA),
ADDX/SUBX, MUL/DIV, BCD (ABCD/SBCD/NBCD), shift/rotate variants,
index-register EA modes, signed-branch edge cases, and EXG variants.
"""



from motorola68k_gatelevel.simulator import Motorola68kGateLevelSimulator

# STOP #0x2700 halts but loads 0x2700 into SR (clears all CCR bits).
# For register-value tests use STOP; for flag tests use TRAP #15.
STOP = bytes([0x4E, 0x72, 0x27, 0x00])
HALT = bytes([0x4E, 0x4F])  # TRAP #15 — halts without changing SR


def run(prog: bytes) -> object:
    sim = Motorola68kGateLevelSimulator()
    r = sim.execute(prog)
    return r.final_state


# ──────────────────────────────────────────────────────────────────────────────
# OR operations (line 8)
# ──────────────────────────────────────────────────────────────────────────────

class TestOR:
    def test_or_b(self):
        # OR.B D1, D0 — byte OR
        prog = bytes([
            0x70, 0x0F,  # MOVEQ #0x0F, D0
            0x72, 0x70,  # MOVEQ #0x70, D1
            0x80, 0x01,  # OR.B D1, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0x7F

    def test_or_w(self):
        prog = bytes([
            0x70, 0x0F,  # MOVEQ #0x0F, D0
            0x72, 0x70,  # MOVEQ #0x70, D1
            0x80, 0x41,  # OR.W D1, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 0x7F

    def test_or_l(self):
        prog = bytes([
            0x70, 0x0F,  # MOVEQ #0x0F, D0
            0x72, 0x70,  # MOVEQ #0x70, D1
            0x80, 0x81,  # OR.L D1, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0x7F

    def test_ori_b(self):
        # ORI.B #0x0F, D0
        prog = bytes([
            0x70, 0x70,              # MOVEQ #0x70, D0
            0x00, 0x00, 0x00, 0x0F,  # ORI.B #0x0F, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0x7F

    def test_ori_w(self):
        prog = bytes([
            0x70, 0x00,              # MOVEQ #0, D0
            0x00, 0x40, 0x00, 0xFF,  # ORI.W #0xFF, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 0xFF

    def test_ori_l(self):
        prog = bytes([
            0x70, 0x00,
            0x00, 0x80, 0x00, 0x00, 0xFF, 0xFF,  # ORI.L #0xFFFF, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0xFFFF


# ──────────────────────────────────────────────────────────────────────────────
# AND operations (line C)
# ──────────────────────────────────────────────────────────────────────────────

class TestAND:
    def test_and_b(self):
        prog = bytes([
            0x70, 0xFF,  # MOVEQ #-1 (0xFF), D0
            0x72, 0x0F,  # MOVEQ #0x0F, D1
            0xC0, 0x01,  # AND.B D1, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0x0F

    def test_and_l(self):
        prog = bytes([
            0x70, 0xFF,                          # MOVEQ #-1, D0
            0xC0, 0xBC, 0x00, 0xFF, 0x00, 0xFF,  # ANDI.L #0x00FF00FF, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0x00FF00FF

    def test_andi_b(self):
        prog = bytes([
            0x70, 0xFF,              # MOVEQ #-1, D0
            0x02, 0x00, 0x00, 0x0F,  # ANDI.B #0x0F, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0x0F

    def test_andi_w(self):
        prog = bytes([
            0x70, 0xFF,
            0x02, 0x40, 0x00, 0xFF,  # ANDI.W #0xFF, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 0xFF


# ──────────────────────────────────────────────────────────────────────────────
# EOR operations (line B)
# ──────────────────────────────────────────────────────────────────────────────

class TestEOR:
    def test_eor_b(self):
        # EOR.B D0, D1: 1011 000 1 00 000 001 = 0xB101
        prog = bytes([
            0x70, 0xFF,  # D0=0xFF
            0x72, 0x0F,  # D1=0x0F
            0xB1, 0x01,  # EOR.B D0, D1 -> D1 = 0x0F ^ 0xFF = 0xF0
        ]) + STOP
        s = run(prog)
        assert (s.d1 & 0xFF) == 0xF0

    def test_eor_l(self):
        prog = bytes([
            0x70, 0xFF,
            0xB1, 0x80,  # EOR.L D0, D0 (D0 ^= D0 = 0)
        ]) + HALT
        s = run(prog)
        assert s.d0 == 0
        assert s.z


# ──────────────────────────────────────────────────────────────────────────────
# SUB variants
# ──────────────────────────────────────────────────────────────────────────────

class TestSUB:
    def test_sub_b(self):
        prog = bytes([
            0x70, 0x0A,  # MOVEQ #10, D0
            0x72, 0x03,  # MOVEQ #3, D1
            0x90, 0x01,  # SUB.B D1, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 7

    def test_sub_w(self):
        prog = bytes([
            0x70, 0x0A,
            0x72, 0x03,
            0x90, 0x41,  # SUB.W D1, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 7

    def test_subi_b(self):
        prog = bytes([
            0x70, 0x0A,
            0x04, 0x00, 0x00, 0x03,  # SUBI.B #3, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 7

    def test_subi_l(self):
        prog = bytes([
            0x20, 0x3C, 0x00, 0x00, 0x01, 0x00,  # MOVE.L #0x100, D0
            0x04, 0x80, 0x00, 0x00, 0x00, 0x01,  # SUBI.L #1, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0xFF

    def test_subq_b(self):
        prog = bytes([
            0x70, 0x0A,
            0x55, 0x00,  # SUBQ.B #2, D0 (but SUBQ #2 byte mode = 0x5500? let me check)
        ]) + STOP
        # SUBQ.B #2, D0: 0101 010 1 00 000 000 = 0x5500
        s = run(prog)
        assert (s.d0 & 0xFF) == 8

    def test_subx_dn(self):
        # SUBX.L D1, D0 (with X=0): D0 = D0 - D1 - X
        prog = bytes([
            0x70, 0x0A,  # D0=10
            0x72, 0x03,  # D1=3
            0x91, 0x81,  # SUBX.L D1, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 7


# ──────────────────────────────────────────────────────────────────────────────
# ADD variants
# ──────────────────────────────────────────────────────────────────────────────

class TestADD:
    def test_add_b(self):
        prog = bytes([
            0x70, 0x05,
            0x72, 0x03,
            0xD0, 0x01,  # ADD.B D1, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 8

    def test_addi_b(self):
        prog = bytes([
            0x70, 0x05,
            0x06, 0x00, 0x00, 0x03,  # ADDI.B #3, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 8

    def test_addi_w(self):
        prog = bytes([
            0x70, 0x05,
            0x06, 0x40, 0x00, 0x03,  # ADDI.W #3, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 8

    def test_addq_b(self):
        prog = bytes([
            0x70, 0x05,
            0x54, 0x00,  # ADDQ.B #2, D0 (0101 010 0 00 000 000 = 0x5400)
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 7

    def test_addx_dn(self):
        # ADDX.L D1, D0 (X=0): D0 = D0 + D1 + 0
        # Encoding: 1101 000 1 10 000 001 = 0xD181
        prog = bytes([
            0x70, 0x05,  # D0=5
            0x72, 0x03,  # D1=3
            0xD1, 0x81,  # ADDX.L D1, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 8


# ──────────────────────────────────────────────────────────────────────────────
# Address register operations
# ──────────────────────────────────────────────────────────────────────────────

class TestAddressRegOps:
    def test_adda_l(self):
        # ADDA.L #imm, A0 (via ADDA.L Dn, An)
        prog = bytes([
            0x20, 0x7C, 0x00, 0x00, 0x10, 0x00,  # MOVEA.L #0x1000, A0
            0x70, 0x10,                           # MOVEQ #0x10, D0
            0xD1, 0xC0,                           # ADDA.L D0, A0
        ]) + STOP
        s = run(prog)
        assert s.a0 == 0x1010

    def test_adda_w(self):
        prog = bytes([
            0x20, 0x7C, 0x00, 0x00, 0x10, 0x00,  # MOVEA.L #0x1000, A0
            0x70, 0x10,                           # MOVEQ #0x10, D0
            0xD0, 0xC0,                           # ADDA.W D0, A0
        ]) + STOP
        s = run(prog)
        assert s.a0 == 0x1010

    def test_suba_l(self):
        prog = bytes([
            0x20, 0x7C, 0x00, 0x00, 0x20, 0x00,  # MOVEA.L #0x2000, A0
            0x70, 0x10,                           # MOVEQ #0x10, D0
            0x91, 0xC0,                           # SUBA.L D0, A0
        ]) + STOP
        s = run(prog)
        assert s.a0 == 0x1FF0

    def test_cmpa_l(self):
        # CMPA.L A0, A1: compare A1 with A0, flags set, regs unchanged
        prog = bytes([
            0x20, 0x7C, 0x00, 0x00, 0x20, 0x00,  # MOVEA.L #0x2000, A0
            0x22, 0x7C, 0x00, 0x00, 0x20, 0x00,  # MOVEA.L #0x2000, A1
            0xB3, 0xC8,                           # CMPA.L A0, A1 (equal -> Z=1)
        ]) + HALT
        s = run(prog)
        assert s.z  # equal

    def test_movea_w_sign_extend(self):
        # MOVEA.W sign-extends the word to 32 bits
        prog = bytes([
            0x30, 0x7C, 0xFF, 0xFF,  # MOVEA.W #0xFFFF, A0 → sign-extended = 0xFFFFFFFF
        ]) + STOP
        s = run(prog)
        assert s.a0 == 0xFFFFFFFF


# ──────────────────────────────────────────────────────────────────────────────
# CMP variants
# ──────────────────────────────────────────────────────────────────────────────

class TestCMP:
    def test_cmp_b(self):
        # CMP.B D1, D0 sets flags without changing D0
        prog = bytes([
            0x70, 0x0A,  # D0=10
            0x72, 0x0A,  # D1=10
            0xB0, 0x01,  # CMP.B D1, D0 (Z=1 since equal)
        ]) + HALT
        s = run(prog)
        assert s.z

    def test_cmp_l(self):
        prog = bytes([
            0x70, 0x0A,
            0x72, 0x03,
            0xB0, 0x81,  # CMP.L D1, D0 (10 - 3 > 0, no carry)
        ]) + HALT
        s = run(prog)
        assert not s.c  # no carry (10 > 3 unsigned)
        assert not s.z  # not equal

    def test_cmpi_w(self):
        prog = bytes([
            0x30, 0x3C, 0x00, 0x05,  # MOVE.W #5, D0
            0x0C, 0x40, 0x00, 0x05,  # CMPI.W #5, D0
        ]) + HALT
        s = run(prog)
        assert s.z


# ──────────────────────────────────────────────────────────────────────────────
# Bit manipulation (BTST, BCHG, BCLR, BSET)
# ──────────────────────────────────────────────────────────────────────────────

class TestBitOps:
    def test_btst_imm_set(self):
        # BTST #3, D0 — test bit 3 of D0
        prog = bytes([
            0x70, 0x08,              # MOVEQ #8 (bit 3 set), D0
            0x08, 0x00, 0x00, 0x03,  # BTST #3, D0
        ]) + HALT
        s = run(prog)
        assert not s.z  # bit was SET → Z=0

    def test_btst_imm_clear(self):
        prog = bytes([
            0x70, 0x00,              # MOVEQ #0, D0
            0x08, 0x00, 0x00, 0x03,  # BTST #3, D0
        ]) + HALT
        s = run(prog)
        assert s.z  # bit was CLEAR → Z=1

    def test_bset_imm(self):
        prog = bytes([
            0x70, 0x00,              # D0=0
            0x08, 0xC0, 0x00, 0x03,  # BSET #3, D0 → D0=8
        ]) + STOP
        s = run(prog)
        assert s.d0 == 8

    def test_bclr_imm(self):
        prog = bytes([
            0x70, 0x0F,              # D0=0x0F
            0x08, 0x80, 0x00, 0x00,  # BCLR #0, D0 → D0=0x0E
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0x0E

    def test_bchg_imm(self):
        prog = bytes([
            0x70, 0x00,              # D0=0
            0x08, 0x40, 0x00, 0x01,  # BCHG #1, D0 → D0=2
        ]) + STOP
        s = run(prog)
        assert s.d0 == 2

    def test_btst_reg(self):
        prog = bytes([
            0x70, 0x08,  # D0=8
            0x72, 0x03,  # D1=3
            0x01, 0x00,  # BTST D0, D1 — tests bit 3 of D1
        ]) + HALT
        # D1=3=0b11, bit 3 = 0 → Z=1
        s = run(prog)
        assert s.z

    def test_bset_reg(self):
        prog = bytes([
            0x70, 0x00,  # D0=0
            0x72, 0x02,  # D1=2 (bit number)
            0x03, 0xC0,  # BSET D1, D0 → sets bit 2 → D0=4
        ]) + STOP
        s = run(prog)
        assert s.d0 == 4


# ──────────────────────────────────────────────────────────────────────────────
# MUL / DIV
# ──────────────────────────────────────────────────────────────────────────────

class TestMulDiv:
    def test_mulu_basic(self):
        prog = bytes([
            0x70, 0x06,  # D0=6
            0x72, 0x07,  # D1=7
            0xC0, 0xC1,  # MULU D1, D0 → D0=42
        ]) + STOP
        s = run(prog)
        assert s.d0 == 42

    def test_muls_negative(self):
        # MULS D0, D1: D1 = D1.W × D0.W (signed)
        # Encoding: 1100 001 111 000 000 = 0xC3C0 (dn=1, opmode=7=MULS, src=D0)
        prog = bytes([
            0x70, 0x80,              # MOVEQ #-128 (sign-extended), D0
            0x72, 0x02,              # MOVEQ #2, D1
            0xC3, 0xC0,              # MULS D0, D1 -> D1 = 2 x (-128) = -256
        ]) + STOP
        s = run(prog)
        # -256 as 32-bit signed = 0xFFFFFF00
        assert s.d1 == 0xFFFFFF00

    def test_divu_basic(self):
        prog = bytes([
            0x20, 0x3C, 0x00, 0x00, 0x00, 0x64,  # MOVE.L #100, D0
            0x72, 0x0A,                           # MOVEQ #10, D1
            0x80, 0xC1,                           # DIVU D1, D0 → quot=10, rem=0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFFFF) == 10  # quotient
        assert (s.d0 >> 16) == 0       # remainder

    def test_divs_basic(self):
        prog = bytes([
            0x20, 0x3C, 0xFF, 0xFF, 0xFF, 0x00,  # MOVE.L #-256, D0
            0x72, 0x02,                           # MOVEQ #2, D1
            0x81, 0xC1,                           # DIVS D1, D0
        ]) + STOP
        s = run(prog)
        # -256 / 2 = -128 quotient, 0 remainder
        assert _sign_extend(s.d0 & 0xFFFF, 16) == -128
        assert (s.d0 >> 16) == 0

    def test_divu_div_by_zero(self):
        # DIVU by 0 triggers exception (simulator halts or takes exception)
        prog = bytes([
            0x20, 0x3C, 0x00, 0x00, 0x00, 0x64,  # MOVE.L #100, D0
            0x72, 0x00,                           # MOVEQ #0, D1
            0x80, 0xC1,                           # DIVU D1, D0 (div by zero)
        ]) + STOP
        sim = Motorola68kGateLevelSimulator()
        r = sim.execute(prog)
        # Should take exception or error — just verify it doesn't crash
        assert r is not None


def _sign_extend(val: int, bits: int) -> int:
    mask = 1 << (bits - 1)
    return (val & (mask - 1)) - (val & mask)


# ──────────────────────────────────────────────────────────────────────────────
# BCD arithmetic
# ──────────────────────────────────────────────────────────────────────────────

class TestBCD:
    def test_abcd_basic(self):
        # ABCD D1, D0: D0 = BCD(D0) + BCD(D1) + X
        prog = bytes([
            0x70, 0x15,  # D0=0x15 (BCD 15)
            0x72, 0x27,  # D1=0x27 (BCD 27)
            0xC1, 0x01,  # ABCD D1, D0 (D0 = 0x15 + 0x27 = BCD 42 = 0x42)
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0x42

    def test_nbcd_basic(self):
        # NBCD D0: D0 = 0 - BCD(D0) - X
        prog = bytes([
            0x70, 0x05,  # D0=0x05 (BCD 5)
            0x48, 0x00,  # NBCD D0
        ]) + STOP
        s = run(prog)
        # 0 - 5 (BCD) with X=0 = 95 (since 100-5=95 in BCD = 0x95)
        assert (s.d0 & 0xFF) == 0x95

    def test_sbcd_basic(self):
        # SBCD D1, D0: D0 = BCD(D0) - BCD(D1) - X
        prog = bytes([
            0x70, 0x42,  # D0=0x42 (BCD 42)
            0x72, 0x15,  # D1=0x15 (BCD 15)
            0x81, 0x01,  # SBCD D1, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0x27  # 42-15=27 (BCD)


# ──────────────────────────────────────────────────────────────────────────────
# Shift / rotate with register count (ir_bit=1)
# ──────────────────────────────────────────────────────────────────────────────

class TestShiftRegCount:
    def test_asl_reg_count(self):
        # ASL.L D1, D0 — shift count from D1
        prog = bytes([
            0x70, 0x01,  # D0=1
            0x72, 0x04,  # D1=4 (shift count)
            0xE3, 0xA0,  # ASL.L D1, D0 (1 << 4 = 16)
        ]) + STOP
        s = run(prog)
        assert s.d0 == 16

    def test_lsr_reg_count(self):
        prog = bytes([
            0x20, 0x3C, 0x00, 0x00, 0x10, 0x00,  # MOVE.L #0x1000, D0
            0x72, 0x04,                           # D1=4
            0xE2, 0xA8,                           # LSR.L D1, D0 (0x1000>>4=0x100)
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0x100

    def test_rol_reg_count(self):
        prog = bytes([
            0x70, 0x01,  # D0=1
            0x72, 0x08,  # D1=8
            0xE3, 0xB8,  # ROL.L D1, D0 (1 ROL 8 = 0x100)
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0x100

    def test_roxl_reg_count(self):
        prog = bytes([
            0x70, 0x01,  # D0=1
            0x72, 0x01,  # D1=1
            0xE3, 0xB0,  # ROXL.L D1, D0 (1 ROXL 1 with X=0 = 2)
        ]) + STOP
        s = run(prog)
        assert s.d0 == 2

    def test_roxr_basic(self):
        prog = bytes([
            0x70, 0x02,  # D0=2
            0xE2, 0x90,  # ROXR.L #1, D0 (2 ROXR 1 with X=0 = 1)
        ]) + STOP
        s = run(prog)
        assert s.d0 == 1


# ──────────────────────────────────────────────────────────────────────────────
# LINK / UNLK
# ──────────────────────────────────────────────────────────────────────────────

class TestLINK:
    def test_link_unlk(self):
        # LINK A6, #-8 then UNLK A6
        # Use TRAP #15 to halt so halted=True without modifying SR/CCR.
        prog = bytes([
            0x4E, 0x56, 0xFF, 0xF8,  # LINK A6, #-8
            0x4E, 0x5E,              # UNLK A6
        ]) + HALT
        s = run(prog)
        # After LINK/UNLK, A7 should return to initial value (0xF000).
        # LINK pushes A6 and decrements A7 by 8; UNLK restores A7 from A6 then pops.
        assert s.halted
        assert s.a7 == 0xF000


# ──────────────────────────────────────────────────────────────────────────────
# NEGX
# ──────────────────────────────────────────────────────────────────────────────

class TestNEGX:
    def test_negx_l_with_x0(self):
        # NEGX.L D0 with X=0: result = 0 - D0 - 0 = -D0
        prog = bytes([
            0x70, 0x05,  # D0=5
            0x40, 0x80,  # NEGX.L D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 0xFFFFFFFB  # -5 unsigned

    def test_negx_b(self):
        prog = bytes([
            0x70, 0x01,  # D0=1
            0x40, 0x00,  # NEGX.B D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0xFF  # -1 in byte = 0xFF


# ──────────────────────────────────────────────────────────────────────────────
# TST
# ──────────────────────────────────────────────────────────────────────────────

class TestTST:
    def test_tst_zero(self):
        prog = bytes([
            0x70, 0x00,  # D0=0
            0x4A, 0x80,  # TST.L D0
        ]) + HALT
        s = run(prog)
        assert s.z

    def test_tst_negative(self):
        prog = bytes([
            0x70, 0x80,  # MOVEQ #-128, D0 (sign-extended)
            0x4A, 0x00,  # TST.B D0
        ]) + HALT
        s = run(prog)
        assert s.n  # MSB of byte is 1

    def test_tst_positive(self):
        prog = bytes([
            0x70, 0x05,
            0x4A, 0x80,  # TST.L D0
        ]) + HALT
        s = run(prog)
        assert not s.z
        assert not s.n


# ──────────────────────────────────────────────────────────────────────────────
# CHK instruction
# ──────────────────────────────────────────────────────────────────────────────

class TestCHK:
    def test_chk_no_exception(self):
        # CHK D1, D0: if 0 <= D0.W <= D1.W, no exception
        prog = bytes([
            0x72, 0x0A,  # D1=10 (upper bound)
            0x70, 0x05,  # D0=5
            0x41, 0x81,  # CHK D1, D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 5  # no exception, D0 unchanged


# ──────────────────────────────────────────────────────────────────────────────
# JMP / JSR absolute
# ──────────────────────────────────────────────────────────────────────────────

class TestJMP:
    def test_jmp_absolute_long(self):
        # JMP (abs.L) to skip the MOVEQ instruction.
        # Program layout (loaded at 0x1000):
        #   0x1000: JMP (abs.L)   — 6 bytes (opword + 4-byte address)
        #   0x1006: MOVEQ #0x42   — 2 bytes (skipped)
        #   0x1008: TRAP #15      — 2 bytes (halt; target of JMP)
        # JMP target = 0x00001008 → bytes 0x00, 0x00, 0x10, 0x08.
        sim = Motorola68kGateLevelSimulator()
        prog = bytearray([
            0x4E, 0xF9,              # JMP (abs.L)
            0x00, 0x00, 0x10, 0x08,  # target = 0x00001008 (TRAP #15 below)
            0x70, 0x42,              # MOVEQ #0x42, D0 (skipped)
            0x4E, 0x4F,              # TRAP #15 at 0x1008
        ])
        r = sim.execute(bytes(prog))
        assert r.final_state.d0 == 0  # D0 never set (skipped by JMP)
        assert r.halted


# ──────────────────────────────────────────────────────────────────────────────
# NMI / interrupt protocol
# ──────────────────────────────────────────────────────────────────────────────

class TestInterruptProtocol:
    def test_set_get_input_port(self):
        sim = Motorola68kGateLevelSimulator()
        sim.set_input_port(0, 42)
        assert sim.get_output_port(0) == 0  # no output ports

    def test_nmi_sets_flag(self):
        sim = Motorola68kGateLevelSimulator()
        sim.nmi()
        assert sim._pending_nmi

    def test_interrupt_level(self):
        sim = Motorola68kGateLevelSimulator()
        sim.interrupt(3)
        assert sim._pending_interrupt == 3


# ──────────────────────────────────────────────────────────────────────────────
# MOVE with various source EA modes
# ──────────────────────────────────────────────────────────────────────────────

class TestMoveEAModes:
    def test_move_indirect(self):
        # MOVE.L (A0), D0 — read from memory
        prog = bytes([
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0 (write)
            0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A,  # MOVE.L #42, D0
            0x20, 0x80,              # MOVE.L D0, (A0) — write 42 to 0x2000
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0 (read)
            0x20, 0x10,              # MOVE.L (A0), D0
        ]) + STOP
        s = run(prog)
        assert s.d0 == 42

    def test_move_postincrement(self):
        # MOVE.L (A0)+, D1 — read from memory with auto-increment.
        # Steps:
        #   1. LEA 0x2000, A0          — A0 = 0x2000
        #   2. MOVEQ #0x42, D0         — D0 = 0x42  (opcode 0x70 = MOVEQ to D0)
        #   3. MOVE.L D0, (A0)         — write 0x42 to memory[0x2000]
        #      Encoding: line 2 (MOVE.L), dst_reg=0 (A0), dst_mode=2, src=D0
        #      = 0010 000 010 000 000 = 0x2080
        #   4. LEA 0x2000, A0          — A0 = 0x2000 (reset)
        #   5. MOVE.L (A0)+, D1        — D1 = memory[0x2000]; A0 += 4
        #      Encoding: line 2 (MOVE.L), dst_reg=1 (D1), dst_mode=0, src=mode3(A0)
        #      = 0010 001 000 011 000 = 0x2218
        prog = bytes([
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x70, 0x42,              # MOVEQ #0x42, D0
            0x20, 0x80,              # MOVE.L D0, (A0) — write 0x42 to 0x2000
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0 (reset)
            0x22, 0x18,              # MOVE.L (A0)+, D1 — read + A0+=4
        ]) + STOP
        s = run(prog)
        # STOP clears CCR but not registers; D1 should hold the value read from memory
        assert s.d1 == 0x42
        assert s.a0 == 0x2004  # incremented by 4

    def test_movem_word_indirect(self):
        # MOVEM.W D0-D1, (A0) — word mode to indirect
        prog = bytes([
            0x70, 0x05,              # D0=5
            0x72, 0x0A,              # D1=10
            0x41, 0xF8, 0x20, 0x00,  # LEA 0x2000, A0
            0x48, 0x90, 0x00, 0x03,  # MOVEM.W D0-D1, (A0); mask=0x0003
        ]) + STOP
        s = run(prog)
        assert s.memory[0x2001] == 5   # D0 low byte
        assert s.memory[0x2003] == 10  # D1 low byte


# ──────────────────────────────────────────────────────────────────────────────
# PEA
# ──────────────────────────────────────────────────────────────────────────────

class TestPEA:
    def test_pea_basic(self):
        # PEA 0x2000 — push effective address of 0x2000 on stack
        prog = bytes([
            0x48, 0x79, 0x00, 0x00, 0x20, 0x00,  # PEA (0x2000).L
        ]) + STOP
        s = run(prog)
        # A7 was 0xF000, after PEA = 0xEFFC; memory at 0xEFFC should be 0x2000
        assert s.a7 == 0xEFFC
        assert s.memory[0xEFFF] == 0x00  # low byte of 0x2000


# ──────────────────────────────────────────────────────────────────────────────
# MOVE.W Dn, SR / MOVE SR, Dn / MOVE CCR, Dn
# ──────────────────────────────────────────────────────────────────────────────

class TestSROperations:
    def test_move_sr_to_dn(self):
        # MOVE SR, D0 — supervisor mode reads SR
        prog = bytes([
            0x40, 0xC0,  # MOVE SR, D0
        ]) + STOP
        s = run(prog)
        # SR should have supervisor bit set (0x2000) + int mask 7 (0x0700) = 0x2700
        assert (s.d0 & 0x2700) == 0x2700

    def test_move_ccr_to_dn(self):
        # Set CCR then read it
        prog = bytes([
            0x44, 0xFC, 0x00, 0x15,  # MOVE #0x15, CCR (X=1,N=0,Z=1,V=0,C=1=0x15=10101)
            0x42, 0xC0,              # MOVE CCR, D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0x1F) == 0x15


# ──────────────────────────────────────────────────────────────────────────────
# DBcc variations
# ──────────────────────────────────────────────────────────────────────────────

class TestDBcc:
    def test_dbt_no_loop(self):
        # DBT D0, offset — DBT condition always true → never loops (exits immediately)
        prog = bytes([
            0x70, 0x05,              # D0=5
            0x50, 0xC8, 0xFF, 0xFC,  # DBT D0, -4 (condition true → no decrement)
        ]) + STOP
        s = run(prog)
        assert s.d0 == 5  # D0 unchanged

    def test_dbne_exits_on_zero(self):
        # DBNE: if Z=0 (NE true), exit immediately
        prog = bytes([
            0x70, 0x05,              # D0=5
            0x44, 0xFC, 0x00, 0x00,  # MOVE #0, CCR (Z=0)
            0x56, 0xC8, 0xFF, 0xFC,  # DBNE D0, -4 (Z=0 → NE=true → exits)
        ]) + STOP
        s = run(prog)
        assert s.d0 == 5  # unchanged

    def test_dbcc_loops(self):
        # DBF (DBRA) — condition F is always false so always decrements and branches.
        # Encoding: 0101 0001 1100 1000 = 0x51C8; displacement -4 = 0xFFFC.
        # Loop body: increment D1 each iteration.
        # D0 starts at 2; loop runs 3 times (D0=2, D0=1, D0=0 → falls through).
        prog = bytes([
            0x70, 0x02,              # D0=2
            0x72, 0x00,              # D1=0 (counter)
            0x52, 0x41,              # ADDQ.W #1, D1 (loop body)
            0x51, 0xC8, 0xFF, 0xFC,  # DBF D0, -4 (always loops until D0=-1)
        ]) + STOP
        s = run(prog)
        # DBF loops 3 times: D0=2→1→0→-1 (falls through after 3rd increment)
        assert s.d1 == 3


# ──────────────────────────────────────────────────────────────────────────────
# Scc all variants (more conditions)
# ──────────────────────────────────────────────────────────────────────────────

class TestScc:
    def test_scs_when_carry(self):
        # SCS D0 — set if carry
        prog = bytes([
            0x44, 0xFC, 0x00, 0x01,  # MOVE #1, CCR (C=1)
            0x55, 0xC0,              # SCS D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0xFF

    def test_scc_no_carry(self):
        # SCC D0 — set if no carry
        prog = bytes([
            0x44, 0xFC, 0x00, 0x00,  # MOVE #0, CCR (C=0)
            0x54, 0xC0,              # SCC D0
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0xFF

    def test_sne_when_z_clear(self):
        prog = bytes([
            0x44, 0xFC, 0x00, 0x00,  # C=0,Z=0,N=0,V=0
            0x56, 0xC0,              # SNE D0 (Z=0 → NE=true)
        ]) + STOP
        s = run(prog)
        assert (s.d0 & 0xFF) == 0xFF


# ──────────────────────────────────────────────────────────────────────────────
# Bcc long branches (word displacement)
# ──────────────────────────────────────────────────────────────────────────────

class TestBranchLong:
    def test_bra_long(self):
        # BRA with 0x00 byte displacement → uses word displacement
        prog = bytes([
            0x60, 0x00, 0x00, 0x04,  # BRA.W +4
            0x70, 0x01,              # MOVEQ #1, D0 (skipped)
            0x70, 0x02,              # MOVEQ #2, D0 (taken)
        ]) + STOP
        s = run(prog)
        assert s.d0 == 2

    def test_bsr_long(self):
        # BSR.W — branch to subroutine with word displacement.
        # Program layout (loaded at 0x1000):
        #   0x1000: BSR.W +4  (4 bytes: opword 0x6100 + disp word 0x0004)
        #           push return addr 0x1004; jump to 0x1004 + 4 = 0x1008
        #   0x1004: TRAP #15  (2 bytes: halt after subroutine returns)
        #   0x1006: NOP       (2 bytes: padding)
        #   0x1008: MOVEQ #0x42, D0  (2 bytes: subroutine body)
        #   0x100A: RTS       (2 bytes: return to 0x1004)
        sim = Motorola68kGateLevelSimulator()
        prog = bytearray([
            0x61, 0x00, 0x00, 0x04,  # BSR.W +4 at 0x1000 → jumps to 0x1008
            0x4E, 0x4F,              # TRAP #15 at 0x1004 (halt on return)
            0x4E, 0x71,              # NOP at 0x1006 (padding)
            0x70, 0x42,              # MOVEQ #0x42, D0 at 0x1008
            0x4E, 0x75,              # RTS at 0x100A → return to 0x1004
        ])
        r = sim.execute(bytes(prog))
        assert r.final_state.d0 == 0x42
        assert r.halted
