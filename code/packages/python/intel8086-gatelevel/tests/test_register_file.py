"""Tests for register_file.py — Intel 8086 register file."""

import pytest

from intel8086_gatelevel.register_file import RegisterFile8086


class TestRegisterFile16:
    """Tests for 16-bit register read/write."""

    def setup_method(self):
        self.rf = RegisterFile8086()

    def test_initial_zero(self):
        for reg in ["ax", "bx", "cx", "dx", "si", "di", "sp", "bp",
                    "cs", "ds", "ss", "es", "ip"]:
            assert self.rf.read16(reg) == 0

    def test_write_read_ax(self):
        self.rf.write16("ax", 0x1234)
        assert self.rf.read16("ax") == 0x1234

    def test_write_read_bx(self):
        self.rf.write16("bx", 0xABCD)
        assert self.rf.read16("bx") == 0xABCD

    def test_write_read_cx(self):
        self.rf.write16("cx", 0x5678)
        assert self.rf.read16("cx") == 0x5678

    def test_write_read_dx(self):
        self.rf.write16("dx", 0xFFFF)
        assert self.rf.read16("dx") == 0xFFFF

    def test_write_read_si(self):
        self.rf.write16("si", 0x1000)
        assert self.rf.read16("si") == 0x1000

    def test_write_read_di(self):
        self.rf.write16("di", 0x2000)
        assert self.rf.read16("di") == 0x2000

    def test_write_read_sp(self):
        self.rf.write16("sp", 0xFFFE)
        assert self.rf.read16("sp") == 0xFFFE

    def test_write_read_bp(self):
        self.rf.write16("bp", 0x3000)
        assert self.rf.read16("bp") == 0x3000

    def test_write_read_cs(self):
        self.rf.write16("cs", 0x1000)
        assert self.rf.read16("cs") == 0x1000

    def test_write_read_ds(self):
        self.rf.write16("ds", 0x2000)
        assert self.rf.read16("ds") == 0x2000

    def test_write_read_ss(self):
        self.rf.write16("ss", 0x3000)
        assert self.rf.read16("ss") == 0x3000

    def test_write_read_es(self):
        self.rf.write16("es", 0x4000)
        assert self.rf.read16("es") == 0x4000

    def test_write_read_ip(self):
        self.rf.write16("ip", 0x0200)
        assert self.rf.read16("ip") == 0x0200

    def test_max_value(self):
        self.rf.write16("ax", 0xFFFF)
        assert self.rf.read16("ax") == 0xFFFF

    def test_zero_value(self):
        self.rf.write16("ax", 0x1234)
        self.rf.write16("ax", 0)
        assert self.rf.read16("ax") == 0

    def test_masked_to_16bit(self):
        self.rf.write16("bx", 0x10000)  # should mask to 0
        assert self.rf.read16("bx") == 0

    def test_multiple_registers_independent(self):
        self.rf.write16("ax", 0x1111)
        self.rf.write16("bx", 0x2222)
        self.rf.write16("cx", 0x3333)
        assert self.rf.read16("ax") == 0x1111
        assert self.rf.read16("bx") == 0x2222
        assert self.rf.read16("cx") == 0x3333


class TestRegisterFile8Bit:
    """Tests for 8-bit high/low byte access."""

    def setup_method(self):
        self.rf = RegisterFile8086()

    def test_read8_low_ax(self):
        self.rf.write16("ax", 0x1234)
        assert self.rf.read8_low("ax") == 0x34   # AL

    def test_read8_high_ax(self):
        self.rf.write16("ax", 0x1234)
        assert self.rf.read8_high("ax") == 0x12  # AH

    def test_read8_low_bx(self):
        self.rf.write16("bx", 0xABCD)
        assert self.rf.read8_low("bx") == 0xCD   # BL

    def test_read8_high_bx(self):
        self.rf.write16("bx", 0xABCD)
        assert self.rf.read8_high("bx") == 0xAB  # BH

    def test_read8_low_cx(self):
        self.rf.write16("cx", 0x5678)
        assert self.rf.read8_low("cx") == 0x78

    def test_read8_high_cx(self):
        self.rf.write16("cx", 0x5678)
        assert self.rf.read8_high("cx") == 0x56

    def test_read8_low_dx(self):
        self.rf.write16("dx", 0x1200)
        assert self.rf.read8_low("dx") == 0x00

    def test_read8_high_dx(self):
        self.rf.write16("dx", 0x1200)
        assert self.rf.read8_high("dx") == 0x12

    def test_write8_low_preserves_high(self):
        self.rf.write16("ax", 0x1200)
        self.rf.write8_low("ax", 0x56)
        assert self.rf.read16("ax") == 0x1256
        assert self.rf.read8_high("ax") == 0x12  # AH preserved

    def test_write8_high_preserves_low(self):
        self.rf.write16("ax", 0x0034)
        self.rf.write8_high("ax", 0x12)
        assert self.rf.read16("ax") == 0x1234
        assert self.rf.read8_low("ax") == 0x34  # AL preserved

    def test_write8_low_bx(self):
        self.rf.write16("bx", 0xAB00)
        self.rf.write8_low("bx", 0xCD)
        assert self.rf.read16("bx") == 0xABCD

    def test_write8_high_cx(self):
        self.rf.write16("cx", 0x0078)
        self.rf.write8_high("cx", 0x56)
        assert self.rf.read16("cx") == 0x5678

    def test_write8_low_masks(self):
        self.rf.write16("ax", 0x0000)
        self.rf.write8_low("ax", 0x1FF)   # mask to 0xFF
        assert self.rf.read8_low("ax") == 0xFF

    def test_write8_high_masks(self):
        self.rf.write16("ax", 0x0000)
        self.rf.write8_high("ax", 0x1FF)   # mask to 0xFF
        assert self.rf.read8_high("ax") == 0xFF

    def test_al_zero_initial(self):
        assert self.rf.read8_low("ax") == 0

    def test_ah_zero_initial(self):
        assert self.rf.read8_high("ax") == 0


class TestFlagsPackUnpack:
    """Tests for FLAGS pack/unpack."""

    def setup_method(self):
        self.rf = RegisterFile8086()

    def test_initial_flags_zero(self):
        # Bit 1 is always 1
        assert self.rf.pack_flags() == 0x0002

    def test_pack_cf(self):
        self.rf._flag_cf = 1
        flags = self.rf.pack_flags()
        assert flags & 1 == 1

    def test_pack_pf(self):
        self.rf._flag_pf = 1
        flags = self.rf.pack_flags()
        assert (flags >> 2) & 1 == 1

    def test_pack_af(self):
        self.rf._flag_af = 1
        flags = self.rf.pack_flags()
        assert (flags >> 4) & 1 == 1

    def test_pack_zf(self):
        self.rf._flag_zf = 1
        flags = self.rf.pack_flags()
        assert (flags >> 6) & 1 == 1

    def test_pack_sf(self):
        self.rf._flag_sf = 1
        flags = self.rf.pack_flags()
        assert (flags >> 7) & 1 == 1

    def test_pack_tf(self):
        self.rf._flag_tf = 1
        flags = self.rf.pack_flags()
        assert (flags >> 8) & 1 == 1

    def test_pack_if(self):
        self.rf._flag_if = 1
        flags = self.rf.pack_flags()
        assert (flags >> 9) & 1 == 1

    def test_pack_df(self):
        self.rf._flag_df = 1
        flags = self.rf.pack_flags()
        assert (flags >> 10) & 1 == 1

    def test_pack_of(self):
        self.rf._flag_of = 1
        flags = self.rf.pack_flags()
        assert (flags >> 11) & 1 == 1

    def test_bit1_always_set(self):
        self.rf._flag_cf = 0
        flags = self.rf.pack_flags()
        assert (flags >> 1) & 1 == 1

    def test_zf_only(self):
        self.rf._flag_zf = 1
        flags = self.rf.pack_flags()
        assert flags == 0x0042  # bit1=1 (0x02) | zf=1 (0x40)

    def test_unpack_cf(self):
        self.rf.unpack_flags(0x0001)
        assert self.rf._flag_cf == 1

    def test_unpack_pf(self):
        self.rf.unpack_flags(0x0004)
        assert self.rf._flag_pf == 1

    def test_unpack_af(self):
        self.rf.unpack_flags(0x0010)
        assert self.rf._flag_af == 1

    def test_unpack_zf(self):
        self.rf.unpack_flags(0x0040)
        assert self.rf._flag_zf == 1

    def test_unpack_sf(self):
        self.rf.unpack_flags(0x0080)
        assert self.rf._flag_sf == 1

    def test_unpack_tf(self):
        self.rf.unpack_flags(0x0100)
        assert self.rf._flag_tf == 1

    def test_unpack_if(self):
        self.rf.unpack_flags(0x0200)
        assert self.rf._flag_if == 1

    def test_unpack_df(self):
        self.rf.unpack_flags(0x0400)
        assert self.rf._flag_df == 1

    def test_unpack_of(self):
        self.rf.unpack_flags(0x0800)
        assert self.rf._flag_of == 1

    def test_roundtrip_flags(self):
        self.rf._flag_cf = 1; self.rf._flag_zf = 1; self.rf._flag_of = 1
        flags = self.rf.pack_flags()
        rf2 = RegisterFile8086()
        rf2.unpack_flags(flags)
        assert rf2._flag_cf == 1
        assert rf2._flag_zf == 1
        assert rf2._flag_of == 1

    def test_unpack_clears_flags(self):
        self.rf._flag_cf = 1; self.rf._flag_zf = 1
        self.rf.unpack_flags(0x0000)
        assert self.rf._flag_cf == 0
        assert self.rf._flag_zf == 0


class TestPhysicalAddress:
    """Tests for physical_address()."""

    def setup_method(self):
        self.rf = RegisterFile8086()

    def test_zero_zero(self):
        self.rf.write16("cs", 0)
        assert self.rf.physical_address("cs", 0) == 0

    def test_segment_shift(self):
        self.rf.write16("cs", 0x1000)
        # CS=0x1000 → CS<<4 = 0x10000; IP=0 → physical=0x10000
        assert self.rf.physical_address("cs", 0) == 0x10000

    def test_segment_plus_offset(self):
        self.rf.write16("cs", 0x1000)
        assert self.rf.physical_address("cs", 0x0100) == 0x10100

    def test_ds_segment(self):
        self.rf.write16("ds", 0x2000)
        assert self.rf.physical_address("ds", 0x0050) == 0x20050

    def test_ss_segment(self):
        self.rf.write16("ss", 0x3000)
        assert self.rf.physical_address("ss", 0) == 0x30000

    def test_es_segment(self):
        self.rf.write16("es", 0x4000)
        assert self.rf.physical_address("es", 0x10) == 0x40010

    def test_max_segment(self):
        self.rf.write16("cs", 0xFFFF)
        # CS=0xFFFF → CS<<4 = 0xFFFF0; IP=0xF → physical=0xFFFFF
        assert self.rf.physical_address("cs", 0x000F) == 0xFFFFF

    def test_wrap_20bit(self):
        self.rf.write16("cs", 0xFFFF)
        # CS<<4=0xFFFF0; offset=0x10 → 0x100000 → masked to 0x00000
        result = self.rf.physical_address("cs", 0x10)
        assert result == (0xFFFF0 + 0x10) & 0xFFFFF

    def test_segment_0_offset_nonzero(self):
        self.rf.write16("ds", 0)
        assert self.rf.physical_address("ds", 0x1234) == 0x1234
