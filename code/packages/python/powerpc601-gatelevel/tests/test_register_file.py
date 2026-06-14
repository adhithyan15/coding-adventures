"""test_register_file.py — Unit tests for the PowerPC 601 gate-level register file.

Tests cover:
- Read/write all 32 GPRs
- LR, CTR, XER, CR, CIA
- increment_cia (gate-level)
- set_cr_field (gate-level)
- get_cr_bit (gate-level)
- Reset
"""

from __future__ import annotations

from powerpc601_gatelevel.register_file import RegisterFilePPC


class TestGPRAccess:
    def test_initial_all_zero(self):
        rf = RegisterFilePPC()
        for i in range(32):
            assert rf.read_gpr(i) == 0

    def test_write_read_all_gprs(self):
        rf = RegisterFilePPC()
        for i in range(32):
            rf.write_gpr(i, i * 100 + 1)
        for i in range(32):
            assert rf.read_gpr(i) == i * 100 + 1

    def test_write_truncates_to_32bit(self):
        rf = RegisterFilePPC()
        rf.write_gpr(0, 0xFFFFFFFF + 1)  # 2^32 → 0
        assert rf.read_gpr(0) == 0

    def test_write_negative_masked(self):
        rf = RegisterFilePPC()
        rf.write_gpr(3, -1)
        assert rf.read_gpr(3) == 0xFFFFFFFF

    def test_gpr0_can_hold_nonzero(self):
        # GPR0 can hold any value (special case is in EA calc, not here)
        rf = RegisterFilePPC()
        rf.write_gpr(0, 42)
        assert rf.read_gpr(0) == 42

    def test_gpr31_write_read(self):
        rf = RegisterFilePPC()
        rf.write_gpr(31, 0xDEADBEEF)
        assert rf.read_gpr(31) == 0xDEADBEEF

    def test_independent_gprs(self):
        rf = RegisterFilePPC()
        rf.write_gpr(5, 100)
        rf.write_gpr(6, 200)
        assert rf.read_gpr(5) == 100
        assert rf.read_gpr(6) == 200


class TestLRAccess:
    def test_initial_zero(self):
        rf = RegisterFilePPC()
        assert rf.read_lr() == 0

    def test_write_read(self):
        rf = RegisterFilePPC()
        rf.write_lr(0xDEAD0000)
        assert rf.read_lr() == 0xDEAD0000

    def test_truncated(self):
        rf = RegisterFilePPC()
        rf.write_lr(-1)
        assert rf.read_lr() == 0xFFFFFFFF


class TestCTRAccess:
    def test_initial_zero(self):
        rf = RegisterFilePPC()
        assert rf.read_ctr() == 0

    def test_write_read(self):
        rf = RegisterFilePPC()
        rf.write_ctr(10)
        assert rf.read_ctr() == 10

    def test_truncated(self):
        rf = RegisterFilePPC()
        rf.write_ctr(0x100000000)
        assert rf.read_ctr() == 0


class TestXERAccess:
    def test_initial_zero(self):
        rf = RegisterFilePPC()
        assert rf.read_xer() == 0

    def test_write_read_ca_bit(self):
        rf = RegisterFilePPC()
        rf.write_xer(1 << 29)  # XER_CA
        assert rf.read_xer() == (1 << 29)

    def test_write_read_so_bit(self):
        rf = RegisterFilePPC()
        rf.write_xer(1 << 31)  # XER_SO
        assert rf.read_xer() == (1 << 31)


class TestCRAccess:
    def test_initial_zero(self):
        rf = RegisterFilePPC()
        assert rf.read_cr() == 0

    def test_write_read(self):
        rf = RegisterFilePPC()
        rf.write_cr(0xF0000000)
        assert rf.read_cr() == 0xF0000000

    def test_write_all_ones(self):
        rf = RegisterFilePPC()
        rf.write_cr(0xFFFFFFFF)
        assert rf.read_cr() == 0xFFFFFFFF


class TestCIAAccess:
    def test_initial_zero(self):
        rf = RegisterFilePPC()
        assert rf.read_cia() == 0

    def test_write_read(self):
        rf = RegisterFilePPC()
        rf.write_cia(0x1000)
        assert rf.read_cia() == 0x1000

    def test_write_large(self):
        rf = RegisterFilePPC()
        rf.write_cia(0xFFFFFFFC)
        assert rf.read_cia() == 0xFFFFFFFC


class TestIncrementCIA:
    def test_increment_by_four(self):
        rf = RegisterFilePPC()
        rf.write_cia(0x1000)
        rf.increment_cia(4)
        assert rf.read_cia() == 0x1004

    def test_increment_default(self):
        rf = RegisterFilePPC()
        rf.write_cia(0x100)
        rf.increment_cia()
        assert rf.read_cia() == 0x104

    def test_increment_wraps(self):
        rf = RegisterFilePPC()
        rf.write_cia(0xFFFFFFFC)
        rf.increment_cia(4)
        assert rf.read_cia() == 0

    def test_multiple_increments(self):
        rf = RegisterFilePPC()
        rf.write_cia(0)
        for _ in range(10):
            rf.increment_cia(4)
        assert rf.read_cia() == 40


class TestSetCRField:
    def test_set_cr0_lt(self):
        rf = RegisterFilePPC()
        rf.set_cr_field(0, lt=1, gt=0, eq=0, so=0)
        # CR0 is bits [31:28]: LT=bit31, GT=bit30, EQ=bit29, SO=bit28
        # LT=1 → nibble=0b1000 → value 8 at shift 28 = 0x80000000
        assert rf.read_cr() == 0x80000000

    def test_set_cr0_eq(self):
        rf = RegisterFilePPC()
        rf.set_cr_field(0, lt=0, gt=0, eq=1, so=0)
        # EQ=1 → nibble=0b0010 → 2 at shift 28 = 0x20000000
        assert rf.read_cr() == 0x20000000

    def test_set_cr0_gt(self):
        rf = RegisterFilePPC()
        rf.set_cr_field(0, lt=0, gt=1, eq=0, so=0)
        # GT=1 → nibble=0b0100 → 4 at shift 28 = 0x40000000
        assert rf.read_cr() == 0x40000000

    def test_set_cr7_lt(self):
        # CR7 is the LSB nibble (bits [3:0])
        rf = RegisterFilePPC()
        rf.set_cr_field(7, lt=1, gt=0, eq=0, so=0)
        # shift = 28 - 7*4 = 0; nibble=0b1000=8 at shift 0 = 8
        assert rf.read_cr() == 8

    def test_set_multiple_fields(self):
        rf = RegisterFilePPC()
        rf.set_cr_field(0, lt=1, gt=0, eq=0, so=0)  # CR0
        rf.set_cr_field(7, lt=0, gt=1, eq=0, so=0)  # CR7
        cr = rf.read_cr()
        # CR0 nibble = 0b1000 at bit 31 = 0x80000000
        # CR7 nibble = 0b0100 at bit 3 = 0x4
        assert cr == (0x80000000 | 0x4)

    def test_overwrite_field(self):
        rf = RegisterFilePPC()
        rf.set_cr_field(0, lt=1, gt=1, eq=1, so=1)
        rf.set_cr_field(0, lt=0, gt=0, eq=1, so=0)
        # After overwrite, CR0 = 0b0010 = 2 at shift 28 = 0x20000000
        assert rf.read_cr() == 0x20000000

    def test_all_bits_in_field(self):
        rf = RegisterFilePPC()
        rf.set_cr_field(0, lt=1, gt=1, eq=1, so=1)
        # nibble=0b1111=15 at shift 28 = 0xF0000000
        assert rf.read_cr() == 0xF0000000


class TestGetCRBit:
    def test_bit_zero_is_cr0_lt(self):
        rf = RegisterFilePPC()
        rf.write_cr(0x80000000)  # bit 31 set = CR0.LT
        assert rf.get_cr_bit(0) == 1  # BI=0 → bit 31

    def test_bit_zero_clear(self):
        rf = RegisterFilePPC()
        rf.write_cr(0)
        assert rf.get_cr_bit(0) == 0

    def test_bit_two_is_cr0_eq(self):
        rf = RegisterFilePPC()
        rf.write_cr(0x20000000)  # CR0.EQ = bit 29
        assert rf.get_cr_bit(2) == 1  # BI=2 → bit 29

    def test_bit_31_is_lsb(self):
        rf = RegisterFilePPC()
        rf.write_cr(0x00000001)  # bit 0 set
        assert rf.get_cr_bit(31) == 1  # BI=31 → bit 0

    def test_all_bits_clear(self):
        rf = RegisterFilePPC()
        rf.write_cr(0)
        for bi in range(32):
            assert rf.get_cr_bit(bi) == 0

    def test_all_bits_set(self):
        rf = RegisterFilePPC()
        rf.write_cr(0xFFFFFFFF)
        for bi in range(32):
            assert rf.get_cr_bit(bi) == 1


class TestReset:
    def test_reset_clears_gprs(self):
        rf = RegisterFilePPC()
        for i in range(32):
            rf.write_gpr(i, 0xDEADBEEF)
        rf.reset()
        for i in range(32):
            assert rf.read_gpr(i) == 0

    def test_reset_clears_sprs(self):
        rf = RegisterFilePPC()
        rf.write_lr(0x1234)
        rf.write_ctr(0x5678)
        rf.write_xer(0xABCD)
        rf.write_cr(0xFFFF)
        rf.write_cia(0x1000)
        rf.reset()
        assert rf.read_lr() == 0
        assert rf.read_ctr() == 0
        assert rf.read_xer() == 0
        assert rf.read_cr() == 0
        assert rf.read_cia() == 0

    def test_get_gprs_tuple(self):
        rf = RegisterFilePPC()
        for i in range(32):
            rf.write_gpr(i, i)
        gprs = rf.get_gprs_tuple()
        assert len(gprs) == 32
        for i in range(32):
            assert gprs[i] == i
