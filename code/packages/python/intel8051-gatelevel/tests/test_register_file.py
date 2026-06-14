"""Tests for intel8051_gatelevel.register_file — flip-flop IRAM and PC."""

from intel8051_gatelevel.register_file import RegisterFile8051


class TestIRAMReadWrite:
    def setup_method(self):
        self.rf = RegisterFile8051()

    def test_initial_zero(self):
        for addr in range(256):
            assert self.rf.read_iram8(addr) == 0

    def test_write_read_roundtrip(self):
        self.rf.write_iram8(0x10, 0x42)
        assert self.rf.read_iram8(0x10) == 0x42

    def test_all_addresses(self):
        for addr in range(256):
            self.rf.write_iram8(addr, (addr * 7) & 0xFF)
        for addr in range(256):
            assert self.rf.read_iram8(addr) == (addr * 7) & 0xFF

    def test_max_value(self):
        self.rf.write_iram8(0x00, 0xFF)
        assert self.rf.read_iram8(0x00) == 0xFF

    def test_mask_high_bits(self):
        # Values > 255 should be masked
        self.rf.write_iram8(0x00, 0x1FF)  # should store 0xFF
        assert self.rf.read_iram8(0x00) == 0xFF

    def test_addr_mask(self):
        # Address should wrap to 0-255
        self.rf.write_iram8(0x100, 0x42)  # wraps to 0x00
        assert self.rf.read_iram8(0x00) == 0x42

    def test_sfr_region(self):
        self.rf.write_iram8(0xE0, 0xAB)  # ACC
        assert self.rf.read_iram8(0xE0) == 0xAB

    def test_independence(self):
        self.rf.write_iram8(0x10, 0x11)
        self.rf.write_iram8(0x11, 0x22)
        assert self.rf.read_iram8(0x10) == 0x11
        assert self.rf.read_iram8(0x11) == 0x22


class TestPC:
    def setup_method(self):
        self.rf = RegisterFile8051()

    def test_initial_zero(self):
        assert self.rf.read_pc() == 0

    def test_write_read(self):
        self.rf.write_pc(0x1234)
        assert self.rf.read_pc() == 0x1234

    def test_max_value(self):
        self.rf.write_pc(0xFFFF)
        assert self.rf.read_pc() == 0xFFFF

    def test_mask(self):
        self.rf.write_pc(0x10000)  # wraps to 0
        assert self.rf.read_pc() == 0

    def test_increment_by_one(self):
        self.rf.write_pc(0x1000)
        self.rf.increment_pc(1)
        assert self.rf.read_pc() == 0x1001

    def test_increment_by_two(self):
        self.rf.write_pc(0x1000)
        self.rf.increment_pc(2)
        assert self.rf.read_pc() == 0x1002

    def test_increment_wraparound(self):
        self.rf.write_pc(0xFFFF)
        self.rf.increment_pc(1)
        assert self.rf.read_pc() == 0x0000

    def test_increment_default(self):
        self.rf.write_pc(0x0100)
        self.rf.increment_pc()
        assert self.rf.read_pc() == 0x0101

    def test_sequential_increments(self):
        self.rf.write_pc(0)
        for _i in range(100):
            self.rf.increment_pc(1)
        assert self.rf.read_pc() == 100


class TestBitAddressable:
    def setup_method(self):
        self.rf = RegisterFile8051()

    def test_lower_ram_bit_zero(self):
        # Bit address 0x00 → byte 0x20, bit 0
        self.rf.write_bit(0x00, 1)
        assert self.rf.read_bit(0x00) == 1

    def test_lower_ram_bit_seven(self):
        # Bit address 0x07 → byte 0x20, bit 7
        self.rf.write_bit(0x07, 1)
        assert self.rf.read_bit(0x07) == 1
        # Should not affect bit 0x00
        assert self.rf.read_bit(0x00) == 0

    def test_lower_ram_last_bit(self):
        # Bit address 0x7F → byte 0x2F, bit 7
        self.rf.write_bit(0x7F, 1)
        assert self.rf.read_bit(0x7F) == 1

    def test_sfr_bit_psw(self):
        # PSW is at 0xD0; bit address 0xD0 = PSW bit 0 (P flag)
        self.rf.write_bit(0xD0, 1)
        assert self.rf.read_bit(0xD0) == 1
        # Check the byte changed too
        assert (self.rf.read_iram8(0xD0) & 0x01) == 1

    def test_sfr_bit_acc(self):
        # ACC is at 0xE0; bit address 0xE7 = ACC bit 7 (MSB)
        self.rf.write_bit(0xE7, 1)
        assert self.rf.read_bit(0xE7) == 1
        assert (self.rf.read_iram8(0xE0) & 0x80) == 0x80

    def test_clear_bit(self):
        self.rf.write_bit(0x10, 1)
        self.rf.write_bit(0x10, 0)
        assert self.rf.read_bit(0x10) == 0

    def test_bit_independence(self):
        # Setting bit 0x00 should not affect bit 0x01
        self.rf.write_bit(0x00, 1)
        assert self.rf.read_bit(0x01) == 0

    def test_byte_reflects_bits(self):
        # Set bits 0x20 (bit 0 of byte 0x20) and 0x21 (bit 1 of byte 0x20)
        self.rf.write_bit(0x20, 1)
        self.rf.write_bit(0x21, 1)
        byte_val = self.rf.read_iram8(0x20 + 4)  # bit 0x20 is at byte 0x24
        # bit 0x20 = byte 0x20 + (0x20>>3) = 0x20+4 = 0x24, bit 0
        # bit 0x21 = byte 0x24, bit 1
        byte_val = self.rf.read_iram8(0x24)
        assert (byte_val & 0x01) == 1  # bit 0
        assert (byte_val & 0x02) == 2  # bit 1

    def test_sfr_bit_0x80(self):
        # Bit address 0x80 → byte 0x80, bit 0 (P0.0)
        self.rf.write_bit(0x80, 1)
        assert self.rf.read_bit(0x80) == 1
        assert (self.rf.read_iram8(0x80) & 0x01) == 1


class TestBulkOperations:
    def setup_method(self):
        self.rf = RegisterFile8051()

    def test_load_and_dump(self):
        data = bytes(range(256))
        self.rf.load_iram(data)
        result = self.rf.dump_iram()
        assert bytes(result) == data

    def test_partial_load(self):
        data = bytes([0x42, 0x43, 0x44])
        self.rf.load_iram(data)
        assert self.rf.read_iram8(0) == 0x42
        assert self.rf.read_iram8(1) == 0x43
        assert self.rf.read_iram8(2) == 0x44
        assert self.rf.read_iram8(3) == 0  # unchanged

    def test_dump_default_zeros(self):
        result = self.rf.dump_iram()
        assert all(b == 0 for b in result)
        assert len(result) == 256
