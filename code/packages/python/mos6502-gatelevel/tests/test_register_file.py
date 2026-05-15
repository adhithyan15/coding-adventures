"""Tests for mos6502_gatelevel.register_file."""

from __future__ import annotations

import pytest

from mos6502_gatelevel.register_file import (
    FlagRegister,
    Register8,
    Register16,
    RegisterFile6502,
)


# ── Register8 ─────────────────────────────────────────────────────────────────

class TestRegister8:
    def test_initial_zero(self):
        r = Register8()
        assert r.read() == 0

    def test_write_and_read(self):
        r = Register8()
        r.write(0xAB)
        assert r.read() == 0xAB

    def test_write_masks_to_8bit(self):
        r = Register8()
        r.write(0x1FF)   # 9 bits — should mask to 0xFF
        assert r.read() == 0xFF

    def test_overwrite(self):
        r = Register8()
        r.write(0x11)
        r.write(0x22)
        assert r.read() == 0x22

    def test_write_zero(self):
        r = Register8()
        r.write(0xAB)
        r.write(0)
        assert r.read() == 0

    def test_read_bits_lsb_first(self):
        r = Register8()
        r.write(1)
        bits = r.read_bits()
        assert bits[0] == 1   # LSB
        assert bits[7] == 0   # MSB
        assert len(bits) == 8

    def test_read_bits_msb_set(self):
        r = Register8()
        r.write(0x80)
        bits = r.read_bits()
        assert bits[7] == 1
        assert bits[0] == 0

    def test_all_values(self):
        r = Register8()
        for v in range(256):
            r.write(v)
            assert r.read() == v


# ── Register16 ────────────────────────────────────────────────────────────────

class TestRegister16:
    def test_initial_zero(self):
        r = Register16()
        assert r.read() == 0

    def test_write_and_read(self):
        r = Register16()
        r.write(0x1234)
        assert r.read() == 0x1234

    def test_write_masks_to_16bit(self):
        r = Register16()
        r.write(0x10000)   # 17 bits — masks to 0
        assert r.read() == 0

    def test_inc_basic(self):
        r = Register16()
        r.write(0x1234)
        r.inc(1)
        assert r.read() == 0x1235

    def test_inc_wraps(self):
        r = Register16()
        r.write(0xFFFF)
        r.inc(1)
        assert r.read() == 0

    def test_inc_by_2(self):
        r = Register16()
        r.write(0x0100)
        r.inc(2)
        assert r.read() == 0x0102

    def test_write_0x8000(self):
        r = Register16()
        r.write(0x8000)
        assert r.read() == 0x8000

    def test_write_0xFFFF(self):
        r = Register16()
        r.write(0xFFFF)
        assert r.read() == 0xFFFF

    def test_inc_from_zero(self):
        r = Register16()
        for expected in range(1, 10):
            r.inc(1)
            assert r.read() == expected


# ── FlagRegister ─────────────────────────────────────────────────────────────

class TestFlagRegister:
    def test_initial_state(self):
        f = FlagRegister()
        assert f.get_n() == 0
        assert f.get_v() == 0
        assert f.get_b() == 0
        assert f.get_d() == 0
        assert f.get_i() == 1   # I=1 at power-on
        assert f.get_z() == 0
        assert f.get_c() == 0

    def test_set_and_get_n(self):
        f = FlagRegister()
        f.set_n(1)
        assert f.get_n() == 1
        f.set_n(0)
        assert f.get_n() == 0

    def test_set_and_get_v(self):
        f = FlagRegister()
        f.set_v(1)
        assert f.get_v() == 1

    def test_set_and_get_all_flags(self):
        f = FlagRegister()
        for setter, getter in [
            (f.set_n, f.get_n), (f.set_v, f.get_v),
            (f.set_b, f.get_b), (f.set_d, f.get_d),
            (f.set_i, f.get_i), (f.set_z, f.get_z),
            (f.set_c, f.get_c),
        ]:
            setter(1)
            assert getter() == 1
            setter(0)
            assert getter() == 0

    def test_pack_initial(self):
        f = FlagRegister()
        # I=1, bit5=1: P = 0x24
        assert f.pack() == 0x24

    def test_pack_all_flags(self):
        f = FlagRegister()
        f.set_n(1); f.set_v(1); f.set_b(1); f.set_d(1)
        f.set_i(1); f.set_z(1); f.set_c(1)
        p = f.pack()
        assert p == 0xFF

    def test_pack_bit5_always_1(self):
        f = FlagRegister()
        # Even with all flags 0, bit5 must be 1
        f.set_i(0)
        p = f.pack()
        assert p & 0x20 == 0x20

    def test_pack_with_b_override(self):
        f = FlagRegister()
        f.set_b(0)
        p = f.pack(with_b=1)
        assert p & 0x10 == 0x10   # B bit forced to 1

    def test_pack_with_b_zero_override(self):
        f = FlagRegister()
        f.set_b(1)
        p = f.pack(with_b=0)
        assert p & 0x10 == 0   # B bit forced to 0

    def test_unpack(self):
        f = FlagRegister()
        # P = 0xFF: all flags set
        f.unpack(0xFF)
        assert f.get_n() == 1
        assert f.get_v() == 1
        assert f.get_b() == 1
        assert f.get_d() == 1
        assert f.get_i() == 1
        assert f.get_z() == 1
        assert f.get_c() == 1

    def test_unpack_zero(self):
        f = FlagRegister()
        f.unpack(0x00)
        assert f.get_n() == 0
        assert f.get_v() == 0
        assert f.get_b() == 0
        assert f.get_d() == 0
        assert f.get_i() == 0
        assert f.get_z() == 0
        assert f.get_c() == 0

    def test_pack_unpack_roundtrip(self):
        f = FlagRegister()
        for p_val in range(256):
            f.unpack(p_val)
            packed = f.pack()
            # Bit 5 always 1; all other bits should match
            assert (packed & ~0x20) == (p_val & ~0x20)
            assert (packed & 0x20) == 0x20

    def test_unpack_0x24(self):
        # Power-on reset state
        f = FlagRegister()
        f.unpack(0x24)
        assert f.get_i() == 1
        assert f.get_n() == 0
        assert f.get_c() == 0


# ── RegisterFile6502 ─────────────────────────────────────────────────────────

class TestRegisterFile6502:
    def test_initial_registers(self):
        rf = RegisterFile6502()
        assert rf.a.read() == 0
        assert rf.x.read() == 0
        assert rf.y.read() == 0
        assert rf.s.read() == 0xFD   # Power-on stack pointer
        assert rf.pc.read() == 0

    def test_all_registers_accessible(self):
        rf = RegisterFile6502()
        rf.a.write(0x01)
        rf.x.write(0x02)
        rf.y.write(0x03)
        rf.s.write(0xFD)
        rf.pc.write(0x1000)
        assert rf.a.read() == 0x01
        assert rf.x.read() == 0x02
        assert rf.y.read() == 0x03
        assert rf.s.read() == 0xFD
        assert rf.pc.read() == 0x1000

    def test_reset(self):
        rf = RegisterFile6502()
        rf.a.write(0xFF)
        rf.x.write(0xFF)
        rf.y.write(0xFF)
        rf.pc.write(0xBEEF)
        rf.flags.set_n(1)
        rf.flags.set_c(1)
        rf.flags.set_d(1)

        rf.reset()

        assert rf.a.read() == 0
        assert rf.x.read() == 0
        assert rf.y.read() == 0
        assert rf.s.read() == 0xFD
        assert rf.pc.read() == 0
        assert rf.flags.get_n() == 0
        assert rf.flags.get_c() == 0
        assert rf.flags.get_d() == 0
        assert rf.flags.get_i() == 1   # I=1 after reset

    def test_flags_initial_i(self):
        rf = RegisterFile6502()
        assert rf.flags.get_i() == 1

    def test_pc_increment(self):
        rf = RegisterFile6502()
        rf.pc.write(0x0200)
        rf.pc.inc(1)
        assert rf.pc.read() == 0x0201

    def test_independent_registers(self):
        # Changing one register should not affect others
        rf = RegisterFile6502()
        rf.a.write(0xAA)
        rf.x.write(0x55)
        assert rf.a.read() == 0xAA
        assert rf.x.read() == 0x55
