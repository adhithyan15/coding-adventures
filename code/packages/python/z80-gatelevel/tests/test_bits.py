"""Tests for bits.py — bit conversion helpers."""

from z80_gatelevel.bits import (
    add_8bit,
    add_16bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_8bit,
    invert_16bit,
)


class TestIntToBits:
    def test_zero(self):
        assert int_to_bits(0, 8) == [0] * 8

    def test_one(self):
        result = int_to_bits(1, 8)
        assert result[0] == 1
        assert all(b == 0 for b in result[1:])

    def test_five(self):
        assert int_to_bits(5, 8) == [1, 0, 1, 0, 0, 0, 0, 0]

    def test_0xff(self):
        assert int_to_bits(0xFF, 8) == [1] * 8

    def test_0x80(self):
        bits = int_to_bits(0x80, 8)
        assert bits[7] == 1
        assert all(b == 0 for b in bits[:7])

    def test_masking(self):
        # Values that overflow 8 bits are masked
        assert int_to_bits(0x1FF, 8) == int_to_bits(0xFF, 8)

    def test_16bit(self):
        bits = int_to_bits(0x0100, 16)
        assert bits[8] == 1
        assert all(b == 0 for b in bits[:8])
        assert all(b == 0 for b in bits[9:])

    def test_length(self):
        for w in (1, 4, 8, 16):
            assert len(int_to_bits(0, w)) == w


class TestBitsToInt:
    def test_zero(self):
        assert bits_to_int([0] * 8) == 0

    def test_one(self):
        assert bits_to_int([1] + [0] * 7) == 1

    def test_five(self):
        assert bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) == 5

    def test_all_ones_8bit(self):
        assert bits_to_int([1] * 8) == 255

    def test_roundtrip(self):
        for v in (0, 1, 42, 127, 128, 255):
            assert bits_to_int(int_to_bits(v, 8)) == v

    def test_roundtrip_16bit(self):
        for v in (0, 0x1234, 0xFFFF, 0x8000):
            assert bits_to_int(int_to_bits(v, 16)) == v


class TestComputeParity:
    def test_zero_even(self):
        # 0 ones → even parity
        assert compute_parity([0] * 8) == 1

    def test_one_odd(self):
        assert compute_parity([1] + [0] * 7) == 0

    def test_two_even(self):
        assert compute_parity([1, 1] + [0] * 6) == 1

    def test_all_ones_even(self):
        assert compute_parity([1] * 8) == 1

    def test_value_3_even(self):
        # 0b00000011: two 1-bits → even
        assert compute_parity(int_to_bits(3, 8)) == 1

    def test_value_1_odd(self):
        assert compute_parity(int_to_bits(1, 8)) == 0

    def test_empty_even(self):
        assert compute_parity([]) == 1


class TestComputeZero:
    def test_all_zero(self):
        assert compute_zero([0] * 8) == 1

    def test_one_set(self):
        assert compute_zero([1] + [0] * 7) == 0

    def test_msb_set(self):
        assert compute_zero([0] * 7 + [1]) == 0

    def test_all_ones(self):
        assert compute_zero([1] * 8) == 0

    def test_16bit_zero(self):
        assert compute_zero([0] * 16) == 1


class TestAdd8Bit:
    def test_simple(self):
        result, cout, hc = add_8bit(5, 3)
        assert result == 8
        assert cout == 0

    def test_no_overflow(self):
        result, cout, hc = add_8bit(0x7F, 0x01)
        assert result == 0x80
        assert cout == 0

    def test_overflow_carry(self):
        result, cout, hc = add_8bit(0xFF, 0x01)
        assert result == 0
        assert cout == 1

    def test_half_carry(self):
        # 0x0F + 0x01 = 0x10: carry from bit 3 to bit 4
        result, cout, hc = add_8bit(0x0F, 0x01)
        assert result == 0x10
        assert hc == 1
        assert cout == 0

    def test_with_carry_in(self):
        result, cout, hc = add_8bit(5, 3, carry_in=1)
        assert result == 9

    def test_zero_plus_zero(self):
        result, cout, hc = add_8bit(0, 0)
        assert result == 0
        assert cout == 0
        assert hc == 0


class TestAdd16Bit:
    def test_simple(self):
        result, cout, hc = add_16bit(0x1234, 0x0001)
        assert result == 0x1235
        assert cout == 0

    def test_overflow(self):
        result, cout, hc = add_16bit(0xFFFF, 0x0001)
        assert result == 0
        assert cout == 1

    def test_half_carry_16(self):
        # Carry from bit 11 to bit 12
        result, cout, hc = add_16bit(0x0FFF, 0x0001)
        assert result == 0x1000
        assert hc == 1

    def test_no_half_carry_16(self):
        result, cout, hc = add_16bit(0x0100, 0x0100)
        assert result == 0x0200
        assert hc == 0


class TestInvert8Bit:
    def test_zero(self):
        assert invert_8bit(0) == 255

    def test_0xff(self):
        assert invert_8bit(0xFF) == 0

    def test_0xaa(self):
        assert invert_8bit(0xAA) == 0x55

    def test_0x55(self):
        assert invert_8bit(0x55) == 0xAA

    def test_roundtrip(self):
        for v in range(256):
            assert invert_8bit(invert_8bit(v)) == v


class TestInvert16Bit:
    def test_zero(self):
        assert invert_16bit(0) == 0xFFFF

    def test_0xffff(self):
        assert invert_16bit(0xFFFF) == 0

    def test_0x1234(self):
        assert invert_16bit(0x1234) == 0xEDCB
