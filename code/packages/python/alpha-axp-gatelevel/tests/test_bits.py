"""test_bits.py — Unit tests for bits.py (64-bit bit-list conversion helpers).

Tests cover:
  - int_to_bits / bits_to_int round-trips at all widths
  - add_64bit: basic addition, carry propagation, overflow detection
  - add_128bit: basic 128-bit addition
  - add_32bit: basic 32-bit addition, overflow
  - invert_64bit / invert_32bit: NOT via gate functions
  - shl_64 / shr_64_logical / shr_64_arith: shifts by 0, 1, 32, 63
  - sext32_to_64: sign extension for positive and negative values
  - compute_zero: all-zeros and non-zero detection
"""

from __future__ import annotations

from alpha_axp_gatelevel.bits import (
    add_32bit,
    add_64bit,
    add_128bit,
    bits_to_int,
    compute_zero,
    int_to_bits,
    invert_32bit,
    invert_64bit,
    sext32_to_64,
    shl_64,
    shr_64_arith,
    shr_64_logical,
)

MASK64 = 0xFFFF_FFFF_FFFF_FFFF
MASK32 = 0xFFFF_FFFF


# ── int_to_bits / bits_to_int ─────────────────────────────────────────────────

class TestIntBitsRoundtrip:
    """int_to_bits and bits_to_int must be inverses of each other."""

    def test_zero_8bit(self):
        assert bits_to_int(int_to_bits(0, 8)) == 0

    def test_one_8bit(self):
        b = int_to_bits(1, 8)
        assert b[0] == 1
        assert all(x == 0 for x in b[1:])
        assert bits_to_int(b) == 1

    def test_five_8bit(self):
        b = int_to_bits(5, 8)
        # 5 = 0b00000101: bit0=1, bit1=0, bit2=1
        assert b == [1, 0, 1, 0, 0, 0, 0, 0]
        assert bits_to_int(b) == 5

    def test_max_8bit(self):
        assert bits_to_int(int_to_bits(255, 8)) == 255

    def test_zero_64bit(self):
        b = int_to_bits(0, 64)
        assert len(b) == 64
        assert all(x == 0 for x in b)
        assert bits_to_int(b) == 0

    def test_max_64bit(self):
        v = MASK64
        b = int_to_bits(v, 64)
        assert len(b) == 64
        assert all(x == 1 for x in b)
        assert bits_to_int(b) == v

    def test_roundtrip_64bit_various(self):
        for v in [0, 1, 0x1234, 0xDEAD_BEEF, 0x8000_0000_0000_0000, MASK64]:
            assert bits_to_int(int_to_bits(v, 64)) == v

    def test_lsb_first_ordering(self):
        # 4 = 0b100 → bit0=0, bit1=0, bit2=1
        b = int_to_bits(4, 8)
        assert b[0] == 0
        assert b[1] == 0
        assert b[2] == 1

    def test_negative_python_int_masked(self):
        # Python int -1 masked to 64 bits = all ones
        b = int_to_bits(-1, 64)
        assert all(x == 1 for x in b)

    def test_wider_than_width_masked(self):
        # Value 0x1FF masked to 8 bits = 0xFF
        b = int_to_bits(0x1FF, 8)
        assert bits_to_int(b) == 0xFF


# ── add_64bit ─────────────────────────────────────────────────────────────────

class TestAdd64bit:
    def test_zero_plus_zero(self):
        result, carry, overflow = add_64bit(0, 0)
        assert result == 0
        assert carry == 0
        assert overflow == 0

    def test_basic_add(self):
        result, carry, overflow = add_64bit(3, 4)
        assert result == 7
        assert carry == 0
        assert overflow == 0

    def test_add_with_carry_in(self):
        result, carry, overflow = add_64bit(3, 4, carry_in=1)
        assert result == 8

    def test_carry_out(self):
        # 2^64 - 1 + 1 = 2^64 → carry=1, result=0
        result, carry, overflow = add_64bit(MASK64, 1)
        assert result == 0
        assert carry == 1

    def test_no_overflow_positive(self):
        # Small positive + small positive, no overflow
        result, carry, overflow = add_64bit(1, 1)
        assert overflow == 0

    def test_overflow_positive_plus_positive(self):
        # MAX_POS + 1 → overflow (wrap to negative)
        max_pos = 0x7FFF_FFFF_FFFF_FFFF
        result, carry, overflow = add_64bit(max_pos, 1)
        assert overflow == 1
        assert result == 0x8000_0000_0000_0000

    def test_overflow_negative_plus_negative(self):
        # Two large negative numbers: MIN_NEG + MIN_NEG
        min_neg = 0x8000_0000_0000_0000
        result, carry, overflow = add_64bit(min_neg, min_neg)
        assert overflow == 1

    def test_no_overflow_mixed_signs(self):
        # Positive + negative: never overflows
        pos = 0x7FFF_FFFF_FFFF_FFFF
        neg = 0x8000_0000_0000_0000
        result, carry, overflow = add_64bit(pos, neg)
        assert overflow == 0


# ── add_128bit ────────────────────────────────────────────────────────────────

class TestAdd128bit:
    def test_basic(self):
        result, carry = add_128bit(1, 2)
        assert result == 3
        assert carry == 0

    def test_zero(self):
        result, carry = add_128bit(0, 0)
        assert result == 0

    def test_large(self):
        # 2^127 + 2^127 = 2^128 → carry=1, result=0
        half = 1 << 127
        result, carry = add_128bit(half, half)
        assert result == 0
        assert carry == 1

    def test_with_carry_in(self):
        result, carry = add_128bit(0, 0, carry_in=1)
        assert result == 1


# ── add_32bit ─────────────────────────────────────────────────────────────────

class TestAdd32bit:
    def test_basic(self):
        result, carry, overflow = add_32bit(3, 4)
        assert result == 7

    def test_carry(self):
        result, carry, overflow = add_32bit(MASK32, 1)
        assert result == 0
        assert carry == 1

    def test_overflow(self):
        max32 = 0x7FFF_FFFF
        result, carry, overflow = add_32bit(max32, 1)
        assert overflow == 1

    def test_no_overflow(self):
        _, _, overflow = add_32bit(1, 1)
        assert overflow == 0


# ── invert_64bit / invert_32bit ───────────────────────────────────────────────

class TestInvert:
    def test_invert_zero_64(self):
        assert invert_64bit(0) == MASK64

    def test_invert_all_ones_64(self):
        assert invert_64bit(MASK64) == 0

    def test_invert_pattern_64(self):
        v = 0xAAAA_AAAA_AAAA_AAAA
        assert invert_64bit(v) == 0x5555_5555_5555_5555

    def test_double_invert_64(self):
        v = 0x1234_5678_9ABC_DEF0
        assert invert_64bit(invert_64bit(v)) == v

    def test_invert_zero_32(self):
        assert invert_32bit(0) == MASK32

    def test_invert_all_ones_32(self):
        assert invert_32bit(MASK32) == 0

    def test_double_invert_32(self):
        v = 0xDEAD_BEEF
        assert invert_32bit(invert_32bit(v)) == v


# ── shl_64 ────────────────────────────────────────────────────────────────────

class TestShl64:
    def test_shift_by_zero(self):
        assert shl_64(1, 0) == 1
        assert shl_64(0xABCD, 0) == 0xABCD

    def test_shift_by_one(self):
        assert shl_64(1, 1) == 2

    def test_shift_by_four(self):
        assert shl_64(1, 4) == 16

    def test_shift_by_32(self):
        assert shl_64(1, 32) == 0x1_0000_0000

    def test_shift_by_63(self):
        assert shl_64(1, 63) == 0x8000_0000_0000_0000

    def test_shift_wraparound(self):
        # Shifting 1 by 64 (masked to 0) = 1
        assert shl_64(1, 64) == 1  # 64 & 63 = 0

    def test_shift_loses_bits(self):
        # 0xFFFF_FFFF_FFFF_FFFF << 1: low bit becomes 0, high bit lost
        v = shl_64(MASK64, 1)
        assert v == MASK64 & ~1  # 0xFFFFFFFFFFFFFFFE

    def test_shift_by_63_word(self):
        assert shl_64(3, 63) == 0x8000_0000_0000_0000  # low bit of 3 shifted to MSB


# ── shr_64_logical ────────────────────────────────────────────────────────────

class TestShr64Logical:
    def test_shift_by_zero(self):
        assert shr_64_logical(8, 0) == 8

    def test_shift_by_one(self):
        assert shr_64_logical(8, 1) == 4

    def test_shift_by_four(self):
        assert shr_64_logical(16, 4) == 1

    def test_shift_by_32(self):
        assert shr_64_logical(0x1_0000_0000, 32) == 1

    def test_shift_by_63(self):
        assert shr_64_logical(0x8000_0000_0000_0000, 63) == 1

    def test_zero_fill_msbs(self):
        # MSBs must be zero after logical shift
        v = shr_64_logical(MASK64, 1)
        assert v == 0x7FFF_FFFF_FFFF_FFFF  # MSB cleared

    def test_negative_zero_fills(self):
        # 0xFFFF... >> 1 (logical) = 0x7FFF... (no sign extension)
        v = shr_64_logical(MASK64, 4)
        assert v == MASK64 >> 4


# ── shr_64_arith ─────────────────────────────────────────────────────────────

class TestShr64Arith:
    def test_positive_same_as_logical(self):
        assert shr_64_arith(8, 1) == 4

    def test_negative_sign_extends(self):
        # 0xFFFF... >> 1 (arith) = 0xFFFF... (sign bit replicated)
        assert shr_64_arith(MASK64, 1) == MASK64

    def test_shift_by_zero(self):
        assert shr_64_arith(5, 0) == 5

    def test_negative_shift_by_63(self):
        # Any negative >> 63 = all ones (sign bit fills all positions)
        assert shr_64_arith(0x8000_0000_0000_0000, 63) == MASK64

    def test_positive_shift_by_63(self):
        # Positive >> 63 = 0 (sign bit is 0)
        assert shr_64_arith(0x7FFF_FFFF_FFFF_FFFF, 63) == 0

    def test_shift_by_32_negative(self):
        v = shr_64_arith(0xFFFF_FFFF_0000_0000, 32)
        assert v == MASK64  # all ones (sign extended)


# ── sext32_to_64 ──────────────────────────────────────────────────────────────

class TestSext32To64:
    def test_zero(self):
        assert sext32_to_64(0) == 0

    def test_positive_max(self):
        # 0x7FFF_FFFF stays 0x7FFF_FFFF (bit 31 = 0)
        assert sext32_to_64(0x7FFF_FFFF) == 0x7FFF_FFFF

    def test_negative_min(self):
        # 0x8000_0000 → 0xFFFF_FFFF_8000_0000
        assert sext32_to_64(0x8000_0000) == 0xFFFF_FFFF_8000_0000

    def test_minus_one(self):
        # 0xFFFF_FFFF → 0xFFFF_FFFF_FFFF_FFFF
        assert sext32_to_64(0xFFFF_FFFF) == MASK64

    def test_one(self):
        assert sext32_to_64(1) == 1

    def test_boundary_between_pos_neg(self):
        # 0x7FFF_FFFF is max positive (bit31=0); 0x8000_0000 is min negative (bit31=1)
        assert sext32_to_64(0x7FFF_FFFF) < 0x8000_0000_0000_0000
        assert sext32_to_64(0x8000_0000) >= 0x8000_0000_0000_0000


# ── compute_zero ──────────────────────────────────────────────────────────────

class TestComputeZero:
    def test_all_zeros_returns_1(self):
        assert compute_zero([0, 0, 0, 0]) == 1

    def test_any_one_returns_0(self):
        assert compute_zero([0, 1, 0, 0]) == 0
        assert compute_zero([1, 0, 0, 0]) == 0
        assert compute_zero([0, 0, 0, 1]) == 0

    def test_all_ones_returns_0(self):
        assert compute_zero([1, 1, 1, 1]) == 0

    def test_64bit_zero(self):
        assert compute_zero([0] * 64) == 1

    def test_64bit_nonzero(self):
        bits = [0] * 64
        bits[0] = 1
        assert compute_zero(bits) == 0
