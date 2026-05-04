"""Tests for Z80 I/O instructions.

Covers: OUT (n), A; IN A, (n);
        ED-prefix: IN r, (C); OUT (C), r;
        Block I/O: OTIR, OTDR, INIR, INDR.
"""

from z80_simulator import Z80Simulator

# ── OUT (n), A ────────────────────────────────────────────────────────────────

class TestOutN:
    def test_out_port_7(self):
        sim = Z80Simulator()
        sim.execute(bytes([0x3E, 0x42, 0xD3, 0x07, 0x76]))
        assert sim.get_output_port(7) == 0x42

    def test_out_port_0(self):
        sim = Z80Simulator()
        sim.execute(bytes([0x3E, 0xFF, 0xD3, 0x00, 0x76]))
        assert sim.get_output_port(0) == 0xFF

    def test_out_port_ff(self):
        sim = Z80Simulator()
        sim.execute(bytes([0x3E, 0x01, 0xD3, 0xFF, 0x76]))
        assert sim.get_output_port(0xFF) == 0x01

    def test_out_does_not_change_a(self):
        sim = Z80Simulator()
        r = sim.execute(bytes([0x3E, 0x55, 0xD3, 0x10, 0x76]))
        assert r.final_state.a == 0x55


# ── IN A, (n) ─────────────────────────────────────────────────────────────────

class TestInN:
    def test_in_reads_port(self):
        sim = Z80Simulator()
        sim.set_input_port(0x10, 0xBB)
        r = sim.execute(bytes([0xDB, 0x10, 0x76]))
        assert r.final_state.a == 0xBB

    def test_in_port_0(self):
        sim = Z80Simulator()
        sim.set_input_port(0, 0x99)
        r = sim.execute(bytes([0xDB, 0x00, 0x76]))
        assert r.final_state.a == 0x99

    def test_in_different_ports_independent(self):
        sim = Z80Simulator()
        sim.set_input_port(0x01, 0xAA)
        sim.set_input_port(0x02, 0xBB)
        r = sim.execute(bytes([0xDB, 0x01, 0x76]))
        assert r.final_state.a == 0xAA


# ── IN r, (C) and OUT (C), r ─────────────────────────────────────────────────

class TestInOutC:
    def test_in_b_c(self):
        # ED 40: IN B, (C) — reads from port C into B
        sim = Z80Simulator()
        sim.set_input_port(0x05, 0x77)
        prog = bytes([
            0x0E, 0x05,   # LD C, 5 (port)
            0xED, 0x40,   # IN B, (C)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.b == 0x77

    def test_in_a_c(self):
        # ED 78: IN A, (C)
        sim = Z80Simulator()
        sim.set_input_port(0x0A, 0x33)
        prog = bytes([
            0x0E, 0x0A,   # LD C, 0x0A
            0xED, 0x78,   # IN A, (C)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.a == 0x33

    def test_out_c_b(self):
        # ED 41: OUT (C), B — writes B to port C
        sim = Z80Simulator()
        prog = bytes([
            0x06, 0xAB,   # LD B, 0xAB
            0x0E, 0x07,   # LD C, 7
            0xED, 0x41,   # OUT (C), B
            0x76,
        ])
        sim.execute(prog)
        assert sim.get_output_port(7) == 0xAB

    def test_out_c_a(self):
        # ED 79: OUT (C), A
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x55,   # LD A, 0x55
            0x0E, 0x03,   # LD C, 3
            0xED, 0x79,   # OUT (C), A
            0x76,
        ])
        sim.execute(prog)
        assert sim.get_output_port(3) == 0x55

    def test_in_sets_flags(self):
        # IN r,(C) sets S, Z, PV (parity), clears H, N
        sim = Z80Simulator()
        sim.set_input_port(0x01, 0x00)  # reading 0 → Z=1
        prog = bytes([
            0x0E, 0x01,   # LD C, 1
            0xED, 0x40,   # IN B, (C)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_z is True
        assert r.final_state.flag_h is False
        assert r.final_state.flag_n is False


# ── OTIR ─────────────────────────────────────────────────────────────────────

class TestOTIR:
    def test_otir_sends_buffer(self):
        # Write 3 bytes [0xAA, 0xBB, 0xCC] to port 5 using OTIR
        sim = Z80Simulator()
        prog = bytes([
            # Fill buffer at 0x1000
            0x3E, 0xAA, 0x32, 0x00, 0x10,
            0x3E, 0xBB, 0x32, 0x01, 0x10,
            0x3E, 0xCC, 0x32, 0x02, 0x10,
            # OTIR setup: HL=src, B=count, C=port
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0x06, 0x03,         # LD B, 3
            0x0E, 0x05,         # LD C, 5
            0xED, 0xB3,         # OTIR
            0x76,
        ])
        sim.execute(prog)
        # Last value written to port 5 should be 0xCC
        assert sim.get_output_port(5) == 0xCC

    def test_otir_decrements_b_to_zero(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x01, 0x32, 0x00, 0x10,
            0x21, 0x00, 0x10,
            0x06, 0x01,
            0x0E, 0x00,
            0xED, 0xB3,
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.b == 0


# ── OTDR ─────────────────────────────────────────────────────────────────────

class TestOTDR:
    def test_otdr_sends_buffer_backwards(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x11, 0x32, 0x00, 0x10,
            0x3E, 0x22, 0x32, 0x01, 0x10,
            0x3E, 0x33, 0x32, 0x02, 0x10,
            0x21, 0x02, 0x10,   # LD HL, 0x1002 (start from end)
            0x06, 0x03,
            0x0E, 0x07,
            0xED, 0xBB,         # OTDR
            0x76,
        ])
        sim.execute(prog)
        # Last value written is from 0x1000 = 0x11
        assert sim.get_output_port(7) == 0x11


# ── INIR ─────────────────────────────────────────────────────────────────────

class TestINIR:
    def test_inir_fills_buffer(self):
        # Read from port 3 into buffer at 0x2000; B=2 iterations
        sim = Z80Simulator()
        sim.set_input_port(3, 0x42)
        prog = bytes([
            0x21, 0x00, 0x20,   # LD HL, 0x2000
            0x06, 0x02,         # LD B, 2
            0x0E, 0x03,         # LD C, 3
            0xED, 0xB2,         # INIR
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x2000] == 0x42
        assert r.final_state.memory[0x2001] == 0x42
        assert r.final_state.b == 0


# ── INDR ─────────────────────────────────────────────────────────────────────

class TestINDR:
    def test_indr_fills_buffer_backwards(self):
        sim = Z80Simulator()
        sim.set_input_port(2, 0x77)
        prog = bytes([
            0x21, 0x02, 0x20,   # LD HL, 0x2002
            0x06, 0x03,         # LD B, 3
            0x0E, 0x02,         # LD C, 2
            0xED, 0xBA,         # INDR
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x2000] == 0x77
        assert r.final_state.memory[0x2001] == 0x77
        assert r.final_state.memory[0x2002] == 0x77
        assert r.final_state.b == 0
