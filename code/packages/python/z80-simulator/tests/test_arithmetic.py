"""Tests for Z80 arithmetic instructions.

Covers: ADD A,r/n; ADC A,r/n; SUB r/n; SBC A,r/n; INC r/rp; DEC r/rp;
        ADD HL,rp (ED) ADC HL,rp; SBC HL,rp; NEG; DAA.
"""

from z80_simulator import Z80Simulator

# ── ADD A, n (immediate) ──────────────────────────────────────────────────────

class TestAddImmediate:
    def test_basic_add(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x0A, 0xC6, 0x05, 0x76]))  # LD A,10; ADD A,5
        assert r.final_state.a == 15

    def test_add_zero_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x00, 0xC6, 0x00, 0x76]))  # 0+0=0 → Z=1
        assert r.final_state.flag_z is True
        assert r.final_state.a == 0

    def test_add_carry_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xFF, 0xC6, 0x01, 0x76]))  # 0xFF+1 → carry
        assert r.final_state.flag_c is True
        assert r.final_state.a == 0x00
        assert r.final_state.flag_z is True

    def test_add_half_carry_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x0F, 0xC6, 0x01, 0x76]))  # 0x0F+1 → H=1
        assert r.final_state.flag_h is True

    def test_add_overflow(self):
        # 0x7F + 1 = 0x80: signed overflow (127 → -128)
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x7F, 0xC6, 0x01, 0x76]))
        assert r.final_state.flag_pv is True
        assert r.final_state.a == 0x80
        assert r.final_state.flag_s is True

    def test_add_clears_n_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x05, 0xC6, 0x03, 0x76]))
        assert r.final_state.flag_n is False


# ── ADC A, n ──────────────────────────────────────────────────────────────────

class TestAdcImmediate:
    def test_adc_with_carry(self):
        # SCF; LD A,5; ADC A,3  → 5+3+1=9
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0x05, 0xCE, 0x03, 0x76]))
        assert r.final_state.a == 9

    def test_adc_without_carry(self):
        # No carry set: ADC same as ADD
        sim = Z80Simulator()
        r = sim.execute(bytes([0xAF, 0x3E, 0x05, 0xCE, 0x03, 0x76]))  # XOR A clears C
        assert r.final_state.a == 8

    def test_adc_carry_propagates(self):
        # 0xFF + 0x00 + C=1 = 0x00 with carry out
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0xFF, 0xCE, 0x00, 0x76]))
        assert r.final_state.a == 0
        assert r.final_state.flag_c is True
        assert r.final_state.flag_z is True


# ── SUB n ─────────────────────────────────────────────────────────────────────

class TestSubImmediate:
    def test_basic_sub(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x0A, 0xD6, 0x03, 0x76]))  # 10-3=7
        assert r.final_state.a == 7

    def test_sub_zero(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x05, 0xD6, 0x05, 0x76]))  # 5-5=0
        assert r.final_state.a == 0
        assert r.final_state.flag_z is True

    def test_sub_sets_n_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x05, 0xD6, 0x03, 0x76]))
        assert r.final_state.flag_n is True

    def test_sub_borrow(self):
        # 0 - 1 = 0xFF with borrow (carry)
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x00, 0xD6, 0x01, 0x76]))
        assert r.final_state.a == 0xFF
        assert r.final_state.flag_c is True

    def test_sub_overflow(self):
        # 0x80 - 1 = 0x7F: overflow (-128 - 1 → +127)
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x80, 0xD6, 0x01, 0x76]))
        assert r.final_state.flag_pv is True


# ── SBC A, n ──────────────────────────────────────────────────────────────────

class TestSbcImmediate:
    def test_sbc_with_borrow(self):
        # SCF; LD A,10; SBC A,3  → 10-3-1=6
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0x0A, 0xDE, 0x03, 0x76]))
        assert r.final_state.a == 6

    def test_sbc_no_borrow(self):
        # C=0: SBC A,3 → 10-3=7
        sim = Z80Simulator()
        r = sim.execute(bytes([0xAF, 0x3E, 0x0A, 0xDE, 0x03, 0x76]))
        assert r.final_state.a == 7


# ── INC r / DEC r ─────────────────────────────────────────────────────────────

class TestIncDecR:
    def test_inc_a(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x41, 0x3C, 0x76]))   # LD A,0x41; INC A
        assert r.final_state.a == 0x42

    def test_inc_overflow(self):
        # INC 0x7F → 0x80: overflow flag set
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x7F, 0x3C, 0x76]))
        assert r.final_state.flag_pv is True
        assert r.final_state.a == 0x80

    def test_inc_wrap(self):
        # INC 0xFF → 0x00, Z=1, but C unchanged
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0xFF, 0x3C, 0x76]))  # SCF; INC A
        assert r.final_state.a == 0x00
        assert r.final_state.flag_z is True
        assert r.final_state.flag_c is True   # carry NOT affected by INC

    def test_dec_a(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x0A, 0x3D, 0x76]))   # LD A,10; DEC A
        assert r.final_state.a == 9

    def test_dec_sets_n_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x05, 0x3D, 0x76]))
        assert r.final_state.flag_n is True

    def test_dec_overflow(self):
        # DEC 0x80 → 0x7F: overflow
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x80, 0x3D, 0x76]))
        assert r.final_state.flag_pv is True
        assert r.final_state.a == 0x7F

    def test_dec_zero_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0x3D, 0x76]))   # 1→0
        assert r.final_state.flag_z is True


# ── INC rp / DEC rp ──────────────────────────────────────────────────────────

class TestIncDecRP:
    def test_inc_bc(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x01, 0xFF, 0x00, 0x03, 0x76]))  # LD BC,0xFF; INC BC
        assert r.final_state.bc == 0x0100

    def test_dec_hl(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x21, 0x00, 0x01, 0x2B, 0x76]))  # LD HL,0x100; DEC HL
        assert r.final_state.hl == 0x00FF

    def test_inc_sp(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x31, 0xFE, 0x7F, 0x33, 0x76]))  # LD SP,0x7FFE; INC SP
        assert r.final_state.sp == 0x7FFF


# ── ADD HL, rp ────────────────────────────────────────────────────────────────

class TestAddHlRp:
    def test_add_hl_bc(self):
        sim = Z80Simulator()
        prog = bytes([
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0x01, 0x00, 0x01,   # LD BC, 0x0100
            0x09,               # ADD HL, BC
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.hl == 0x1100

    def test_add_hl_hl(self):
        # LD HL, 0x1234; ADD HL, HL → 0x2468
        sim = Z80Simulator()
        r = sim.execute(bytes([0x21, 0x34, 0x12, 0x29, 0x76]))
        assert r.final_state.hl == 0x2468

    def test_add_hl_carry(self):
        sim = Z80Simulator()
        prog = bytes([
            0x21, 0xFF, 0xFF,   # LD HL, 0xFFFF
            0x01, 0x01, 0x00,   # LD BC, 0x0001
            0x09,               # ADD HL, BC → wrap
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.hl == 0x0000
        assert r.final_state.flag_c is True

    def test_add_hl_clears_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x21, 0x01, 0x00, 0x29, 0x76]))
        assert r.final_state.flag_n is False


# ── ADC HL, rp (ED prefix) ───────────────────────────────────────────────────

class TestAdcHlRp:
    def test_adc_hl_bc_no_carry(self):
        sim = Z80Simulator()
        prog = bytes([
            0xAF,               # XOR A (clears C)
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0x01, 0x00, 0x01,   # LD BC, 0x0100
            0xED, 0x4A,         # ADC HL, BC
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.hl == 0x1100

    def test_adc_hl_with_carry(self):
        sim = Z80Simulator()
        prog = bytes([
            0x37,               # SCF (C=1)
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0x01, 0x00, 0x01,   # LD BC, 0x0100
            0xED, 0x4A,         # ADC HL, BC
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.hl == 0x1101


# ── SBC HL, rp (ED prefix) ───────────────────────────────────────────────────

class TestSbcHlRp:
    def test_sbc_hl_bc(self):
        sim = Z80Simulator()
        prog = bytes([
            0xAF,               # XOR A (clears C)
            0x21, 0x00, 0x20,   # LD HL, 0x2000
            0x01, 0x00, 0x10,   # LD BC, 0x1000
            0xED, 0x42,         # SBC HL, BC
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.hl == 0x1000

    def test_sbc_hl_self_is_zero(self):
        sim = Z80Simulator()
        prog = bytes([
            0xAF,               # XOR A (clears C)
            0x21, 0x56, 0x12,   # LD HL, 0x1256
            0x21, 0x56, 0x12,   # LD HL, 0x1256 (same)
            0xED, 0x62,         # SBC HL, HL
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.hl == 0
        assert r.final_state.flag_z is True


# ── NEG (ED prefix) ───────────────────────────────────────────────────────────

class TestNEG:
    def test_neg_basic(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x05, 0xED, 0x44, 0x76]))  # LD A,5; NEG
        assert r.final_state.a == 0xFB   # -5 two's complement

    def test_neg_zero_stays_zero(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x00, 0xED, 0x44, 0x76]))
        assert r.final_state.a == 0x00
        assert r.final_state.flag_c is False

    def test_neg_0x80_overflows(self):
        # -(-128) = +128 which doesn't fit in 8-bit signed: overflow
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x80, 0xED, 0x44, 0x76]))
        assert r.final_state.a == 0x80
        assert r.final_state.flag_pv is True

    def test_neg_sets_n_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0xED, 0x44, 0x76]))
        assert r.final_state.flag_n is True

    def test_neg_nonzero_sets_carry(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0xED, 0x44, 0x76]))
        assert r.final_state.flag_c is True


# ── DAA ───────────────────────────────────────────────────────────────────────

class TestDAA:
    def test_daa_after_bcd_add(self):
        # 0x09 (BCD 9) + 0x01 (BCD 1) = 0x0A → DAA → 0x10 (BCD 10)
        sim = Z80Simulator()
        r = sim.execute(bytes([
            0x3E, 0x09,   # LD A, 9
            0xC6, 0x01,   # ADD A, 1  → A = 0x0A
            0x27,         # DAA
            0x76,
        ]))
        assert r.final_state.a == 0x10

    def test_daa_after_bcd_add_both_nibbles(self):
        # BCD 99 + 1 = BCD 100 → A=0x00, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([
            0x3E, 0x99,   # LD A, 0x99 (BCD 99)
            0xC6, 0x01,   # ADD A, 1 → 0x9A
            0x27,         # DAA → 0x00 + C
            0x76,
        ]))
        assert r.final_state.a == 0x00
        assert r.final_state.flag_c is True
