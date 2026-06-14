"""Tests for bits.py — 32-bit integer ↔ bit-list conversion helpers."""


from mips_r2000_gatelevel.bits import (
    add_32bit,
    add_64bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_32bit,
    shl_32,
    shr_32_arith,
    shr_32_logical,
)

# ── int_to_bits / bits_to_int round-trips ─────────────────────────────────────


class TestIntBitsRoundtrip:
    def test_zero(self):
        assert int_to_bits(0, 32) == [0] * 32
        assert bits_to_int([0] * 32) == 0

    def test_one(self):
        bits = int_to_bits(1, 32)
        assert bits[0] == 1
        assert all(b == 0 for b in bits[1:])
        assert bits_to_int(bits) == 1

    def test_five(self):
        bits = int_to_bits(5, 8)
        assert bits == [1, 0, 1, 0, 0, 0, 0, 0]
        assert bits_to_int(bits) == 5

    def test_max_uint32(self):
        val = 0xFFFF_FFFF
        bits = int_to_bits(val, 32)
        assert all(b == 1 for b in bits)
        assert bits_to_int(bits) == val

    def test_msb_only(self):
        val = 0x8000_0000
        bits = int_to_bits(val, 32)
        assert bits[31] == 1
        assert all(b == 0 for b in bits[:31])
        assert bits_to_int(bits) == val

    def test_round_trip_arbitrary(self):
        for val in [0, 1, 127, 255, 0x1234_5678, 0xDEAD_BEEF, 0xFFFF_FFFF]:
            assert bits_to_int(int_to_bits(val, 32)) == val

    def test_overflow_masked(self):
        # Value larger than 32 bits is masked
        assert bits_to_int(int_to_bits(0x1_0000_0000, 32)) == 0

    def test_width_4(self):
        assert int_to_bits(7, 4) == [1, 1, 1, 0]

    def test_lsb_first_ordering(self):
        # 5 = 0b101 → bit0=1, bit1=0, bit2=1
        bits = int_to_bits(5, 4)
        assert bits[0] == 1  # bit 0 (value 1)
        assert bits[1] == 0  # bit 1 (value 2)
        assert bits[2] == 1  # bit 2 (value 4)
        assert bits[3] == 0  # bit 3 (value 8)


# ── add_32bit ──────────────────────────────────────────────────────────────────


class TestAdd32bit:
    def test_basic(self):
        result, carry, overflow = add_32bit(1, 1)
        assert result == 2
        assert carry == 0
        assert overflow == 0

    def test_zero_plus_zero(self):
        result, carry, overflow = add_32bit(0, 0)
        assert result == 0
        assert carry == 0
        assert overflow == 0

    def test_carry_out(self):
        # 0xFFFFFFFF + 1 = 0 with carry_out=1
        result, carry, overflow = add_32bit(0xFFFF_FFFF, 1)
        assert result == 0
        assert carry == 1

    def test_no_overflow_positive(self):
        # 1 + 1 = 2 (no signed overflow)
        _, _, overflow = add_32bit(1, 1)
        assert overflow == 0

    def test_positive_overflow(self):
        # 0x7FFFFFFF + 1 = 0x80000000 — signed overflow (positive + positive = negative)
        result, carry, overflow = add_32bit(0x7FFF_FFFF, 1)
        assert result == 0x8000_0000
        assert overflow == 1

    def test_negative_overflow(self):
        # 0x80000000 + 0x80000000 — both negative, result = 0 (overflow)
        result, carry, overflow = add_32bit(0x8000_0000, 0x8000_0000)
        assert result == 0
        assert overflow == 1

    def test_carry_in(self):
        result, carry, overflow = add_32bit(0, 0, carry_in=1)
        assert result == 1
        assert carry == 0

    def test_large_values(self):
        result, carry, overflow = add_32bit(0x1234_5678, 0x8765_4321)
        assert result == (0x1234_5678 + 0x8765_4321) & 0xFFFF_FFFF

    def test_no_overflow_negative_plus_positive(self):
        # -1 (0xFFFFFFFF) + 1 = 0; no signed overflow
        result, carry, overflow = add_32bit(0xFFFF_FFFF, 1)
        assert result == 0
        assert overflow == 0


# ── add_64bit ──────────────────────────────────────────────────────────────────


class TestAdd64bit:
    def test_basic(self):
        result, carry = add_64bit(1, 1)
        assert result == 2
        assert carry == 0

    def test_zero(self):
        result, carry = add_64bit(0, 0)
        assert result == 0
        assert carry == 0

    def test_large_values(self):
        a = 0xFFFF_FFFF_0000_0000
        b = 0x0000_0000_FFFF_FFFF
        result, carry = add_64bit(a, b)
        assert result == a + b
        assert carry == 0

    def test_carry_out(self):
        max64 = 0xFFFF_FFFF_FFFF_FFFF
        result, carry = add_64bit(max64, 1)
        assert result == 0
        assert carry == 1

    def test_with_carry_in(self):
        result, carry = add_64bit(0, 0, carry_in=1)
        assert result == 1


# ── invert_32bit ───────────────────────────────────────────────────────────────


class TestInvert32bit:
    def test_zero_inverted(self):
        assert invert_32bit(0) == 0xFFFF_FFFF

    def test_all_ones_inverted(self):
        assert invert_32bit(0xFFFF_FFFF) == 0

    def test_pattern(self):
        assert invert_32bit(0xAAAA_AAAA) == 0x5555_5555

    def test_double_invert(self):
        for val in [0, 1, 0x1234_5678, 0xDEAD_BEEF, 0xFFFF_FFFF]:
            assert invert_32bit(invert_32bit(val)) == val

    def test_one(self):
        assert invert_32bit(1) == 0xFFFF_FFFE


# ── compute_parity ─────────────────────────────────────────────────────────────


class TestComputeParity:
    def test_all_zero(self):
        assert compute_parity([0] * 8) == 0

    def test_single_one(self):
        assert compute_parity([1, 0, 0, 0]) == 1

    def test_two_ones(self):
        assert compute_parity([1, 1, 0, 0]) == 0

    def test_odd_count(self):
        assert compute_parity([1, 0, 1, 0, 1]) == 1


# ── compute_zero ───────────────────────────────────────────────────────────────


class TestComputeZero:
    def test_all_zero(self):
        assert compute_zero([0] * 32) == 1

    def test_one_set(self):
        bits = [0] * 32
        bits[15] = 1
        assert compute_zero(bits) == 0

    def test_all_ones(self):
        assert compute_zero([1] * 32) == 0

    def test_single_lsb(self):
        bits = [1] + [0] * 31
        assert compute_zero(bits) == 0


# ── shl_32 ─────────────────────────────────────────────────────────────────────


class TestShl32:
    def test_shift_zero(self):
        assert shl_32(0b1010, 0) == 0b1010

    def test_shift_one(self):
        assert shl_32(1, 1) == 2

    def test_shift_31(self):
        assert shl_32(1, 31) == 0x8000_0000

    def test_shift_out_of_range(self):
        assert shl_32(0xFFFF_FFFF, 32) == 0

    def test_shift_16(self):
        assert shl_32(0xFF, 16) == 0xFF_0000

    def test_overflow_wrapped(self):
        # Shifting 1 by 32 → 0 (overflow)
        assert shl_32(1, 32) == 0

    def test_identity(self):
        assert shl_32(0xABCD, 0) == 0xABCD


# ── shr_32_logical ─────────────────────────────────────────────────────────────


class TestShr32Logical:
    def test_shift_zero(self):
        assert shr_32_logical(0b1010, 0) == 0b1010

    def test_shift_one(self):
        assert shr_32_logical(4, 1) == 2

    def test_shift_31(self):
        assert shr_32_logical(0x8000_0000, 31) == 1

    def test_zero_fill(self):
        # MSB bit 31 = 1 but result MSB should be 0 after logical shift
        result = shr_32_logical(0xFFFF_FFFF, 1)
        assert result == 0x7FFF_FFFF

    def test_shift_out_of_range(self):
        assert shr_32_logical(0xFFFF_FFFF, 32) == 0

    def test_shift_16(self):
        assert shr_32_logical(0xFFFF_0000, 16) == 0x0000_FFFF


# ── shr_32_arith ───────────────────────────────────────────────────────────────


class TestShr32Arith:
    def test_positive_no_change(self):
        # Positive value: arithmetic same as logical
        assert shr_32_arith(4, 1) == 2

    def test_negative_sign_fill(self):
        # 0x80000000 (min signed) >> 1 should fill with 1s
        result = shr_32_arith(0x8000_0000, 1)
        assert result == 0xC000_0000

    def test_negative_fully_extended(self):
        # 0x80000000 >> 31 = 0xFFFFFFFF (all sign bits)
        result = shr_32_arith(0x8000_0000, 31)
        assert result == 0xFFFF_FFFF

    def test_zero_shift(self):
        assert shr_32_arith(0xDEAD_BEEF, 0) == 0xDEAD_BEEF

    def test_positive_large_shift(self):
        # positive value shifted more than width → 0
        assert shr_32_arith(0x7FFF_FFFF, 32) == 0

    def test_negative_large_shift(self):
        # negative value shifted more than width → 0xFFFFFFFF
        assert shr_32_arith(0xFFFF_FFFF, 32) == 0xFFFF_FFFF

    def test_shift_one_negative(self):
        # -2 (0xFFFFFFFE) >> 1 = -1 (0xFFFFFFFF)
        result = shr_32_arith(0xFFFF_FFFE, 1)
        assert result == 0xFFFF_FFFF
