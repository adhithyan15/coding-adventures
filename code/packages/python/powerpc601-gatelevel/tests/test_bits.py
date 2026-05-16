"""test_bits.py — Unit tests for the 32-bit bit-list helpers.

Tests cover:
- int_to_bits / bits_to_int round-trip
- add_32bit (carry, overflow detection)
- add_64bit
- invert_32bit
- compute_zero / compute_parity
- shl_32, shr_32_logical, shr_32_arith
- rotl_32
"""

from __future__ import annotations

from powerpc601_gatelevel.bits import (
    add_32bit,
    add_64bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_32bit,
    rotl_32,
    shl_32,
    shr_32_arith,
    shr_32_logical,
)

# ── int_to_bits / bits_to_int ─────────────────────────────────────────────────

class TestIntToBits:
    def test_zero(self):
        assert int_to_bits(0, 8) == [0, 0, 0, 0, 0, 0, 0, 0]

    def test_one(self):
        bits = int_to_bits(1, 8)
        assert bits[0] == 1
        assert bits[1:] == [0] * 7

    def test_five(self):
        # 5 = 0b101
        bits = int_to_bits(5, 8)
        assert bits[0] == 1  # bit 0 (value 1)
        assert bits[1] == 0  # bit 1 (value 2)
        assert bits[2] == 1  # bit 2 (value 4)
        assert bits[3:] == [0] * 5

    def test_all_ones(self):
        bits = int_to_bits(0xFF, 8)
        assert bits == [1] * 8

    def test_32bit_max(self):
        bits = int_to_bits(0xFFFFFFFF, 32)
        assert bits == [1] * 32

    def test_negative_masked(self):
        # -1 in 32-bit = 0xFFFFFFFF
        bits = int_to_bits(-1, 32)
        assert bits == [1] * 32

    def test_width_4(self):
        bits = int_to_bits(0xA, 4)
        assert bits == [0, 1, 0, 1]

    def test_msb_set(self):
        bits = int_to_bits(0x80000000, 32)
        assert bits[31] == 1
        assert bits[:31] == [0] * 31


class TestBitsToInt:
    def test_zero(self):
        assert bits_to_int([0, 0, 0, 0]) == 0

    def test_one(self):
        assert bits_to_int([1, 0, 0, 0]) == 1

    def test_five(self):
        assert bits_to_int([1, 0, 1, 0]) == 5  # 1 + 4

    def test_all_ones_8bit(self):
        assert bits_to_int([1] * 8) == 255

    def test_round_trip_random(self):
        for v in [0, 1, 42, 127, 128, 255, 0x12345678, 0xFFFFFFFF]:
            bits = int_to_bits(v, 32)
            assert bits_to_int(bits) == v


# ── add_32bit ─────────────────────────────────────────────────────────────────

class TestAdd32bit:
    def test_zero_plus_zero(self):
        result, carry, overflow = add_32bit(0, 0)
        assert result == 0
        assert carry == 0
        assert overflow == 0

    def test_simple_add(self):
        result, carry, overflow = add_32bit(3, 4)
        assert result == 7
        assert carry == 0
        assert overflow == 0

    def test_max_plus_one_wraps(self):
        result, carry, overflow = add_32bit(0xFFFFFFFF, 1)
        assert result == 0
        assert carry == 1
        assert overflow == 0  # unsigned wraparound, not signed overflow

    def test_signed_overflow_positive(self):
        # 0x7FFFFFFF + 1 = 0x80000000: signed overflow
        result, carry, overflow = add_32bit(0x7FFFFFFF, 1)
        assert result == 0x80000000
        assert carry == 0
        assert overflow == 1

    def test_signed_overflow_negative(self):
        # 0x80000000 + 0x80000000 → 0 with overflow
        result, carry, overflow = add_32bit(0x80000000, 0x80000000)
        assert result == 0
        assert carry == 1
        assert overflow == 1

    def test_with_carry_in(self):
        result, carry, overflow = add_32bit(0, 0, carry_in=1)
        assert result == 1
        assert carry == 0

    def test_carry_in_with_max(self):
        result, carry, overflow = add_32bit(0xFFFFFFFF, 0, carry_in=1)
        assert result == 0
        assert carry == 1

    def test_commutative(self):
        r1, c1, ov1 = add_32bit(10, 20)
        r2, c2, ov2 = add_32bit(20, 10)
        assert r1 == r2
        assert c1 == c2
        assert ov1 == ov2

    def test_no_overflow_negative_plus_positive(self):
        # -1 + 1 = 0, no overflow
        result, carry, overflow = add_32bit(0xFFFFFFFF, 1)
        assert result == 0
        assert overflow == 0


# ── add_64bit ─────────────────────────────────────────────────────────────────

class TestAdd64bit:
    def test_simple(self):
        result, carry = add_64bit(3, 4)
        assert result == 7
        assert carry == 0

    def test_zero(self):
        result, carry = add_64bit(0, 0)
        assert result == 0
        assert carry == 0

    def test_overflow(self):
        mask64 = (1 << 64) - 1
        result, carry = add_64bit(mask64, 1)
        assert result == 0
        assert carry == 1

    def test_carry_in(self):
        result, carry = add_64bit(0, 0, carry_in=1)
        assert result == 1
        assert carry == 0

    def test_large(self):
        a = 0xDEADBEEFCAFEBABE
        b = 0x0102030405060708
        expected = (a + b) & ((1 << 64) - 1)
        result, _ = add_64bit(a, b)
        assert result == expected


# ── invert_32bit ──────────────────────────────────────────────────────────────

class TestInvert32bit:
    def test_zero_to_all_ones(self):
        assert invert_32bit(0) == 0xFFFFFFFF

    def test_all_ones_to_zero(self):
        assert invert_32bit(0xFFFFFFFF) == 0

    def test_pattern(self):
        assert invert_32bit(0xAAAAAAAA) == 0x55555555

    def test_involution(self):
        # NOT(NOT(x)) = x
        for v in [0, 1, 0xDEADBEEF, 0x7FFFFFFF]:
            assert invert_32bit(invert_32bit(v)) == v


# ── compute_zero / compute_parity ─────────────────────────────────────────────

class TestComputeZero:
    def test_all_zeros(self):
        assert compute_zero([0, 0, 0, 0]) == 1

    def test_has_one(self):
        assert compute_zero([0, 1, 0, 0]) == 0

    def test_all_ones(self):
        assert compute_zero([1, 1, 1, 1]) == 0

    def test_single_zero(self):
        assert compute_zero([0]) == 1

    def test_single_one(self):
        assert compute_zero([1]) == 0

    def test_32bits_zero(self):
        assert compute_zero([0] * 32) == 1

    def test_32bits_nonzero(self):
        bits = [0] * 32
        bits[17] = 1
        assert compute_zero(bits) == 0


class TestComputeParity:
    def test_zero_bits(self):
        assert compute_parity([0, 0, 0, 0]) == 0

    def test_one_set_bit(self):
        assert compute_parity([1, 0, 0, 0]) == 1

    def test_two_set_bits(self):
        assert compute_parity([1, 1, 0, 0]) == 0

    def test_three_set_bits(self):
        assert compute_parity([1, 1, 1, 0]) == 1

    def test_all_ones(self):
        # 4 ones → even → parity = 0
        assert compute_parity([1, 1, 1, 1]) == 0

    def test_single_one(self):
        assert compute_parity([1]) == 1


# ── shl_32 ────────────────────────────────────────────────────────────────────

class TestShl32:
    def test_shift_by_zero(self):
        assert shl_32(0xDEADBEEF, 0) == 0xDEADBEEF

    def test_shift_one_by_one(self):
        assert shl_32(1, 1) == 2

    def test_shift_to_msb(self):
        assert shl_32(1, 31) == 0x80000000

    def test_shift_out(self):
        # MSB shifted out
        assert shl_32(0x80000000, 1) == 0

    def test_shift_by_32_gives_zero(self):
        assert shl_32(0xFFFFFFFF, 32) == 0

    def test_shift_by_large(self):
        # shamt=63: 63 & 0x3F = 63 >= 32 → 0
        assert shl_32(1, 63) == 0

    def test_shift_byte(self):
        assert shl_32(0xFF, 8) == 0xFF00

    def test_shift_preserves_low_bits(self):
        # 0b1011 << 2 = 0b101100
        assert shl_32(0b1011, 2) == 0b101100


# ── shr_32_logical ────────────────────────────────────────────────────────────

class TestShr32Logical:
    def test_shift_by_zero(self):
        assert shr_32_logical(0xDEADBEEF, 0) == 0xDEADBEEF

    def test_shift_one_right(self):
        assert shr_32_logical(2, 1) == 1

    def test_no_sign_extension(self):
        # Negative number shifted: high bit fills with 0
        result = shr_32_logical(0xFFFFFFFF, 1)
        assert result == 0x7FFFFFFF

    def test_shift_to_zero(self):
        assert shr_32_logical(1, 1) == 0

    def test_shift_by_32_gives_zero(self):
        assert shr_32_logical(0xFFFFFFFF, 32) == 0

    def test_msb_shift(self):
        assert shr_32_logical(0x80000000, 31) == 1


# ── shr_32_arith ──────────────────────────────────────────────────────────────

class TestShr32Arith:
    def test_positive_no_sign_ext(self):
        assert shr_32_arith(8, 3) == 1

    def test_negative_sign_ext(self):
        # -1 >> 1 = -1 (all ones)
        assert shr_32_arith(0xFFFFFFFF, 1) == 0xFFFFFFFF

    def test_negative_partial(self):
        # 0x80000000 >> 1 = 0xC0000000
        assert shr_32_arith(0x80000000, 1) == 0xC0000000

    def test_shift_by_zero(self):
        assert shr_32_arith(0xDEADBEEF, 0) == 0xDEADBEEF

    def test_saturate_to_sign(self):
        # Large shift: fills with sign bit
        result = shr_32_arith(0x80000000, 31)
        assert result == 0xFFFFFFFF

    def test_positive_sign_fill_zero(self):
        assert shr_32_arith(0x7FFFFFFF, 1) == 0x3FFFFFFF


# ── rotl_32 ───────────────────────────────────────────────────────────────────

class TestRotl32:
    def test_rotate_by_zero(self):
        assert rotl_32(0xDEADBEEF, 0) == 0xDEADBEEF

    def test_rotate_msb_to_lsb(self):
        # 0x80000000 rotated left 1 → LSB becomes 1
        assert rotl_32(0x80000000, 1) == 1

    def test_rotate_lsb_to_bit1(self):
        assert rotl_32(1, 1) == 2

    def test_rotate_by_32_is_identity(self):
        v = 0xDEADBEEF
        assert rotl_32(v, 32 & 31) == v  # 32 % 32 = 0

    def test_rotate_8_known(self):
        # 0xDEADBEEF rotated left 8 should give specific value
        v = 0xDEADBEEF
        expected = ((v << 8) | (v >> 24)) & 0xFFFFFFFF
        assert rotl_32(v, 8) == expected

    def test_rotate_and_back(self):
        # Rotate left n then right n (= left 32-n) should give original
        v = 0x12345678
        for n in [1, 4, 8, 16, 31]:
            rotated = rotl_32(v, n)
            back = rotl_32(rotated, 32 - n)
            assert back == v, f"failed for n={n}"

    def test_rotate_nibbles(self):
        # 0x12345678 rotated left 4 → 0x23456781
        assert rotl_32(0x12345678, 4) == 0x23456781
