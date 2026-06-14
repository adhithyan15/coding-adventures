"""Tests for Z80 logical instructions.

Covers: AND, OR, XOR, CP (all affect flags);
        CPL, CCF, SCF (accumulator/carry operations).
"""

from z80_simulator import Z80Simulator

# ── AND n ─────────────────────────────────────────────────────────────────────

class TestAnd:
    def test_and_basic(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xF0, 0xE6, 0x0F, 0x76]))  # A=0xF0; AND 0x0F
        assert r.final_state.a == 0x00
        assert r.final_state.flag_z is True

    def test_and_keeps_bits(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xFF, 0xE6, 0xAA, 0x76]))  # A=0xFF; AND 0xAA
        assert r.final_state.a == 0xAA

    def test_and_sets_h_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xFF, 0xE6, 0xFF, 0x76]))
        assert r.final_state.flag_h is True

    def test_and_clears_n_and_c(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0xFF, 0xE6, 0xFF, 0x76]))  # SCF then AND
        assert r.final_state.flag_n is False
        assert r.final_state.flag_c is False

    def test_and_sets_parity(self):
        # 0xFF & 0xFF = 0xFF (8 bits set → even parity → PV=1)
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xFF, 0xE6, 0xFF, 0x76]))
        assert r.final_state.flag_pv is True

    def test_and_sign_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x80, 0xE6, 0x80, 0x76]))
        assert r.final_state.flag_s is True


# ── OR n ──────────────────────────────────────────────────────────────────────

class TestOr:
    def test_or_basic(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xF0, 0xF6, 0x0F, 0x76]))  # A=0xF0; OR 0x0F
        assert r.final_state.a == 0xFF

    def test_or_zero(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x00, 0xF6, 0x00, 0x76]))
        assert r.final_state.a == 0
        assert r.final_state.flag_z is True

    def test_or_clears_h_n_c(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0x01, 0xF6, 0x02, 0x76]))  # SCF first
        assert r.final_state.flag_h is False
        assert r.final_state.flag_n is False
        assert r.final_state.flag_c is False

    def test_or_parity(self):
        # OR A with itself: 0b10101010 = 4 bits → even parity
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xAA, 0xF6, 0x00, 0x76]))
        assert r.final_state.flag_pv is True   # 4 set bits = even


# ── XOR n ─────────────────────────────────────────────────────────────────────

class TestXor:
    def test_xor_self_clears(self):
        # XOR A: A^A = 0, Z=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x42, 0xAF, 0x76]))  # LD A,0x42; XOR A
        assert r.final_state.a == 0
        assert r.final_state.flag_z is True

    def test_xor_toggles_bits(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xF0, 0xEE, 0xFF, 0x76]))  # A=0xF0; XOR 0xFF
        assert r.final_state.a == 0x0F

    def test_xor_clears_h_n_c(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0xAF, 0x76]))  # SCF; XOR A
        assert r.final_state.flag_h is False
        assert r.final_state.flag_n is False
        assert r.final_state.flag_c is False

    def test_xor_parity(self):
        # 0xF0 XOR 0x00 = 0xF0 (4 set bits → even parity)
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xF0, 0xEE, 0x00, 0x76]))
        assert r.final_state.flag_pv is True


# ── CP n ──────────────────────────────────────────────────────────────────────

class TestCp:
    def test_cp_equal(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x0A, 0xFE, 0x0A, 0x76]))  # CP same value
        assert r.final_state.flag_z is True
        assert r.final_state.a == 0x0A   # A unchanged!

    def test_cp_less_than(self):
        # A < operand: borrow → C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x05, 0xFE, 0x0A, 0x76]))
        assert r.final_state.flag_c is True
        assert r.final_state.a == 0x05   # A unchanged

    def test_cp_greater_than(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x0A, 0xFE, 0x05, 0x76]))
        assert r.final_state.flag_z is False
        assert r.final_state.flag_c is False
        assert r.final_state.a == 0x0A

    def test_cp_sets_n_flag(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x05, 0xFE, 0x03, 0x76]))
        assert r.final_state.flag_n is True


# ── CPL ───────────────────────────────────────────────────────────────────────

class TestCpl:
    def test_cpl_inverts_a(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xF0, 0x2F, 0x76]))  # A=0xF0; CPL
        assert r.final_state.a == 0x0F

    def test_cpl_all_zeros(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x00, 0x2F, 0x76]))
        assert r.final_state.a == 0xFF

    def test_cpl_sets_h_and_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x55, 0x2F, 0x76]))
        assert r.final_state.flag_h is True
        assert r.final_state.flag_n is True

    def test_cpl_does_not_change_c(self):
        # After SCF (C=1), CPL should not clear C
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0x55, 0x2F, 0x76]))
        assert r.final_state.flag_c is True


# ── CCF / SCF ─────────────────────────────────────────────────────────────────

class TestCcfScf:
    def test_scf_sets_carry(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0xAF, 0x37, 0x76]))   # XOR A (C=0); SCF
        assert r.final_state.flag_c is True

    def test_scf_clears_h_and_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x76]))
        assert r.final_state.flag_h is False
        assert r.final_state.flag_n is False

    def test_ccf_toggles_carry_off(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3F, 0x76]))   # SCF; CCF → C=0
        assert r.final_state.flag_c is False

    def test_ccf_toggles_carry_on(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0xAF, 0x3F, 0x76]))   # XOR A (C=0); CCF → C=1
        assert r.final_state.flag_c is True

    def test_ccf_sets_h_to_prev_c(self):
        # Before CCF, C=1 → H becomes 1 (H = old C)
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3F, 0x76]))   # SCF (C=1); CCF
        assert r.final_state.flag_h is True

    def test_ccf_clears_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3F, 0x76]))
        assert r.final_state.flag_n is False
