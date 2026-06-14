"""Tests for Z80 IX and IY index register instructions (DD/FD prefix).

Covers: LD IX/IY,nn; LD IX/IY,(nn); LD (nn),IX/IY; LD SP,IX/IY;
        PUSH/POP IX/IY; ADD IX/IY,rp; INC/DEC IX/IY;
        LD r,(IX+d); LD (IX+d),r; LD (IX+d),n;
        INC/DEC (IX+d); ALU ops with (IX+d); JP (IX).
"""

from z80_simulator import Z80Simulator

# ── LD IX, nn ─────────────────────────────────────────────────────────────────

class TestLdIX:
    def test_ld_ix_immediate(self):
        # DD 21 lo hi: LD IX, nn
        sim = Z80Simulator()
        r = sim.execute(bytes([0xDD, 0x21, 0x34, 0x12, 0x76]))  # LD IX, 0x1234
        assert r.final_state.ix == 0x1234

    def test_ld_iy_immediate(self):
        # FD 21 lo hi: LD IY, nn
        sim = Z80Simulator()
        r = sim.execute(bytes([0xFD, 0x21, 0x78, 0x56, 0x76]))  # LD IY, 0x5678
        assert r.final_state.iy == 0x5678


# ── LD IX, (nn) / LD (nn), IX ────────────────────────────────────────────────

class TestLdIXIndirect:
    def test_ld_ix_nn_indirect(self):
        # Store 0x1234 at 0x2000, then LD IX, (0x2000)
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x34, 0x32, 0x00, 0x20,  # LD (0x2000), 0x34
            0x3E, 0x12, 0x32, 0x01, 0x20,  # LD (0x2001), 0x12
            0xDD, 0x2A, 0x00, 0x20,        # LD IX, (0x2000)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.ix == 0x1234

    def test_ld_nn_ix(self):
        # LD IX, 0xABCD; LD (0x3000), IX
        sim = Z80Simulator()
        prog = bytes([
            0xDD, 0x21, 0xCD, 0xAB,        # LD IX, 0xABCD
            0xDD, 0x22, 0x00, 0x30,        # LD (0x3000), IX
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x3000] == 0xCD
        assert r.final_state.memory[0x3001] == 0xAB


# ── LD SP, IX ─────────────────────────────────────────────────────────────────

class TestLdSpIX:
    def test_ld_sp_ix(self):
        # DD F9: LD SP, IX
        sim = Z80Simulator()
        r = sim.execute(bytes([0xDD, 0x21, 0x00, 0x80, 0xDD, 0xF9, 0x76]))
        assert r.final_state.sp == 0x8000

    def test_ld_sp_iy(self):
        # FD F9: LD SP, IY
        sim = Z80Simulator()
        r = sim.execute(bytes([0xFD, 0x21, 0xFF, 0x7F, 0xFD, 0xF9, 0x76]))
        assert r.final_state.sp == 0x7FFF


# ── PUSH / POP IX ─────────────────────────────────────────────────────────────

class TestPushPopIX:
    def test_push_pop_ix(self):
        sim = Z80Simulator()
        prog = bytes([
            0x31, 0x00, 0x80,        # LD SP, 0x8000
            0xDD, 0x21, 0x34, 0x12,  # LD IX, 0x1234
            0xDD, 0xE5,              # PUSH IX
            0xDD, 0x21, 0x00, 0x00,  # LD IX, 0 (clobber)
            0xDD, 0xE1,              # POP IX
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.ix == 0x1234

    def test_push_pop_iy(self):
        sim = Z80Simulator()
        prog = bytes([
            0x31, 0x00, 0x80,        # LD SP, 0x8000
            0xFD, 0x21, 0x78, 0x56,  # LD IY, 0x5678
            0xFD, 0xE5,              # PUSH IY
            0xFD, 0x21, 0x00, 0x00,  # LD IY, 0 (clobber)
            0xFD, 0xE1,              # POP IY
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.iy == 0x5678


# ── ADD IX, rp ────────────────────────────────────────────────────────────────

class TestAddIX:
    def test_add_ix_bc(self):
        # LD IX,0x1000; LD BC,0x0200; ADD IX,BC → IX=0x1200
        sim = Z80Simulator()
        prog = bytes([
            0xDD, 0x21, 0x00, 0x10,  # LD IX, 0x1000
            0x01, 0x00, 0x02,        # LD BC, 0x0200
            0xDD, 0x09,              # ADD IX, BC
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.ix == 0x1200

    def test_add_iy_de(self):
        # LD IY,0x0500; LD DE,0x0100; ADD IY,DE → IY=0x0600
        sim = Z80Simulator()
        prog = bytes([
            0xFD, 0x21, 0x00, 0x05,  # LD IY, 0x0500
            0x11, 0x00, 0x01,        # LD DE, 0x0100
            0xFD, 0x19,              # ADD IY, DE
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.iy == 0x0600

    def test_add_ix_ix(self):
        # ADD IX, IX doubles IX
        sim = Z80Simulator()
        prog = bytes([
            0xDD, 0x21, 0x34, 0x12,  # LD IX, 0x1234
            0xDD, 0x29,              # ADD IX, IX
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.ix == 0x2468


# ── INC/DEC IX ────────────────────────────────────────────────────────────────

class TestIncDecIX:
    def test_inc_ix(self):
        # DD 23: INC IX
        sim = Z80Simulator()
        r = sim.execute(bytes([0xDD, 0x21, 0xFF, 0x00, 0xDD, 0x23, 0x76]))
        assert r.final_state.ix == 0x0100

    def test_dec_iy(self):
        # FD 2B: DEC IY
        sim = Z80Simulator()
        r = sim.execute(bytes([0xFD, 0x21, 0x00, 0x01, 0xFD, 0x2B, 0x76]))
        assert r.final_state.iy == 0x00FF


# ── LD r, (IX+d) ──────────────────────────────────────────────────────────────

class TestLdRIxDisp:
    def test_ld_a_ix_plus_d(self):
        # DD 7E d: LD A, (IX+d)
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xBB, 0x32, 0x05, 0x10,  # LD (0x1005), 0xBB
            0xDD, 0x21, 0x00, 0x10,         # LD IX, 0x1000
            0xDD, 0x7E, 0x05,               # LD A, (IX+5)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0xBB

    def test_ld_b_ix_minus_d(self):
        # Negative displacement: IX=0x1010; LD B, (IX-1) = (0x100F)
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xCC, 0x32, 0x0F, 0x10,  # LD (0x100F), 0xCC
            0xDD, 0x21, 0x10, 0x10,         # LD IX, 0x1010
            0xDD, 0x46, 0xFF,               # LD B, (IX-1)  (0xFF = -1)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.b == 0xCC

    def test_ld_iy_displacement(self):
        # FD 7E d: LD A, (IY+d)
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xDD, 0x32, 0x03, 0x10,  # LD (0x1003), 0xDD
            0xFD, 0x21, 0x00, 0x10,         # LD IY, 0x1000
            0xFD, 0x7E, 0x03,               # LD A, (IY+3)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0xDD


# ── LD (IX+d), r ─────────────────────────────────────────────────────────────

class TestLdIxDispR:
    def test_ld_ix_plus_d_a(self):
        # DD 77 d: LD (IX+d), A
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x42,                     # LD A, 0x42
            0xDD, 0x21, 0x00, 0x20,         # LD IX, 0x2000
            0xDD, 0x77, 0x02,               # LD (IX+2), A
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x2002] == 0x42


# ── LD (IX+d), n ─────────────────────────────────────────────────────────────

class TestLdIxDispImm:
    def test_ld_ix_plus_d_n(self):
        # DD 36 d n: LD (IX+d), n
        sim = Z80Simulator()
        prog = bytes([
            0xDD, 0x21, 0x00, 0x30,  # LD IX, 0x3000
            0xDD, 0x36, 0x04, 0xAB,  # LD (IX+4), 0xAB
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x3004] == 0xAB


# ── INC/DEC (IX+d) ───────────────────────────────────────────────────────────

class TestIncDecIxDisp:
    def test_inc_ix_plus_d(self):
        # DD 34 d: INC (IX+d)
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x41, 0x32, 0x01, 0x10,  # LD (0x1001), 0x41
            0xDD, 0x21, 0x00, 0x10,         # LD IX, 0x1000
            0xDD, 0x34, 0x01,               # INC (IX+1)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x1001] == 0x42

    def test_dec_iy_plus_d(self):
        # FD 35 d: DEC (IY+d)
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x0A, 0x32, 0x00, 0x10,  # LD (0x1000), 0x0A
            0xFD, 0x21, 0x00, 0x10,         # LD IY, 0x1000
            0xFD, 0x35, 0x00,               # DEC (IY+0)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x1000] == 0x09


# ── ALU ops with (IX+d) ───────────────────────────────────────────────────────

class TestAluIxDisp:
    def test_add_a_ix_d(self):
        # DD 86 d: ADD A, (IX+d)
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x05, 0x32, 0x02, 0x10,  # LD (0x1002), 0x05
            0x3E, 0x0A,                     # LD A, 0x0A
            0xDD, 0x21, 0x00, 0x10,         # LD IX, 0x1000
            0xDD, 0x86, 0x02,               # ADD A, (IX+2)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x0F

    def test_cp_iy_d(self):
        # FD BE d: CP (IY+d)
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x42, 0x32, 0x00, 0x10,  # LD (0x1000), 0x42
            0x3E, 0x42,                     # LD A, 0x42
            0xFD, 0x21, 0x00, 0x10,         # LD IY, 0x1000
            0xFD, 0xBE, 0x00,               # CP (IY+0)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_z is True  # equal


# ── JP (IX) ───────────────────────────────────────────────────────────────────

class TestJpIX:
    def test_jp_ix(self):
        # DD E9: JP (IX) — jumps to address in IX.
        # Layout: LD IX(0-3), JP(IX)(4-5), NOP(6), HALT(7)
        # IX = 0x0007 so JP (IX) jumps to HALT at offset 7.
        sim = Z80Simulator()
        prog = bytes([
            0xDD, 0x21, 0x07, 0x00,  # LD IX, 0x0007  (0-3)
            0xDD, 0xE9,              # JP (IX)         (4-5)
            0x00,                    # NOP at 0x0006 (skipped)
            0x76,                    # HALT at 0x0007
        ])
        r = sim.execute(prog)
        assert r.final_state.halted is True
        assert r.final_state.pc == 0x0008  # PC points past HALT
