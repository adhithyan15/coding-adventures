"""Tests for Z80 block memory and search instructions (ED prefix).

Covers: LDI, LDD, LDIR, LDDR (block copy);
        CPI, CPD, CPIR, CPDR (block compare).
"""

from z80_simulator import Z80Simulator


def _make_sim_with_memory(patches: dict) -> "tuple[Z80Simulator, bytearray]":
    """Helper: return a fresh simulator with memory pre-patched."""
    sim = Z80Simulator()
    # We'll use the load+execute approach: pre-fill via LD instructions
    return sim


# ── LDI ───────────────────────────────────────────────────────────────────────

class TestLDI:
    def test_ldi_copies_one_byte(self):
        # LDI: (DE) ← (HL); HL++; DE++; BC--
        # Set HL→src, DE→dst, BC=1; call LDI
        sim = Z80Simulator()
        prog = bytes([
            # Prime memory: LD (0x1000), 0xAB
            0x3E, 0xAB,             # LD A, 0xAB
            0x32, 0x00, 0x10,       # LD (0x1000), A
            # Set registers
            0x21, 0x00, 0x10,       # LD HL, 0x1000 (source)
            0x11, 0x00, 0x20,       # LD DE, 0x2000 (dest)
            0x01, 0x01, 0x00,       # LD BC, 1
            0xED, 0xA0,             # LDI
            0x76,                   # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x2000] == 0xAB
        assert r.final_state.hl == 0x1001
        assert r.final_state.de == 0x2001
        assert r.final_state.bc == 0

    def test_ldi_pv_clear_when_bc_reaches_zero(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x01,             # LD A, 1
            0x32, 0x00, 0x10,       # LD (0x1000), A
            0x21, 0x00, 0x10,       # LD HL, 0x1000
            0x11, 0x00, 0x20,       # LD DE, 0x2000
            0x01, 0x01, 0x00,       # LD BC, 1
            0xED, 0xA0,             # LDI (BC→0)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_pv is False

    def test_ldi_pv_set_when_bc_nonzero(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x01,             # LD A, 1
            0x32, 0x00, 0x10,
            0x21, 0x00, 0x10,
            0x11, 0x00, 0x20,
            0x01, 0x02, 0x00,       # LD BC, 2
            0xED, 0xA0,             # LDI (BC→1, still nonzero)
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_pv is True


# ── LDD ───────────────────────────────────────────────────────────────────────

class TestLDD:
    def test_ldd_copies_byte_backwards(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xCD,             # LD A, 0xCD
            0x32, 0x04, 0x10,       # LD (0x1004), A
            0x21, 0x04, 0x10,       # LD HL, 0x1004 (source)
            0x11, 0x04, 0x20,       # LD DE, 0x2004 (dest)
            0x01, 0x01, 0x00,       # LD BC, 1
            0xED, 0xA8,             # LDD
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.memory[0x2004] == 0xCD
        assert r.final_state.hl == 0x1003
        assert r.final_state.de == 0x2003


# ── LDIR ──────────────────────────────────────────────────────────────────────

class TestLDIR:
    def test_ldir_copies_block(self):
        # Copy 5 bytes from 0x1000..0x1004 to 0x2000..0x2004
        sim = Z80Simulator()
        prog = bytes([
            # Fill source: 0x10, 0x11, 0x12, 0x13, 0x14 at 0x1000..0x1004
            0x3E, 0x10, 0x32, 0x00, 0x10,  # LD (0x1000),0x10
            0x3E, 0x11, 0x32, 0x01, 0x10,  # LD (0x1001),0x11
            0x3E, 0x12, 0x32, 0x02, 0x10,  # LD (0x1002),0x12
            0x3E, 0x13, 0x32, 0x03, 0x10,  # LD (0x1003),0x13
            0x3E, 0x14, 0x32, 0x04, 0x10,  # LD (0x1004),0x14
            # Set up LDIR
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0x11, 0x00, 0x20,   # LD DE, 0x2000
            0x01, 0x05, 0x00,   # LD BC, 5
            0xED, 0xB0,         # LDIR
            0x76,               # HALT
        ])
        r = sim.execute(prog)
        m = r.final_state.memory
        assert m[0x2000] == 0x10
        assert m[0x2001] == 0x11
        assert m[0x2002] == 0x12
        assert m[0x2003] == 0x13
        assert m[0x2004] == 0x14
        assert r.final_state.bc == 0
        assert r.final_state.flag_pv is False   # BC=0 after

    def test_ldir_bc_zero_after(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x42, 0x32, 0x00, 0x10,
            0x21, 0x00, 0x10,
            0x11, 0x00, 0x20,
            0x01, 0x01, 0x00,
            0xED, 0xB0,
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.bc == 0


# ── LDDR ──────────────────────────────────────────────────────────────────────

class TestLDDR:
    def test_lddr_copies_block_backwards(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0xAA, 0x32, 0x04, 0x10,  # LD (0x1004),0xAA
            0x3E, 0xBB, 0x32, 0x03, 0x10,  # LD (0x1003),0xBB
            0x3E, 0xCC, 0x32, 0x02, 0x10,  # LD (0x1002),0xCC
            0x21, 0x04, 0x10,   # LD HL, 0x1004 (end of source)
            0x11, 0x04, 0x20,   # LD DE, 0x2004 (end of dest)
            0x01, 0x03, 0x00,   # LD BC, 3
            0xED, 0xB8,         # LDDR
            0x76,
        ])
        r = sim.execute(prog)
        m = r.final_state.memory
        assert m[0x2002] == 0xCC
        assert m[0x2003] == 0xBB
        assert m[0x2004] == 0xAA


# ── CPI ───────────────────────────────────────────────────────────────────────

class TestCPI:
    def test_cpi_no_match(self):
        # CPI: compare A with (HL); HL++; BC--; Z not set when no match.
        # Use 0x1000 for data to avoid overlap with code bytes.
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x99, 0x32, 0x00, 0x10,   # LD (0x1000), 0x99
            0x3E, 0x01,                       # LD A, 0x01 (compare target)
            0x21, 0x00, 0x10,                 # LD HL, 0x1000
            0x01, 0x01, 0x00,                 # LD BC, 1
            0xED, 0xA1,                       # CPI  → 0x01 vs 0x99 → Z=0
            0x76,                             # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_z is False
        assert r.final_state.hl == 0x1001
        assert r.final_state.bc == 0

    def test_cpi_match(self):
        # A=0x99, (HL)=0x99 → match → Z=1
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x99, 0x32, 0x00, 0x10,   # LD (0x1000), 0x99
            0x3E, 0x99,                       # LD A, 0x99
            0x21, 0x00, 0x10,                 # LD HL, 0x1000
            0x01, 0x01, 0x00,                 # LD BC, 1
            0xED, 0xA1,                       # CPI
            0x76,                             # HALT
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_z is True


# ── CPIR ──────────────────────────────────────────────────────────────────────

class TestCPIR:
    def test_cpir_finds_byte(self):
        # Search for 0x42 in a 5-byte buffer
        sim = Z80Simulator()
        prog = bytes([
            # Write data: 0x01, 0x02, 0x42, 0x04, 0x05 at 0x1000..0x1004
            0x3E, 0x01, 0x32, 0x00, 0x10,
            0x3E, 0x02, 0x32, 0x01, 0x10,
            0x3E, 0x42, 0x32, 0x02, 0x10,  # target at 0x1002
            0x3E, 0x04, 0x32, 0x03, 0x10,
            0x3E, 0x05, 0x32, 0x04, 0x10,
            # CPIR setup
            0x3E, 0x42,         # LD A, 0x42 (target)
            0x21, 0x00, 0x10,   # LD HL, 0x1000
            0x01, 0x05, 0x00,   # LD BC, 5
            0xED, 0xB1,         # CPIR
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_z is True   # found it
        # After finding 0x42 at 0x1002, HL points to 0x1003
        assert r.final_state.hl == 0x1003

    def test_cpir_not_found(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x01, 0x32, 0x00, 0x10,
            0x3E, 0x02, 0x32, 0x01, 0x10,
            0x3E, 0x03, 0x32, 0x02, 0x10,
            0x3E, 0xFF,         # LD A, 0xFF (not in buffer)
            0x21, 0x00, 0x10,
            0x01, 0x03, 0x00,
            0xED, 0xB1,         # CPIR
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_z is False  # not found
        assert r.final_state.bc == 0          # exhausted


# ── CPD ───────────────────────────────────────────────────────────────────────

class TestCPD:
    def test_cpd_decrements_hl(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x55,             # LD A, 0x55
            0x21, 0x00, 0x10,       # LD HL, 0x1000
            0x01, 0x01, 0x00,       # LD BC, 1
            0xED, 0xA9,             # CPD
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.hl == 0x0FFF   # decremented


# ── CPDR ──────────────────────────────────────────────────────────────────────

class TestCPDR:
    def test_cpdr_finds_byte_backwards(self):
        sim = Z80Simulator()
        prog = bytes([
            0x3E, 0x42, 0x32, 0x00, 0x10,  # data[0]=0x42
            0x3E, 0x01, 0x32, 0x01, 0x10,
            0x3E, 0x02, 0x32, 0x02, 0x10,
            0x3E, 0x42,         # LD A, 0x42
            0x21, 0x02, 0x10,   # LD HL, 0x1002 (start from end)
            0x01, 0x03, 0x00,   # LD BC, 3
            0xED, 0xB9,         # CPDR
            0x76,
        ])
        r = sim.execute(prog)
        assert r.final_state.flag_z is True
        # Found 0x42 at 0x1000; after CPD HL=0x0FFF
        assert r.final_state.hl == 0x0FFF
