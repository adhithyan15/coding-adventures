"""Tests for Z80 CB-prefix bit manipulation instructions.

Covers: BIT b, r  — test a single bit, set Z flag;
        SET b, r  — set a single bit;
        RES b, r  — reset (clear) a single bit;
        DDCB/FDCB — same operations on (IX+d)/(IY+d).
"""

from z80_simulator import Z80Simulator

# ── BIT b, r ─────────────────────────────────────────────────────────────────

class TestBIT:
    def test_bit_set_clears_z(self):
        # BIT 0, A where A=0x01: bit 0 is set → Z=0
        # CB 0x47: BIT 0, A
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0xCB, 0x47, 0x76]))
        assert r.final_state.flag_z is False

    def test_bit_clear_sets_z(self):
        # BIT 1, A where A=0x01: bit 1 is 0 → Z=1
        # CB 0x4F: BIT 1, A
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x01, 0xCB, 0x4F, 0x76]))
        assert r.final_state.flag_z is True

    def test_bit_7(self):
        # BIT 7, A where A=0x80: bit 7 set → Z=0
        # CB 0x7F: BIT 7, A
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x80, 0xCB, 0x7F, 0x76]))
        assert r.final_state.flag_z is False

    def test_bit_sets_h_clears_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xFF, 0xCB, 0x47, 0x76]))
        assert r.final_state.flag_h is True
        assert r.final_state.flag_n is False

    def test_bit_register_unchanged(self):
        # BIT should not modify the register
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xAB, 0xCB, 0x47, 0x76]))
        assert r.final_state.a == 0xAB

    def test_bit_on_b(self):
        # BIT 3, B where B=0x08: CB 0x58
        sim = Z80Simulator()
        r = sim.execute(bytes([0x06, 0x08, 0xCB, 0x58, 0x76]))
        assert r.final_state.flag_z is False

    def test_bit_on_hl_indirect(self):
        # BIT 0, (HL): CB 0x46; data must be at the address HL points to.
        # Use absolute address 0x1000 to avoid overlap with code.
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x01,         # LD A, 0x01 (value with bit 0 set)
            0x32, 0x00, 0x10,   # LD (0x1000), A
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0xCB, 0x46,         # BIT 0, (HL)
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_z is False


# ── SET b, r ──────────────────────────────────────────────────────────────────

class TestSET:
    def test_set_bit_0_a(self):
        # SET 0, A: CB 0xC7
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x00, 0xCB, 0xC7, 0x76]))
        assert r.final_state.a == 0x01

    def test_set_bit_7_a(self):
        # SET 7, A: CB 0xFF
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x00, 0xCB, 0xFF, 0x76]))
        assert r.final_state.a == 0x80

    def test_set_already_set(self):
        # Setting an already-set bit leaves it set
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xFF, 0xCB, 0xC7, 0x76]))
        assert r.final_state.a == 0xFF

    def test_set_bit_3_b(self):
        # SET 3, B: CB 0xD8
        sim = Z80Simulator()
        r = sim.execute(bytes([0x06, 0x00, 0xCB, 0xD8, 0x76]))
        assert r.final_state.b == 0x08

    def test_set_hl_indirect(self):
        # SET 0, (HL): CB 0xC6 — use 0x1000 to avoid overlap with code
        sim = Z80Simulator()
        prog = bytes([
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0xCB, 0xC6,         # SET 0, (HL)
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x1000] == 0x01


# ── RES b, r ──────────────────────────────────────────────────────────────────

class TestRES:
    def test_res_bit_0_a(self):
        # RES 0, A: CB 0x87
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xFF, 0xCB, 0x87, 0x76]))
        assert r.final_state.a == 0xFE

    def test_res_bit_7_a(self):
        # RES 7, A: CB 0xBF
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0xFF, 0xCB, 0xBF, 0x76]))
        assert r.final_state.a == 0x7F

    def test_res_already_clear(self):
        # Clearing an already-clear bit leaves it clear
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x00, 0xCB, 0x87, 0x76]))
        assert r.final_state.a == 0x00

    def test_res_bit_3_b(self):
        # RES 3, B: CB 0x98
        sim = Z80Simulator()
        r = sim.execute(bytes([0x06, 0xFF, 0xCB, 0x98, 0x76]))
        assert r.final_state.b == 0xF7   # 0xFF & ~0x08

    def test_res_hl_indirect(self):
        # RES 0, (HL): CB 0x86 — use 0x1000 to avoid overlap with code
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xFF,         # LD A, 0xFF
            0x32, 0x00, 0x10,   # LD (0x1000), A
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0xCB, 0x86,         # RES 0, (HL)
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x1000] == 0xFE


# ── BIT/SET/RES via (IX+d) — DDCB prefix ────────────────────────────────────

class TestDDCBBitOps:
    def test_bit_ix_d(self):
        # DDCB prefix: BIT 0, (IX+1)
        # DD CB 01 46: BIT 0, (IX+1)
        # Use a full program that writes 0x01 to memory[0x1001] first,
        # then sets IX=0x1000 and does BIT 0, (IX+1).
        sim2 = Z80Simulator()
        full_prog = bytes([
            0x3E, 0x01,             # LD A, 0x01
            0x32, 0x01, 0x10,       # LD (0x1001), A
            0xDD, 0x21, 0x00, 0x10, # LD IX, 0x1000
            0xDD, 0xCB, 0x01, 0x46, # BIT 0, (IX+1)
            0x76,                    # HALT
        ])
        r = sim2.execute(full_prog)
        assert r.final_state.flag_z is False   # bit 0 is set

    def test_set_ix_d(self):
        # DDCB prefix: SET 0, (IX+0)
        # DD CB 00 C6: SET 0, (IX+0)
        sim = Z80Simulator()
        prog = bytes([
            0xDD, 0x21, 0x00, 0x10, # LD IX, 0x1000
            0xDD, 0xCB, 0x00, 0xC6, # SET 0, (IX+0)
            0x76,                    # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x1000] == 0x01

    def test_res_ix_d(self):
        # DDCB prefix: RES 0, (IX+0)
        # DD CB 00 86: RES 0, (IX+0)
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xFF,             # LD A, 0xFF
            0x32, 0x00, 0x10,       # LD (0x1000), A
            0xDD, 0x21, 0x00, 0x10, # LD IX, 0x1000
            0xDD, 0xCB, 0x00, 0x86, # RES 0, (IX+0)
            0x76,                    # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x1000] == 0xFE
