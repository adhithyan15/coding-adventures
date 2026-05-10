"""Tests for Register8, Register16, and RegisterFile."""

from __future__ import annotations

import pytest

from intel8080_gatelevel.register_file import (
    PAIR_BC,
    PAIR_DE,
    PAIR_HL,
    PAIR_SP,
    REG_A,
    REG_B,
    REG_C,
    REG_D,
    REG_E,
    REG_H,
    REG_L,
    REG_M,
    Register8,
    Register16,
    RegisterFile,
)


class TestRegister8:
    def test_initial_zero(self) -> None:
        r = Register8()
        assert r.read() == 0

    def test_write_read(self) -> None:
        r = Register8()
        r.write(0xAB)
        assert r.read() == 0xAB

    def test_multiple_writes(self) -> None:
        r = Register8()
        r.write(10)
        r.write(20)
        assert r.read() == 20

    def test_write_masked(self) -> None:
        r = Register8()
        r.write(0x1FF)   # should mask to 0xFF
        assert r.read() == 0xFF

    def test_read_bits_length(self) -> None:
        r = Register8()
        r.write(0xAA)
        bits = r.read_bits()
        assert len(bits) == 8

    def test_read_bits_value(self) -> None:
        r = Register8()
        r.write(0x01)
        bits = r.read_bits()
        assert bits[0] == 1  # LSB
        assert all(b == 0 for b in bits[1:])


class TestRegister16:
    def test_initial_zero(self) -> None:
        r = Register16()
        assert r.read() == 0

    def test_write_read(self) -> None:
        r = Register16()
        r.write(0x1234)
        assert r.read() == 0x1234

    def test_inc(self) -> None:
        r = Register16()
        r.write(0x1234)
        r.inc()
        assert r.read() == 0x1235

    def test_inc_by_2(self) -> None:
        r = Register16()
        r.write(0x1234)
        r.inc(2)
        assert r.read() == 0x1236

    def test_inc_by_3(self) -> None:
        r = Register16()
        r.write(0x1234)
        r.inc(3)
        assert r.read() == 0x1237

    def test_inc_wraps(self) -> None:
        r = Register16()
        r.write(0xFFFF)
        r.inc()
        assert r.read() == 0

    def test_dec(self) -> None:
        r = Register16()
        r.write(0x1234)
        r.dec()
        assert r.read() == 0x1233

    def test_dec_by_2(self) -> None:
        r = Register16()
        r.write(0x0200)
        r.dec(2)
        assert r.read() == 0x01FE

    def test_dec_wraps(self) -> None:
        r = Register16()
        r.write(0)
        r.dec()
        assert r.read() == 0xFFFF


class TestRegisterFile:
    def test_initial_all_zero(self) -> None:
        rf = RegisterFile()
        for code in [REG_B, REG_C, REG_D, REG_E, REG_H, REG_L, REG_A]:
            assert rf.read(code) == 0

    def test_write_read_a(self) -> None:
        rf = RegisterFile()
        rf.write(REG_A, 0x42)
        assert rf.read(REG_A) == 0x42

    def test_write_read_all(self) -> None:
        rf = RegisterFile()
        values = {
            REG_B: 1, REG_C: 2, REG_D: 3,
            REG_E: 4, REG_H: 5, REG_L: 6, REG_A: 7,
        }
        for code, val in values.items():
            rf.write(code, val)
        for code, val in values.items():
            assert rf.read(code) == val

    def test_m_raises_on_read(self) -> None:
        rf = RegisterFile()
        with pytest.raises(ValueError):
            rf.read(REG_M)

    def test_m_raises_on_write(self) -> None:
        rf = RegisterFile()
        with pytest.raises(ValueError):
            rf.write(REG_M, 0)

    def test_read_pair_bc(self) -> None:
        rf = RegisterFile()
        rf.write(REG_B, 0x12)
        rf.write(REG_C, 0x34)
        assert rf.read_pair(PAIR_BC) == 0x1234

    def test_read_pair_de(self) -> None:
        rf = RegisterFile()
        rf.write(REG_D, 0xAB)
        rf.write(REG_E, 0xCD)
        assert rf.read_pair(PAIR_DE) == 0xABCD

    def test_read_pair_hl(self) -> None:
        rf = RegisterFile()
        rf.write(REG_H, 0x02)
        rf.write(REG_L, 0x00)
        assert rf.read_pair(PAIR_HL) == 0x0200

    def test_read_pair_sp(self) -> None:
        rf = RegisterFile()
        sp = Register16()
        sp.write(0x1000)
        assert rf.read_pair(PAIR_SP, sp) == 0x1000

    def test_read_pair_sp_requires_sp(self) -> None:
        rf = RegisterFile()
        with pytest.raises(ValueError):
            rf.read_pair(PAIR_SP)

    def test_write_pair_bc(self) -> None:
        rf = RegisterFile()
        rf.write_pair(PAIR_BC, 0x5678)
        assert rf.read(REG_B) == 0x56
        assert rf.read(REG_C) == 0x78

    def test_write_pair_hl(self) -> None:
        rf = RegisterFile()
        rf.write_pair(PAIR_HL, 0x0200)
        assert rf.read(REG_H) == 0x02
        assert rf.read(REG_L) == 0x00

    def test_write_pair_sp(self) -> None:
        rf = RegisterFile()
        sp = Register16()
        rf.write_pair(PAIR_SP, 0x2000, sp)
        assert sp.read() == 0x2000

    def test_invalid_pair_read(self) -> None:
        rf = RegisterFile()
        with pytest.raises(ValueError):
            rf.read_pair(4)

    def test_invalid_pair_write(self) -> None:
        rf = RegisterFile()
        with pytest.raises(ValueError):
            rf.write_pair(4, 0)

    def test_read_bits(self) -> None:
        rf = RegisterFile()
        rf.write(REG_A, 0x01)
        bits = rf.read_bits(REG_A)
        assert bits[0] == 1
        assert all(b == 0 for b in bits[1:])
