"""Tests for register_file.py — Z80 register file."""

import pytest

from z80_gatelevel.register_file import (
    PAIR_BC,
    PAIR_DE,
    PAIR_HL,
    PAIR_IX,
    PAIR_IY,
    PAIR_SP,
    REG_A,
    REG_B,
    REG_C,
    REG_D,
    REG_E,
    REG_H,
    REG_L,
    REG_MEM,
    Register8,
    Register16,
    RegisterFile,
    pack_f,
    unpack_f,
)


class TestRegister8:
    def test_init_zero(self):
        r = Register8()
        assert r.read() == 0

    def test_write_read(self):
        r = Register8()
        r.write(42)
        assert r.read() == 42

    def test_write_0xff(self):
        r = Register8()
        r.write(0xFF)
        assert r.read() == 0xFF

    def test_masking(self):
        r = Register8()
        r.write(0x1FF)  # overflow: should be masked to 0xFF
        assert r.read() == 0xFF

    def test_read_bits(self):
        r = Register8()
        r.write(5)
        bits = r.read_bits()
        assert len(bits) == 8
        assert bits[0] == 1
        assert bits[2] == 1
        assert bits[1] == 0


class TestRegister16:
    def test_init_zero(self):
        r = Register16()
        assert r.read() == 0

    def test_write_read(self):
        r = Register16()
        r.write(0x1234)
        assert r.read() == 0x1234

    def test_inc(self):
        r = Register16()
        r.write(100)
        r.inc()
        assert r.read() == 101

    def test_inc_wrap(self):
        r = Register16()
        r.write(0xFFFF)
        r.inc()
        assert r.read() == 0

    def test_dec(self):
        r = Register16()
        r.write(100)
        r.dec()
        assert r.read() == 99

    def test_dec_wrap(self):
        r = Register16()
        r.write(0)
        r.dec()
        assert r.read() == 0xFFFF

    def test_inc_by_amount(self):
        r = Register16()
        r.write(0x1000)
        r.inc(2)
        assert r.read() == 0x1002


class TestRegisterFile:
    def test_init(self):
        rf = RegisterFile()
        for reg in (REG_A, REG_B, REG_C, REG_D, REG_E, REG_H, REG_L):
            assert rf.read8(reg) == 0

    def test_write_read_a(self):
        rf = RegisterFile()
        rf.write8(REG_A, 0x42)
        assert rf.read8(REG_A) == 0x42

    def test_write_read_all(self):
        rf = RegisterFile()
        values = {REG_A: 10, REG_B: 20, REG_C: 30, REG_D: 40,
                  REG_E: 50, REG_H: 60, REG_L: 70}
        for reg, val in values.items():
            rf.write8(reg, val)
        for reg, val in values.items():
            assert rf.read8(reg) == val

    def test_mem_pseudo_reg_raises(self):
        rf = RegisterFile()
        with pytest.raises(ValueError):
            rf.read8(REG_MEM)
        with pytest.raises(ValueError):
            rf.write8(REG_MEM, 0)

    def test_pair_bc(self):
        rf = RegisterFile()
        rf.write8(REG_B, 0x12)
        rf.write8(REG_C, 0x34)
        assert rf.read16_pair(PAIR_BC) == 0x1234

    def test_pair_de(self):
        rf = RegisterFile()
        rf.write8(REG_D, 0xAB)
        rf.write8(REG_E, 0xCD)
        assert rf.read16_pair(PAIR_DE) == 0xABCD

    def test_pair_hl(self):
        rf = RegisterFile()
        rf.write8(REG_H, 0xFF)
        rf.write8(REG_L, 0x00)
        assert rf.read16_pair(PAIR_HL) == 0xFF00

    def test_write_pair_bc(self):
        rf = RegisterFile()
        rf.write16_pair(PAIR_BC, 0x5678)
        assert rf.read8(REG_B) == 0x56
        assert rf.read8(REG_C) == 0x78

    def test_write_pair_sp(self):
        rf = RegisterFile()
        sp = Register16()
        rf.write16_pair(PAIR_SP, 0xFFFE, sp)
        assert sp.read() == 0xFFFE

    def test_read_pair_sp_requires_sp(self):
        rf = RegisterFile()
        with pytest.raises(ValueError):
            rf.read16_pair(PAIR_SP)  # no sp argument

    def test_ix_iy(self):
        rf = RegisterFile()
        rf.write_ix(0x1234)
        assert rf.read_ix() == 0x1234
        rf.write_iy(0x5678)
        assert rf.read_iy() == 0x5678

    def test_read_pair_ix(self):
        rf = RegisterFile()
        rf.write_ix(0xABCD)
        assert rf.read16_pair(PAIR_IX) == 0xABCD

    def test_read_pair_iy(self):
        rf = RegisterFile()
        rf.write_iy(0x1111)
        assert rf.read16_pair(PAIR_IY) == 0x1111

    def test_invalid_pair(self):
        rf = RegisterFile()
        with pytest.raises(ValueError):
            rf.read16_pair(99)
        with pytest.raises(ValueError):
            rf.write16_pair(99, 0)

    def test_exchange_af(self):
        rf = RegisterFile()
        rf.write8(REG_A, 0x42)
        rf.write_flags(1, 0, 0, 0, 0, 1)  # S=1, C=1
        rf.write_alt8(REG_A, 0x99)
        rf.write_f_prime(0x00)
        rf.exchange_af()
        assert rf.read8(REG_A) == 0x99
        assert rf.read_f_prime() == pack_f(1, 0, 0, 0, 0, 1)

    def test_exchange_bank(self):
        rf = RegisterFile()
        rf.write8(REG_B, 0x11)
        rf.write8(REG_C, 0x22)
        rf.write_alt8(REG_B, 0xAA)
        rf.write_alt8(REG_C, 0xBB)
        rf.exchange_bank()
        assert rf.read8(REG_B) == 0xAA
        assert rf.read8(REG_C) == 0xBB
        assert rf.read_alt8(REG_B) == 0x11
        assert rf.read_alt8(REG_C) == 0x22

    def test_flags_read_write(self):
        rf = RegisterFile()
        rf.write_flags(1, 0, 1, 0, 1, 0)  # S=1, Z=0, H=1, PV=0, N=1, C=0
        flags = rf.read_flags()
        assert flags['s'] == 1
        assert flags['z'] == 0
        assert flags['h'] == 1
        assert flags['pv'] == 0
        assert flags['n'] == 1
        assert flags['c'] == 0


class TestPackUnpackF:
    def test_pack_all_zero(self):
        assert pack_f(0, 0, 0, 0, 0, 0) == 0

    def test_pack_all_ones(self):
        f = pack_f(1, 1, 1, 1, 1, 1)
        assert f == 0b11010111  # S Z 0 H 0 PV N C

    def test_pack_s(self):
        assert pack_f(1, 0, 0, 0, 0, 0) == 0x80

    def test_pack_z(self):
        assert pack_f(0, 1, 0, 0, 0, 0) == 0x40

    def test_pack_c(self):
        assert pack_f(0, 0, 0, 0, 0, 1) == 0x01

    def test_unpack_roundtrip(self):
        for flags in [(1,0,0,0,0,0), (0,1,0,0,0,0), (0,0,1,0,0,0),
                      (0,0,0,1,0,0), (0,0,0,0,1,0), (0,0,0,0,0,1),
                      (1,1,1,1,1,1), (0,0,0,0,0,0)]:
            f = pack_f(*flags)
            s, z, h, pv, n, c = unpack_f(f)
            assert (s, z, h, pv, n, c) == flags

    def test_unpack_0xff(self):
        s, z, h, pv, n, c = unpack_f(0xFF)
        assert s == 1
        assert z == 1
        assert h == 1
        assert pv == 1
        assert n == 1
        assert c == 1
