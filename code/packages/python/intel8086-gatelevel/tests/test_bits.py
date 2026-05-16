"""Tests for bits.py — bit conversion and gate-level helpers."""

import pytest

from intel8086_gatelevel.bits import (
    add_16bit,
    add_20bit,
    add_8bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_16bit,
    invert_8bit,
)


# ── int_to_bits ───────────────────────────────────────────────────────────────

class TestIntToBits:
    def test_zero_8bit(self):
        assert int_to_bits(0, 8) == [0] * 8

    def test_one_8bit(self):
        assert int_to_bits(1, 8) == [1, 0, 0, 0, 0, 0, 0, 0]

    def test_five_8bit(self):
        assert int_to_bits(5, 8) == [1, 0, 1, 0, 0, 0, 0, 0]

    def test_255_8bit(self):
        assert int_to_bits(0xFF, 8) == [1] * 8

    def test_256_8bit_wraps(self):
        assert int_to_bits(0x100, 8) == [0] * 8

    def test_zero_16bit(self):
        assert int_to_bits(0, 16) == [0] * 16

    def test_one_16bit(self):
        result = int_to_bits(1, 16)
        assert result[0] == 1
        assert all(b == 0 for b in result[1:])

    def test_0x100_16bit(self):
        result = int_to_bits(0x100, 16)
        assert result[8] == 1
        assert result[0] == 0

    def test_0xffff_16bit(self):
        assert int_to_bits(0xFFFF, 16) == [1] * 16

    def test_0x10000_16bit_wraps(self):
        assert int_to_bits(0x10000, 16) == [0] * 16

    def test_20bit(self):
        bits = int_to_bits(0xFFFFF, 20)
        assert bits == [1] * 20

    def test_20bit_zero(self):
        assert int_to_bits(0, 20) == [0] * 20

    def test_lsb_first(self):
        # 0b1010 = 10 → bits [0, 1, 0, 1, ...]
        bits = int_to_bits(10, 8)
        assert bits[0] == 0  # LSB
        assert bits[1] == 1
        assert bits[2] == 0
        assert bits[3] == 1


# ── bits_to_int ───────────────────────────────────────────────────────────────

class TestBitsToInt:
    def test_zero(self):
        assert bits_to_int([0] * 8) == 0

    def test_one(self):
        assert bits_to_int([1, 0, 0, 0, 0, 0, 0, 0]) == 1

    def test_five(self):
        assert bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) == 5

    def test_255(self):
        assert bits_to_int([1] * 8) == 255

    def test_16bit_0x100(self):
        bits = [0] * 16
        bits[8] = 1
        assert bits_to_int(bits) == 0x100

    def test_roundtrip_8bit(self):
        for v in [0, 1, 127, 128, 255]:
            assert bits_to_int(int_to_bits(v, 8)) == v

    def test_roundtrip_16bit(self):
        for v in [0, 1, 255, 256, 0x7FFF, 0x8000, 0xFFFF]:
            assert bits_to_int(int_to_bits(v, 16)) == v

    def test_roundtrip_20bit(self):
        for v in [0, 1, 0x0FFFF, 0xFFFFF]:
            assert bits_to_int(int_to_bits(v, 20)) == v


# ── add_8bit ──────────────────────────────────────────────────────────────────

class TestAdd8Bit:
    def test_simple(self):
        r, cout, af = add_8bit(5, 3)
        assert r == 8
        assert cout == 0
        assert af == 0

    def test_no_carry(self):
        r, cout, af = add_8bit(10, 20)
        assert r == 30
        assert cout == 0

    def test_carry_out(self):
        r, cout, af = add_8bit(0xFF, 1)
        assert r == 0
        assert cout == 1

    def test_carry_out_2(self):
        r, cout, af = add_8bit(0xFF, 0xFF)
        assert r == 0xFE
        assert cout == 1

    def test_carry_in(self):
        r, cout, af = add_8bit(5, 3, carry_in=1)
        assert r == 9
        assert cout == 0

    def test_aux_carry(self):
        # 0x0F + 0x01 = 0x10 → carry from low nibble → AF=1
        r, cout, af = add_8bit(0x0F, 0x01)
        assert r == 0x10
        assert af == 1

    def test_no_aux_carry(self):
        r, cout, af = add_8bit(0x05, 0x03)
        assert af == 0

    def test_zero_plus_zero(self):
        r, cout, af = add_8bit(0, 0)
        assert r == 0
        assert cout == 0
        assert af == 0

    def test_carry_in_causes_carry_out(self):
        r, cout, af = add_8bit(0xFF, 0, carry_in=1)
        assert r == 0
        assert cout == 1

    def test_max_values(self):
        r, cout, af = add_8bit(0x80, 0x80)
        assert r == 0
        assert cout == 1


# ── add_16bit ─────────────────────────────────────────────────────────────────

class TestAdd16Bit:
    def test_simple(self):
        r, cout, af = add_16bit(5, 3)
        assert r == 8
        assert cout == 0

    def test_carry_out(self):
        r, cout, af = add_16bit(0xFFFF, 1)
        assert r == 0
        assert cout == 1

    def test_carry_in(self):
        r, cout, af = add_16bit(0xFFFF, 0, carry_in=1)
        assert r == 0
        assert cout == 1

    def test_no_carry(self):
        r, cout, af = add_16bit(0x1234, 0x0001)
        assert r == 0x1235
        assert cout == 0

    def test_roundtrip(self):
        r, cout, af = add_16bit(0x8000, 0x8000)
        assert r == 0
        assert cout == 1

    def test_aux_carry_from_low_nibble(self):
        r, cout, af = add_16bit(0x000F, 0x0001)
        assert r == 0x0010
        assert af == 1

    def test_no_aux_carry(self):
        r, cout, af = add_16bit(0x0001, 0x0001)
        assert af == 0

    def test_large_values(self):
        r, cout, af = add_16bit(0x7FFF, 0x7FFF)
        assert r == 0xFFFE
        assert cout == 0


# ── add_20bit ─────────────────────────────────────────────────────────────────

class TestAdd20Bit:
    def test_simple(self):
        r, cout = add_20bit(0x10000, 0x0100)
        assert r == 0x10100
        assert cout == 0

    def test_no_carry(self):
        r, cout = add_20bit(0, 0)
        assert r == 0

    def test_carry_out(self):
        r, cout = add_20bit(0xFFFFF, 1)
        assert r == 0
        assert cout == 1

    def test_segment_addressing(self):
        # CS=0x1000 → CS<<4 = 0x10000; IP=0x0100
        r, cout = add_20bit(0x10000, 0x0100)
        assert r == 0x10100

    def test_max_segment(self):
        r, cout = add_20bit(0xFFFF0, 0xF)
        assert r == 0xFFFFF

    def test_wrap_to_zero(self):
        r, cout = add_20bit(0xFFFFF, 1)
        assert r == 0
        assert cout == 1


# ── invert_8bit ───────────────────────────────────────────────────────────────

class TestInvert8Bit:
    def test_zero(self):
        assert invert_8bit(0) == 0xFF

    def test_ff(self):
        assert invert_8bit(0xFF) == 0

    def test_aa(self):
        assert invert_8bit(0xAA) == 0x55

    def test_55(self):
        assert invert_8bit(0x55) == 0xAA

    def test_roundtrip(self):
        for v in range(256):
            assert invert_8bit(invert_8bit(v)) == v

    def test_01(self):
        assert invert_8bit(1) == 0xFE

    def test_mask(self):
        # Masked to 8 bits
        assert invert_8bit(0x100) == 0xFF   # 0x100 & 0xFF = 0 → ~0 = 0xFF


# ── invert_16bit ──────────────────────────────────────────────────────────────

class TestInvert16Bit:
    def test_zero(self):
        assert invert_16bit(0) == 0xFFFF

    def test_ffff(self):
        assert invert_16bit(0xFFFF) == 0

    def test_aaaa(self):
        assert invert_16bit(0xAAAA) == 0x5555

    def test_roundtrip(self):
        for v in [0, 1, 0x1234, 0x7FFF, 0x8000, 0xFFFF]:
            assert invert_16bit(invert_16bit(v)) == v

    def test_one(self):
        assert invert_16bit(1) == 0xFFFE


# ── compute_parity ────────────────────────────────────────────────────────────

class TestComputeParity:
    def test_zero(self):
        # 0 ones → even → PF=1
        assert compute_parity([0] * 8) == 1

    def test_one_bit(self):
        # 1 one → odd → PF=0
        bits = [1] + [0] * 7
        assert compute_parity(bits) == 0

    def test_two_bits(self):
        # 2 ones → even → PF=1
        bits = [1, 1] + [0] * 6
        assert compute_parity(bits) == 1

    def test_all_ones(self):
        # 8 ones → even → PF=1
        assert compute_parity([1] * 8) == 1

    def test_seven_ones(self):
        # 7 ones → odd → PF=0
        bits = [1] * 7 + [0]
        assert compute_parity(bits) == 0

    def test_uses_only_low8(self):
        # Extra bits beyond 8 should not matter
        bits = [1, 1, 0, 0, 0, 0, 0, 0, 1, 1]  # 2 ones in low 8
        assert compute_parity(bits) == 1

    def test_four_ones(self):
        bits = [1, 0, 1, 0, 1, 0, 1, 0]
        assert compute_parity(bits) == 1  # 4 ones → even

    def test_three_ones(self):
        bits = [1, 0, 0, 1, 0, 0, 1, 0]
        assert compute_parity(bits) == 0  # 3 ones → odd


# ── compute_zero ─────────────────────────────────────────────────────────────

class TestComputeZero:
    def test_all_zero_8bit(self):
        assert compute_zero([0] * 8) == 1

    def test_one_bit_set(self):
        bits = [0] * 8
        bits[0] = 1
        assert compute_zero(bits) == 0

    def test_msb_set(self):
        bits = [0] * 8
        bits[7] = 1
        assert compute_zero(bits) == 0

    def test_all_ones(self):
        assert compute_zero([1] * 8) == 0

    def test_all_zero_16bit(self):
        assert compute_zero([0] * 16) == 1

    def test_16bit_nonzero(self):
        bits = [0] * 16
        bits[15] = 1
        assert compute_zero(bits) == 0

    def test_all_zero_1bit(self):
        assert compute_zero([0]) == 1

    def test_nonzero_1bit(self):
        assert compute_zero([1]) == 0
