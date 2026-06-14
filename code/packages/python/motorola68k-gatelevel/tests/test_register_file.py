"""Tests for register_file.py — RegisterFile68k."""


from motorola68k_gatelevel.register_file import RegisterFile68k


class TestDataRegister:
    """Dn read/write with byte/word/long size."""

    def setup_method(self):
        self.rf = RegisterFile68k()

    def test_write_long_read_long(self):
        self.rf.write_dn(0, 0xDEADBEEF, 4)
        assert self.rf.read_dn(0, 4) == 0xDEADBEEF

    def test_write_word_read_word(self):
        self.rf.write_dn(1, 0xABCD, 2)
        assert self.rf.read_dn(1, 2) == 0xABCD

    def test_write_byte_read_byte(self):
        self.rf.write_dn(2, 0x42, 1)
        assert self.rf.read_dn(2, 1) == 0x42

    def test_byte_write_preserves_upper(self):
        self.rf.write_dn(0, 0xDEADBEEF, 4)
        self.rf.write_dn(0, 0x42, 1)
        assert self.rf.read_dn(0, 4) == 0xDEADBE42

    def test_word_write_preserves_upper(self):
        self.rf.write_dn(0, 0xDEADBEEF, 4)
        self.rf.write_dn(0, 0x1234, 2)
        assert self.rf.read_dn(0, 4) == 0xDEAD1234

    def test_long_write_replaces_all(self):
        self.rf.write_dn(0, 0xFFFFFFFF, 4)
        self.rf.write_dn(0, 0x12345678, 4)
        assert self.rf.read_dn(0, 4) == 0x12345678

    def test_all_registers_independent(self):
        for i in range(8):
            self.rf.write_dn(i, i * 0x11111111, 4)
        for i in range(8):
            assert self.rf.read_dn(i, 4) == i * 0x11111111

    def test_word_mask(self):
        self.rf.write_dn(0, 0x12345678, 4)
        assert self.rf.read_dn(0, 2) == 0x5678

    def test_byte_mask(self):
        self.rf.write_dn(0, 0x12345678, 4)
        assert self.rf.read_dn(0, 1) == 0x78

    def test_initial_value_zero(self):
        for i in range(8):
            assert self.rf.read_dn(i, 4) == 0


class TestAddressRegister:
    """An read/write — always 32-bit."""

    def setup_method(self):
        self.rf = RegisterFile68k()

    def test_write_read(self):
        self.rf.write_an(0, 0xDEADBEEF)
        assert self.rf.read_an(0) == 0xDEADBEEF

    def test_all_registers(self):
        for i in range(8):
            self.rf.write_an(i, 0x1000 * i)
        for i in range(8):
            assert self.rf.read_an(i) == 0x1000 * i

    def test_initial_a7(self):
        assert self.rf.read_an(7) == 0x00F000

    def test_initial_others_zero(self):
        for i in range(7):
            assert self.rf.read_an(i) == 0

    def test_mask_to_32bit(self):
        self.rf.write_an(0, 0x1FFFFFFFF)
        assert self.rf.read_an(0) == 0xFFFFFFFF


class TestPC:
    """Program counter read/write."""

    def setup_method(self):
        self.rf = RegisterFile68k()

    def test_initial_value(self):
        assert self.rf.read_pc() == 0x001000

    def test_write_read(self):
        self.rf.write_pc(0x00DEAD)
        assert self.rf.read_pc() == 0x00DEAD

    def test_mask_32bit(self):
        self.rf.write_pc(0x1001000)  # 25-bit value, should mask to 32-bit
        assert self.rf.read_pc() == 0x1001000 & 0xFFFFFFFF


class TestCCR:
    """CCR pack/unpack."""

    def setup_method(self):
        self.rf = RegisterFile68k()

    def test_initial_all_zero(self):
        assert self.rf.pack_ccr() == 0

    def test_set_z(self):
        self.rf._flag_z = 1
        assert self.rf.pack_ccr() == 0x04  # bit 2

    def test_set_n(self):
        self.rf._flag_n = 1
        assert self.rf.pack_ccr() == 0x08  # bit 3

    def test_set_v(self):
        self.rf._flag_v = 1
        assert self.rf.pack_ccr() == 0x02  # bit 1

    def test_set_c(self):
        self.rf._flag_c = 1
        assert self.rf.pack_ccr() == 0x01  # bit 0

    def test_set_x(self):
        self.rf._flag_x = 1
        assert self.rf.pack_ccr() == 0x10  # bit 4

    def test_all_set(self):
        self.rf._flag_x = 1
        self.rf._flag_n = 1
        self.rf._flag_z = 1
        self.rf._flag_v = 1
        self.rf._flag_c = 1
        assert self.rf.pack_ccr() == 0x1F

    def test_unpack_ccr(self):
        self.rf.unpack_ccr(0x1F)
        assert self.rf._flag_x == 1
        assert self.rf._flag_n == 1
        assert self.rf._flag_z == 1
        assert self.rf._flag_v == 1
        assert self.rf._flag_c == 1

    def test_unpack_zero(self):
        self.rf._flag_z = 1
        self.rf.unpack_ccr(0)
        assert self.rf._flag_z == 0
        assert self.rf.pack_ccr() == 0

    def test_round_trip(self):
        for ccr in [0, 1, 4, 0x1F, 0x10, 0x0E]:
            self.rf.unpack_ccr(ccr)
            assert self.rf.pack_ccr() == ccr


class TestSR:
    """SR pack/unpack."""

    def setup_method(self):
        self.rf = RegisterFile68k()

    def test_initial_sr(self):
        sr = self.rf.pack_sr()
        assert sr & 0x2000  # S bit set
        assert (sr >> 8) & 7 == 7  # IMask=7

    def test_supervisor_always_set(self):
        self.rf.unpack_sr(0)  # try to clear S
        assert self.rf._flag_s == 1
        assert self.rf.pack_sr() & 0x2000

    def test_interrupt_mask(self):
        self.rf._int_mask = 3
        sr = self.rf.pack_sr()
        assert (sr >> 8) & 7 == 3

    def test_unpack_imask(self):
        self.rf.unpack_sr(0x0300)  # IMask = 3
        assert self.rf._int_mask == 3

    def test_ccr_in_sr(self):
        self.rf._flag_z = 1
        sr = self.rf.pack_sr()
        assert sr & 0x04  # Z bit

    def test_round_trip_ccr_portion(self):
        # Set all CCR flags
        self.rf.unpack_sr(0x2700 | 0x1F)
        sr = self.rf.pack_sr()
        assert sr & 0x1F == 0x1F


class TestReset:
    """Reset restores power-on defaults."""

    def test_reset_clears_d(self):
        rf = RegisterFile68k()
        rf.write_dn(0, 0xDEADBEEF, 4)
        rf.reset()
        assert rf.read_dn(0, 4) == 0

    def test_reset_clears_a(self):
        rf = RegisterFile68k()
        rf.write_an(0, 0xDEADBEEF)
        rf.reset()
        assert rf.read_an(0) == 0

    def test_reset_a7(self):
        rf = RegisterFile68k()
        rf.write_an(7, 0x12345)
        rf.reset()
        assert rf.read_an(7) == 0x00F000

    def test_reset_pc(self):
        rf = RegisterFile68k()
        rf.write_pc(0xDEAD)
        rf.reset()
        assert rf.read_pc() == 0x001000

    def test_reset_flags(self):
        rf = RegisterFile68k()
        rf._flag_z = 1
        rf._flag_n = 1
        rf._flag_c = 1
        rf.reset()
        assert rf.pack_ccr() == 0

    def test_reset_supervisor(self):
        rf = RegisterFile68k()
        rf.reset()
        assert rf._flag_s == 1

    def test_reset_int_mask(self):
        rf = RegisterFile68k()
        rf._int_mask = 0
        rf.reset()
        assert rf._int_mask == 7
