"""Tests for Z80 load and store instructions.

Covers: LD r,n; LD r,r'; LD r,(HL); LD (HL),r; 16-bit LD rp,nn;
        LD SP,HL; LD A,(BC/DE/nn); LD (BC/DE/nn),A;
        LD HL,(nn); LD (nn),HL; PUSH/POP.
"""

from z80_simulator import Z80Simulator

# ── LD r, n (8-bit immediate) ─────────────────────────────────────────────────

class TestLdRImmediate:
    def test_ld_a_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x42, 0x76]))   # LD A,0x42; HALT
        assert r.final_state.a == 0x42

    def test_ld_b_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x06, 0x11, 0x76]))
        assert r.final_state.b == 0x11

    def test_ld_c_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x0E, 0x22, 0x76]))
        assert r.final_state.c == 0x22

    def test_ld_d_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x16, 0x33, 0x76]))
        assert r.final_state.d == 0x33

    def test_ld_e_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x1E, 0x44, 0x76]))
        assert r.final_state.e == 0x44

    def test_ld_h_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x26, 0x55, 0x76]))
        assert r.final_state.h == 0x55

    def test_ld_l_n(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x2E, 0x66, 0x76]))
        assert r.final_state.l == 0x66


# ── LD r, r' (register-to-register) ──────────────────────────────────────────

class TestLdRR:
    def test_ld_a_b(self):
        # LD B,0x55; LD A,B; HALT
        sim = Z80Simulator()
        r = sim.execute(bytes([0x06, 0x55, 0x78, 0x76]))
        assert r.final_state.a == 0x55

    def test_ld_b_a(self):
        # LD A,0x77; LD B,A; HALT
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x77, 0x47, 0x76]))
        assert r.final_state.b == 0x77

    def test_ld_c_d(self):
        # LD D,0x11; LD C,D; HALT
        sim = Z80Simulator()
        r = sim.execute(bytes([0x16, 0x11, 0x4A, 0x76]))
        assert r.final_state.c == 0x11

    def test_ld_hl_memory(self):
        # LD HL,0x0100; LD A,(HL); HALT — memory[0x0100] = 0x00 initially
        sim = Z80Simulator()
        # Put a value at 0x0100 first via load + offset trick
        prog = bytes([
            0x21, 0x05, 0x00,   # LD HL, 0x0005 (address of the 0xAB byte below)
            0x7E,               # LD A, (HL)
            0x76,               # HALT
            0xAB,               # data byte
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0xAB

    def test_ld_hl_store(self):
        # LD A,0x5A; LD HL,0x0100; LD (HL),A; HALT — check memory
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x5A,         # LD A, 0x5A
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0x77,               # LD (HL), A
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x1000] == 0x5A


# ── LD rp, nn (16-bit immediate) ─────────────────────────────────────────────

class TestLdRpImmediate:
    def test_ld_bc_nn(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x01, 0x34, 0x12, 0x76]))   # LD BC, 0x1234
        assert r.final_state.b == 0x12
        assert r.final_state.c == 0x34
        assert r.final_state.bc == 0x1234

    def test_ld_de_nn(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x11, 0x78, 0x56, 0x76]))   # LD DE, 0x5678
        assert r.final_state.d == 0x56
        assert r.final_state.e == 0x78
        assert r.final_state.de == 0x5678

    def test_ld_hl_nn(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x21, 0xBC, 0x9A, 0x76]))   # LD HL, 0x9ABC
        assert r.final_state.h == 0x9A
        assert r.final_state.l == 0xBC
        assert r.final_state.hl == 0x9ABC

    def test_ld_sp_nn(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x31, 0xFF, 0xFF, 0x76]))   # LD SP, 0xFFFF
        assert r.final_state.sp == 0xFFFF

    def test_ld_sp_hl(self):
        # LD HL, 0x8000; LD SP, HL
        sim = Z80Simulator()
        r = sim.execute(bytes([0x21, 0x00, 0x80, 0xF9, 0x76]))
        assert r.final_state.sp == 0x8000


# ── LD A, (BC/DE) and LD (BC/DE), A ─────────────────────────────────────────

class TestLdIndirectBC_DE:
    def test_ld_a_bc_indirect(self):
        # Program: LD BC, addr; LD A, (BC); HALT
        # where addr points to the 0xCC byte after HALT
        sim = Z80Simulator()
        prog = bytes([
            0x01, 0x05, 0x00,   # LD BC, 0x0005
            0x0A,               # LD A, (BC)
            0x76,               # HALT
            0xCC,               # data
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0xCC

    def test_ld_a_de_indirect(self):
        sim = Z80Simulator()
        prog = bytes([
            0x11, 0x05, 0x00,   # LD DE, 0x0005
            0x1A,               # LD A, (DE)
            0x76,               # HALT
            0xDD,               # data
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0xDD

    def test_ld_bc_a_store(self):
        # LD A,0xBE; LD BC,0x1000; LD (BC),A
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xBE,         # LD A, 0xBE
            0x01, 0x00, 0x10,   # LD BC, 0x1000
            0x02,               # LD (BC), A
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x1000] == 0xBE

    def test_ld_de_a_store(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xEF,         # LD A, 0xEF
            0x11, 0x00, 0x20,   # LD DE, 0x2000
            0x12,               # LD (DE), A
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x2000] == 0xEF


# ── LD A, (nn) and LD (nn), A ─────────────────────────────────────────────────

class TestLdAbsolute:
    def test_ld_a_nn(self):
        # LD A, (0x0006); HALT; data 0x99
        sim = Z80Simulator()
        prog = bytes([
            0x3A, 0x06, 0x00,   # LD A, (0x0006)
            0x76,               # HALT
            0x00,               # padding
            0x00,               # padding
            0x99,               # data at 0x0006
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x99

    def test_ld_nn_a(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x77,         # LD A, 0x77
            0x32, 0x00, 0x10,   # LD (0x1000), A
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x1000] == 0x77

    def test_ld_hl_nn_indirect(self):
        # LD (nn), HL then LD HL, (nn) to verify roundtrip
        sim = Z80Simulator()
        prog = bytes([
            0x21, 0xAB, 0xCD,   # LD HL, 0xCDAB
            0x22, 0x00, 0x10,   # LD (0x1000), HL
            0x21, 0x00, 0x00,   # LD HL, 0
            0x2A, 0x00, 0x10,   # LD HL, (0x1000)
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.hl == 0xCDAB
        assert r.final_state.memory[0x1000] == 0xAB
        assert r.final_state.memory[0x1001] == 0xCD


# ── PUSH / POP ────────────────────────────────────────────────────────────────

class TestPushPop:
    def test_push_pop_bc(self):
        # Push BC then pop it into DE
        sim = Z80Simulator()
        prog = bytes([
            0x01, 0x34, 0x12,   # LD BC, 0x1234
            0x31, 0x00, 0x80,   # LD SP, 0x8000
            0xC5,               # PUSH BC
            0x11, 0x00, 0x00,   # LD DE, 0 (clobber)
            0xD1,               # POP DE
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.de == 0x1234

    def test_push_af_pop_af(self):
        # Push AF, then pop it back after clobbering A
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x55,         # LD A, 0x55
            0x31, 0x00, 0x80,   # LD SP, 0x8000
            0xF5,               # PUSH AF
            0x3E, 0x00,         # LD A, 0
            0xF1,               # POP AF
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x55

    def test_stack_pointer_decrements_on_push(self):
        sim = Z80Simulator()
        prog = bytes([
            0x31, 0x00, 0x80,   # LD SP, 0x8000
            0x21, 0x34, 0x12,   # LD HL, 0x1234
            0xE5,               # PUSH HL
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.sp == 0x7FFE

    def test_stack_pointer_increments_on_pop(self):
        sim = Z80Simulator()
        prog = bytes([
            0x31, 0x00, 0x80,   # LD SP, 0x8000
            0x21, 0x34, 0x12,   # LD HL, 0x1234
            0xE5,               # PUSH HL
            0xE1,               # POP HL
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.sp == 0x8000
