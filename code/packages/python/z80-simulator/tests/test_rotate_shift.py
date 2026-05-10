"""Tests for Z80 rotate and shift instructions.

Covers: RLCA, RRCA, RLA, RRA (accumulator rotates, no SZP flags);
        CB-prefix: RLC, RRC, RL, RR, SLA, SRA, SLL, SRL on registers;
        RLD and RRD (ED-prefix digit rotates).
"""

from z80_simulator import Z80Simulator

# ── RLCA ─────────────────────────────────────────────────────────────────────

class TestRLCA:
    def test_rlca_bit7_to_carry_and_bit0(self):
        # 0b10000001 → RLCA → 0b00000011, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x81, 0x07, 0x76]))
        assert r.final_state.a == 0x03
        assert r.final_state.flag_c is True

    def test_rlca_no_carry(self):
        # 0b00000001 → RLCA → 0b00000010, C=0
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0x07, 0x76]))
        assert r.final_state.a == 0x02
        assert r.final_state.flag_c is False

    def test_rlca_clears_h_and_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x80, 0x07, 0x76]))
        assert r.final_state.flag_h is False
        assert r.final_state.flag_n is False

    def test_rlca_rotation(self):
        # 0xFF rotated 8 times should return to 0xFF
        sim = Z80Simulator()
        prog = bytes([0x3E, 0xFF] + [0x07] * 8 + [0x76])
        r = sim.execute(prog)
        assert r.final_state.a == 0xFF


# ── RRCA ─────────────────────────────────────────────────────────────────────

class TestRRCA:
    def test_rrca_bit0_to_carry_and_bit7(self):
        # 0b10000001 → RRCA → 0b11000000, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x81, 0x0F, 0x76]))
        assert r.final_state.a == 0xC0
        assert r.final_state.flag_c is True

    def test_rrca_no_carry(self):
        # 0b00000010 → RRCA → 0b00000001, C=0
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x02, 0x0F, 0x76]))
        assert r.final_state.a == 0x01
        assert r.final_state.flag_c is False

    def test_rrca_clears_h_and_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0x0F, 0x76]))
        assert r.final_state.flag_h is False
        assert r.final_state.flag_n is False


# ── RLA ───────────────────────────────────────────────────────────────────────

class TestRLA:
    def test_rla_shifts_carry_in(self):
        # C=1; A=0b00000000 → RLA → A=0b00000001, C=0
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0x00, 0x17, 0x76]))
        assert r.final_state.a == 0x01
        assert r.final_state.flag_c is False

    def test_rla_bit7_becomes_carry(self):
        # A=0b10000000, C=0 → RLA → A=0b00000000, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0xAF, 0x3E, 0x80, 0x17, 0x76]))  # XOR A clears C
        assert r.final_state.a == 0x00
        assert r.final_state.flag_c is True

    def test_rla_clears_h_and_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0x17, 0x76]))
        assert r.final_state.flag_h is False
        assert r.final_state.flag_n is False


# ── RRA ───────────────────────────────────────────────────────────────────────

class TestRRA:
    def test_rra_shifts_carry_in(self):
        # C=1; A=0b00000000 → RRA → A=0b10000000, C=0
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0x00, 0x1F, 0x76]))
        assert r.final_state.a == 0x80
        assert r.final_state.flag_c is False

    def test_rra_bit0_becomes_carry(self):
        # A=0b00000001, C=0 → RRA → A=0b00000000, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0xAF, 0x3E, 0x01, 0x1F, 0x76]))
        assert r.final_state.a == 0x00
        assert r.final_state.flag_c is True


# ── CB RLC r ──────────────────────────────────────────────────────────────────

class TestCbRLC:
    def test_rlc_a(self):
        # CB 0x07: RLC A; 0b10000001 → 0b00000011, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x81, 0xCB, 0x07, 0x76]))
        assert r.final_state.a == 0x03
        assert r.final_state.flag_c is True

    def test_rlc_b(self):
        # CB 0x00: RLC B
        sim = Z80Simulator()
        r = sim.execute(bytes([0x06, 0x40, 0xCB, 0x00, 0x76]))  # B=0x40
        assert r.final_state.b == 0x80
        assert r.final_state.flag_c is False

    def test_rlc_sets_sz_flags(self):
        # RLC 0x80 → 0x01 (bit 7 set → bit 0); S=0, Z=0
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x80, 0xCB, 0x07, 0x76]))
        assert r.final_state.flag_s is False
        assert r.final_state.flag_z is False

    def test_rlc_zero_result(self):
        # RLC 0x00 → 0x00; Z=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x00, 0xCB, 0x07, 0x76]))
        assert r.final_state.flag_z is True


# ── CB RRC r ──────────────────────────────────────────────────────────────────

class TestCbRRC:
    def test_rrc_a(self):
        # CB 0x0F: RRC A; 0b00000001 → 0b10000000, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0xCB, 0x0F, 0x76]))
        assert r.final_state.a == 0x80
        assert r.final_state.flag_c is True
        assert r.final_state.flag_s is True


# ── CB RL r ───────────────────────────────────────────────────────────────────

class TestCbRL:
    def test_rl_a_with_carry(self):
        # SCF; CB 0x17: RL A; A=0x00, C=1 → A=0x01, C=0
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0x00, 0xCB, 0x17, 0x76]))
        assert r.final_state.a == 0x01
        assert r.final_state.flag_c is False

    def test_rl_a_no_carry(self):
        # C=0; RL A=0x80 → A=0x00, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0xAF, 0x3E, 0x80, 0xCB, 0x17, 0x76]))
        assert r.final_state.a == 0x00
        assert r.final_state.flag_c is True


# ── CB RR r ───────────────────────────────────────────────────────────────────

class TestCbRR:
    def test_rr_a(self):
        # C=1; RR A=0x00 → A=0x80, C=0
        sim = Z80Simulator()
        r = sim.execute(bytes([0x37, 0x3E, 0x00, 0xCB, 0x1F, 0x76]))
        assert r.final_state.a == 0x80
        assert r.final_state.flag_c is False


# ── CB SLA / SRA / SRL ────────────────────────────────────────────────────────

class TestCbSLA:
    def test_sla_a(self):
        # CB 0x27: SLA A; 0b10000001 → 0b00000010, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x81, 0xCB, 0x27, 0x76]))
        assert r.final_state.a == 0x02
        assert r.final_state.flag_c is True

    def test_sla_shifts_zero_in(self):
        # SLA: bit 0 gets 0 (logical shift left)
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0xCB, 0x27, 0x76]))
        assert r.final_state.a == 0x02   # not 0x03


class TestCbSRA:
    def test_sra_preserves_sign(self):
        # CB 0x2F: SRA A; 0b10000010 → 0b11000001 (sign extended)
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x82, 0xCB, 0x2F, 0x76]))
        assert r.final_state.a == 0xC1
        assert r.final_state.flag_s is True

    def test_sra_positive(self):
        # SRA 0x10 → 0x08
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x10, 0xCB, 0x2F, 0x76]))
        assert r.final_state.a == 0x08


class TestCbSRL:
    def test_srl_shifts_zero_in(self):
        # CB 0x3F: SRL A; 0b10000000 → 0b01000000 (bit 7 gets 0)
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x80, 0xCB, 0x3F, 0x76]))
        assert r.final_state.a == 0x40
        assert r.final_state.flag_s is False

    def test_srl_sets_carry(self):
        # SRL 0x01 → 0x00, C=1
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0xCB, 0x3F, 0x76]))
        assert r.final_state.a == 0x00
        assert r.final_state.flag_c is True
        assert r.final_state.flag_z is True


# ── RLD / RRD (ED prefix) ────────────────────────────────────────────────────

class TestRLD:
    def test_rld_basic(self):
        # RLD: A_lo→(HL)_hi, (HL)_hi→A_lo, (HL)_lo unchanged... actually:
        # RLD: new_m = ((m<<4) | A_lo) & 0xFF; A = (A & 0xF0) | (m >> 4)
        # A=0x12, (HL)=0x34 → A = (0x10 | 0x03) = 0x13; (HL) = (0x40 | 0x02) = 0x42
        # Use 0x1000 to avoid RLD overwriting HALT opcode in the program.
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x34,             # LD A, 0x34
            0x32, 0x00, 0x10,       # LD (0x1000), A   (HL data = 0x34)
            0x3E, 0x12,             # LD A, 0x12
            0x21, 0x00, 0x10,       # LD HL, 0x1000
            0xED, 0x6F,             # RLD
            0x76,                   # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x13
        assert r.final_state.memory[0x1000] == 0x42


class TestRRD:
    def test_rrd_basic(self):
        # RRD: new_m = ((A_lo<<4) | (m>>4)) & 0xFF; A = (A & 0xF0) | (m & 0x0F)
        # A=0x12, (HL)=0x34 → A = (0x10 | 0x04) = 0x14; (HL) = (0x20 | 0x03) = 0x23
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x34,             # LD A, 0x34
            0x32, 0x00, 0x10,       # LD (0x1000), A
            0x3E, 0x12,             # LD A, 0x12
            0x21, 0x00, 0x10,       # LD HL, 0x1000
            0xED, 0x67,             # RRD
            0x76,                   # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x14
        assert r.final_state.memory[0x1000] == 0x23
