"""Tests for mos6502_gatelevel.bits — bit conversion and arithmetic helpers."""

from __future__ import annotations

import pytest

from mos6502_gatelevel.bits import (
    add_16bit,
    add_8bit,
    bits_to_int,
    compute_zero,
    int_to_bits,
    invert_8bit,
)


# ── int_to_bits ───────────────────────────────────────────────────────────────

class TestIntToBits:
    def test_zero_8bit(self):
        assert int_to_bits(0, 8) == [0] * 8

    def test_one_8bit(self):
        assert int_to_bits(1, 8) == [1, 0, 0, 0, 0, 0, 0, 0]

    def test_msb_only(self):
        assert int_to_bits(0x80, 8) == [0, 0, 0, 0, 0, 0, 0, 1]

    def test_all_ones_8bit(self):
        assert int_to_bits(0xFF, 8) == [1] * 8

    def test_five(self):
        result = int_to_bits(5, 8)
        assert result == [1, 0, 1, 0, 0, 0, 0, 0]

    def test_lsb_first_ordering(self):
        # 0b10110100 = 180
        result = int_to_bits(0b10110100, 8)
        assert result[0] == 0  # bit 0 = 0 (180 is even)
        assert result[7] == 1  # bit 7 = 1 (MSB set)

    def test_overflow_masked(self):
        # 0x1FF masked to 8 bits = 0xFF
        assert int_to_bits(0x1FF, 8) == [1] * 8

    def test_16bit_zero(self):
        assert int_to_bits(0, 16) == [0] * 16

    def test_16bit_0x0100(self):
        result = int_to_bits(0x0100, 16)
        assert result[8] == 1       # bit 8 is set
        assert sum(result) == 1     # only one bit set

    def test_16bit_0xFFFF(self):
        assert int_to_bits(0xFFFF, 16) == [1] * 16

    def test_16bit_0x8000(self):
        result = int_to_bits(0x8000, 16)
        assert result[15] == 1
        assert sum(result) == 1

    def test_width_1(self):
        assert int_to_bits(1, 1) == [1]
        assert int_to_bits(0, 1) == [0]

    def test_roundtrip_all_bytes(self):
        for v in range(256):
            assert bits_to_int(int_to_bits(v, 8)) == v


# ── bits_to_int ───────────────────────────────────────────────────────────────

class TestBitsToInt:
    def test_zero(self):
        assert bits_to_int([0] * 8) == 0

    def test_one(self):
        assert bits_to_int([1, 0, 0, 0, 0, 0, 0, 0]) == 1

    def test_msb_set(self):
        assert bits_to_int([0, 0, 0, 0, 0, 0, 0, 1]) == 128

    def test_all_ones(self):
        assert bits_to_int([1] * 8) == 255

    def test_five(self):
        assert bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) == 5

    def test_16bit_round_trip(self):
        for v in [0, 1, 0xFF, 0x100, 0x1234, 0xFFFF]:
            assert bits_to_int(int_to_bits(v, 16)) == v


# ── compute_zero ──────────────────────────────────────────────────────────────

class TestComputeZero:
    def test_all_zeros(self):
        assert compute_zero([0] * 8) == 1

    def test_lsb_set(self):
        assert compute_zero([1, 0, 0, 0, 0, 0, 0, 0]) == 0

    def test_msb_set(self):
        assert compute_zero([0, 0, 0, 0, 0, 0, 0, 1]) == 0

    def test_all_ones(self):
        assert compute_zero([1] * 8) == 0

    def test_one_middle_bit(self):
        bits = [0] * 8
        bits[4] = 1
        assert compute_zero(bits) == 0

    def test_empty_list(self):
        # Vacuously zero (empty)
        assert compute_zero([]) == 1

    def test_single_zero(self):
        assert compute_zero([0]) == 1

    def test_single_one(self):
        assert compute_zero([1]) == 0

    def test_16bit_zero(self):
        assert compute_zero([0] * 16) == 1

    def test_16bit_nonzero(self):
        bits = [0] * 16
        bits[15] = 1
        assert compute_zero(bits) == 0


# ── add_8bit ──────────────────────────────────────────────────────────────────

class TestAdd8bit:
    def test_zero_plus_zero(self):
        result, carry = add_8bit(0, 0, 0)
        assert result == 0
        assert carry == 0

    def test_basic_addition(self):
        result, carry = add_8bit(10, 5, 0)
        assert result == 15
        assert carry == 0

    def test_no_carry_out(self):
        result, carry = add_8bit(0x7F, 0x01, 0)
        assert result == 0x80
        assert carry == 0

    def test_carry_out(self):
        result, carry = add_8bit(0xFF, 0x01, 0)
        assert result == 0
        assert carry == 1

    def test_ff_plus_ff(self):
        result, carry = add_8bit(0xFF, 0xFF, 0)
        assert result == 0xFE
        assert carry == 1

    def test_carry_in(self):
        result, carry = add_8bit(0, 0, 1)
        assert result == 1
        assert carry == 0

    def test_carry_in_with_overflow(self):
        result, carry = add_8bit(0xFF, 0xFF, 1)
        assert result == 0xFF
        assert carry == 1

    def test_add_one_to_ff(self):
        result, carry = add_8bit(0xFF, 0x01, 0)
        assert result == 0x00
        assert carry == 1

    def test_identity(self):
        for v in [0, 1, 0x55, 0xAA, 0xFF]:
            result, carry = add_8bit(v, 0, 0)
            assert result == v
            assert carry == 0

    def test_commutativity(self):
        for a, b in [(10, 5), (0x80, 0x80), (0xFE, 0x01)]:
            r1, c1 = add_8bit(a, b, 0)
            r2, c2 = add_8bit(b, a, 0)
            assert r1 == r2
            assert c1 == c2

    def test_all_values_match_python(self):
        for a in range(0, 256, 17):
            for b in range(0, 256, 17):
                result, carry = add_8bit(a, b, 0)
                expected = (a + b) & 0xFF
                expected_carry = (a + b) > 0xFF
                assert result == expected
                assert carry == int(expected_carry)


# ── add_16bit ─────────────────────────────────────────────────────────────────

class TestAdd16bit:
    def test_zero_plus_zero(self):
        result, carry = add_16bit(0, 0, 0)
        assert result == 0
        assert carry == 0

    def test_basic_addition(self):
        result, carry = add_16bit(0x1234, 0x0001, 0)
        assert result == 0x1235
        assert carry == 0

    def test_overflow(self):
        result, carry = add_16bit(0xFFFF, 0x0001, 0)
        assert result == 0
        assert carry == 1

    def test_ffff_plus_ffff(self):
        result, carry = add_16bit(0xFFFF, 0xFFFF, 0)
        assert result == 0xFFFE
        assert carry == 1

    def test_carry_in(self):
        result, carry = add_16bit(0x0000, 0x0000, 1)
        assert result == 1
        assert carry == 0

    def test_no_carry_out(self):
        result, carry = add_16bit(0x7FFF, 0x0001, 0)
        assert result == 0x8000
        assert carry == 0

    def test_matches_python_arithmetic(self):
        for a in [0, 1, 0x1234, 0x8000, 0xFFFF]:
            for b in [0, 1, 0x0100, 0xFFF0]:
                result, carry = add_16bit(a, b, 0)
                expected = (a + b) & 0xFFFF
                expected_carry = (a + b) > 0xFFFF
                assert result == expected
                assert carry == int(expected_carry)

    def test_pc_increment(self):
        # Common use: PC += 1
        for pc in [0, 0x200, 0x8000, 0xFFFE, 0xFFFF]:
            result, carry = add_16bit(pc, 1, 0)
            assert result == (pc + 1) & 0xFFFF


# ── invert_8bit ───────────────────────────────────────────────────────────────

class TestInvert8bit:
    def test_zero(self):
        assert invert_8bit(0) == 0xFF

    def test_ff(self):
        assert invert_8bit(0xFF) == 0

    def test_alternating_aa(self):
        assert invert_8bit(0xAA) == 0x55

    def test_alternating_55(self):
        assert invert_8bit(0x55) == 0xAA

    def test_identity(self):
        assert invert_8bit(0x01) == 0xFE
        assert invert_8bit(0x80) == 0x7F
        assert invert_8bit(0x7F) == 0x80

    def test_double_invert(self):
        for v in range(0, 256, 17):
            assert invert_8bit(invert_8bit(v)) == v

    def test_sbc_twos_complement(self):
        # A - B = A + NOT(B) + 1 (with carry_in=1)
        a, b = 10, 3
        not_b = invert_8bit(b)
        result, carry = add_8bit(a, not_b, 1)
        assert result == 7
        assert carry == 1   # No borrow
