"""test_register_file.py — Tests for the AArch64 gate-level register file.

Covers:
  - XZR (index 31): reads always return 0, writes are discarded
  - X-register (64-bit) read/write
  - W-register (32-bit) read/write — writes zero-extend to 64 bits
  - SP access (separate from XZR)
  - reset() zeroes all registers
  - get_gprs_tuple() snapshot
"""

import pytest
from aarch64_gatelevel.register_file import RegisterFile


def test_initial_all_zero():
    rf = RegisterFile()
    for i in range(32):
        assert rf.read(i, sf=1) == 0


def test_write_read_64bit():
    rf = RegisterFile()
    rf.write_int(5, 0xDEADBEEF, sf=1)
    assert rf.read(5, sf=1) == 0xDEADBEEF


def test_write_read_32bit_zero_extends():
    rf = RegisterFile()
    # Write 32-bit value to W register; X register should be zero-extended
    rf.write_int(3, 0xFFFFFFFF, sf=0)
    assert rf.read(3, sf=1) == 0xFFFFFFFF  # upper 32 bits should be 0


def test_write_64bit_read_32bit():
    rf = RegisterFile()
    rf.write_int(7, 0x1234_5678_DEAD_BEEF, sf=1)
    # Reading as W-register gives only the low 32 bits
    assert rf.read(7, sf=0) == 0xDEAD_BEEF


def test_write_32bit_clears_upper():
    rf = RegisterFile()
    # First set a 64-bit value
    rf.write_int(10, 0xFFFFFFFF_DEADBEEF, sf=1)
    assert rf.read(10, sf=1) == 0xFFFFFFFF_DEADBEEF
    # Then write 32-bit — upper 32 bits must be cleared
    rf.write_int(10, 0x1234ABCD, sf=0)
    assert rf.read(10, sf=1) == 0x1234ABCD   # NOT 0xFFFFFFFF_1234ABCD


def test_xzr_reads_zero():
    rf = RegisterFile()
    assert rf.read(31, sf=1) == 0
    assert rf.read(31, sf=0) == 0


def test_xzr_write_discarded():
    rf = RegisterFile()
    rf.write_int(31, 0xDEAD, sf=1)
    assert rf.read(31, sf=1) == 0


def test_xzr_bits_read():
    rf = RegisterFile()
    bits = rf.read_bits(31, sf=1)
    assert all(b == 0 for b in bits)
    assert len(bits) == 64


def test_xzr_bits_32_read():
    rf = RegisterFile()
    bits = rf.read_bits(31, sf=0)
    assert all(b == 0 for b in bits)
    assert len(bits) == 32


def test_sp_initial_zero():
    rf = RegisterFile()
    assert rf.read_sp() == 0


def test_sp_write_read():
    rf = RegisterFile()
    rf.write_sp_int(0xDEAD_BEEF_1234_5678)
    assert rf.read_sp() == 0xDEAD_BEEF_1234_5678


def test_sp_independent_from_xzr():
    rf = RegisterFile()
    rf.write_sp_int(0xCAFE)
    # XZR (index 31) should still be 0
    assert rf.read(31, sf=1) == 0
    # SP should be 0xCAFE
    assert rf.read_sp() == 0xCAFE


def test_write_bits_64():
    from aarch64_gatelevel.bits import int_to_bits, bits_to_int
    rf = RegisterFile()
    bits = int_to_bits(0xABCDEF, 64)
    rf.write(5, bits, sf=1)
    assert rf.read(5, sf=1) == 0xABCDEF


def test_write_bits_32_zero_extends():
    from aarch64_gatelevel.bits import int_to_bits, bits_to_int
    rf = RegisterFile()
    bits32 = int_to_bits(0xDEADBEEF, 32)
    rf.write(3, bits32, sf=0)
    assert rf.read(3, sf=1) == 0xDEADBEEF   # zero-extended
    assert rf.read(3, sf=0) == 0xDEADBEEF


def test_read_bits_64():
    from aarch64_gatelevel.bits import int_to_bits, bits_to_int
    rf = RegisterFile()
    rf.write_int(2, 0x1234, sf=1)
    bits = rf.read_bits(2, sf=1)
    assert len(bits) == 64
    assert bits_to_int(bits) == 0x1234


def test_read_bits_32():
    from aarch64_gatelevel.bits import int_to_bits, bits_to_int
    rf = RegisterFile()
    rf.write_int(2, 0xFFFFFFFF_12345678, sf=1)
    bits = rf.read_bits(2, sf=0)
    assert len(bits) == 32
    assert bits_to_int(bits) == 0x12345678


def test_get_gprs_tuple():
    rf = RegisterFile()
    rf.write_int(0, 42, sf=1)
    rf.write_int(1, 100, sf=1)
    rf.write_int(31, 999, sf=1)  # write to XZR — discarded
    t = rf.get_gprs_tuple()
    assert len(t) == 32
    assert t[0] == 42
    assert t[1] == 100
    assert t[31] == 0   # XZR always 0


def test_reset():
    rf = RegisterFile()
    rf.write_int(5, 0xDEAD, sf=1)
    rf.write_sp_int(0xBEEF)
    rf.reset()
    assert rf.read(5, sf=1) == 0
    assert rf.read_sp() == 0


def test_all_registers_independent():
    rf = RegisterFile()
    # Write different values to all 31 real registers
    for i in range(31):
        rf.write_int(i, i * 1000 + 1, sf=1)
    for i in range(31):
        assert rf.read(i, sf=1) == i * 1000 + 1


def test_sp_write_bits():
    from aarch64_gatelevel.bits import int_to_bits, bits_to_int
    rf = RegisterFile()
    bits = int_to_bits(0xABCD, 64)
    rf.write_sp(bits)
    assert rf.read_sp() == 0xABCD
    sp_bits = rf.read_sp_bits()
    assert bits_to_int(sp_bits) == 0xABCD
