"""test_register_file.py — Unit tests for register_file.py.

Tests:
  - Read/write all 32 registers
  - r31 always returns 0 (writes discarded)
  - PC read/write
  - increment_pc(4) via gate-level add_64bit
"""

from __future__ import annotations

from alpha_axp_gatelevel.register_file import RegisterFile64

MASK64 = 0xFFFF_FFFF_FFFF_FFFF


class TestRegisterFile:
    """Tests for RegisterFile64."""

    def setup_method(self):
        self.rf = RegisterFile64()

    # ── Initial state ──────────────────────────────────────────────────────────

    def test_initial_all_zeros(self):
        for i in range(32):
            assert self.rf.read_reg(i) == 0

    def test_initial_pc_zero(self):
        assert self.rf.read_pc() == 0

    # ── Write and read back ────────────────────────────────────────────────────

    def test_write_read_r0(self):
        self.rf.write_reg(0, 42)
        assert self.rf.read_reg(0) == 42

    def test_write_read_r1(self):
        self.rf.write_reg(1, 0xDEAD_BEEF)
        assert self.rf.read_reg(1) == 0xDEAD_BEEF

    def test_write_read_r30(self):
        self.rf.write_reg(30, 0x1234_5678_9ABC_DEF0)
        assert self.rf.read_reg(30) == 0x1234_5678_9ABC_DEF0

    def test_write_read_all_non_zero(self):
        for i in range(31):   # skip r31
            self.rf.write_reg(i, i * 100 + 1)
        for i in range(31):
            assert self.rf.read_reg(i) == i * 100 + 1

    def test_max_value(self):
        self.rf.write_reg(5, MASK64)
        assert self.rf.read_reg(5) == MASK64

    def test_mask_to_64bits(self):
        # Values wider than 64 bits are masked
        self.rf.write_reg(5, MASK64 + 1)  # = 2^64 = 0 mod 2^64
        assert self.rf.read_reg(5) == 0

    # ── r31 hardwired zero ─────────────────────────────────────────────────────

    def test_r31_read_always_zero(self):
        assert self.rf.read_reg(31) == 0

    def test_r31_write_discarded(self):
        self.rf.write_reg(31, 0xDEAD_BEEF)
        assert self.rf.read_reg(31) == 0

    def test_r31_write_discarded_large(self):
        self.rf.write_reg(31, MASK64)
        assert self.rf.read_reg(31) == 0

    # ── PC ────────────────────────────────────────────────────────────────────

    def test_write_read_pc(self):
        self.rf.write_pc(0x1000)
        assert self.rf.read_pc() == 0x1000

    def test_write_read_pc_large(self):
        self.rf.write_pc(0xFFFF)
        assert self.rf.read_pc() == 0xFFFF

    def test_increment_pc_by_4(self):
        self.rf.write_pc(0)
        self.rf.increment_pc(4)
        assert self.rf.read_pc() == 4

    def test_increment_pc_multiple(self):
        self.rf.write_pc(0)
        for _ in range(10):
            self.rf.increment_pc(4)
        assert self.rf.read_pc() == 40

    def test_increment_pc_wraps(self):
        # PC wrapping at 64-bit boundary
        self.rf.write_pc(MASK64)
        self.rf.increment_pc(4)
        assert self.rf.read_pc() == 3  # (MASK64 + 4) & MASK64 = 3

    # ── Reset ─────────────────────────────────────────────────────────────────

    def test_reset_clears_registers(self):
        for i in range(31):
            self.rf.write_reg(i, 0xDEAD_BEEF)
        self.rf.write_pc(0x5000)
        self.rf.reset()
        for i in range(32):
            assert self.rf.read_reg(i) == 0
        assert self.rf.read_pc() == 0

    # ── Snapshot ──────────────────────────────────────────────────────────────

    def test_get_regs_tuple(self):
        for i in range(31):
            self.rf.write_reg(i, i * 7)
        t = self.rf.get_regs_tuple()
        assert len(t) == 32
        for i in range(31):
            assert t[i] == i * 7
        assert t[31] == 0  # r31 always 0
