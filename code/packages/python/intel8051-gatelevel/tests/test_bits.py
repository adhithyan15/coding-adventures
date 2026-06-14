"""Tests for intel8051_gatelevel.bits — the integer ↔ bit-list bridge."""

from intel8051_gatelevel.bits import (
    add_8bit,
    add_16bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_8bit,
)


class TestIntToBits:
    def test_zero(self):
        assert int_to_bits(0, 8) == [0, 0, 0, 0, 0, 0, 0, 0]

    def test_one(self):
        assert int_to_bits(1, 8) == [1, 0, 0, 0, 0, 0, 0, 0]

    def test_five(self):
        # 5 = 0b00000101 → LSB first → [1, 0, 1, 0, 0, 0, 0, 0]
        assert int_to_bits(5, 8) == [1, 0, 1, 0, 0, 0, 0, 0]

    def test_max_byte(self):
        assert int_to_bits(0xFF, 8) == [1, 1, 1, 1, 1, 1, 1, 1]

    def test_msb_only(self):
        assert int_to_bits(0x80, 8) == [0, 0, 0, 0, 0, 0, 0, 1]

    def test_16bit(self):
        # 0x0001 → bit 0 set
        result = int_to_bits(1, 16)
        assert result[0] == 1
        assert all(b == 0 for b in result[1:])

    def test_masking(self):
        # Values > 255 are masked to 8 bits
        assert int_to_bits(0x1FF, 8) == int_to_bits(0xFF, 8)

    def test_lsb_first_convention(self):
        # 0xA0 = 0b10100000: bits 7 and 5 set, LSB-first → [0,0,0,0,0,1,0,1]
        result = int_to_bits(0xA0, 8)
        assert result[5] == 1
        assert result[7] == 1
        assert result[0] == 0


class TestBitsToInt:
    def test_zero(self):
        assert bits_to_int([0, 0, 0, 0, 0, 0, 0, 0]) == 0

    def test_one(self):
        assert bits_to_int([1, 0, 0, 0, 0, 0, 0, 0]) == 1

    def test_five(self):
        assert bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) == 5

    def test_max_byte(self):
        assert bits_to_int([1, 1, 1, 1, 1, 1, 1, 1]) == 255

    def test_roundtrip(self):
        for v in range(256):
            assert bits_to_int(int_to_bits(v, 8)) == v

    def test_16bit_roundtrip(self):
        for v in [0, 1, 0x100, 0xFF, 0xFFFF, 0x1234]:
            assert bits_to_int(int_to_bits(v, 16)) == v


class TestAdd8bit:
    def test_zero_plus_zero(self):
        result, carry, ac = add_8bit(0, 0, 0)
        assert result == 0
        assert carry == 0
        assert ac == 0

    def test_one_plus_one(self):
        result, carry, ac = add_8bit(1, 1, 0)
        assert result == 2
        assert carry == 0

    def test_overflow(self):
        result, carry, ac = add_8bit(0xFF, 1, 0)
        assert result == 0
        assert carry == 1

    def test_carry_in(self):
        result, carry, ac = add_8bit(0xFF, 0, 1)
        assert result == 0
        assert carry == 1

    def test_aux_carry(self):
        # 0x0F + 0x01 = 0x10, carry from bit 3 to bit 4 (AC=1)
        result, carry, ac = add_8bit(0x0F, 0x01, 0)
        assert result == 0x10
        assert ac == 1
        assert carry == 0

    def test_no_aux_carry(self):
        # 0x01 + 0x01 = 0x02, no carry from bit 3
        _, _, ac = add_8bit(0x01, 0x01, 0)
        assert ac == 0

    def test_max_plus_max(self):
        result, carry, ac = add_8bit(0xFF, 0xFF, 0)
        assert result == 0xFE
        assert carry == 1

    def test_bcd_addition(self):
        # 0x29 + 0x47 = 0x70, no AC needed for low nibble (9+7=16>9 but that's ok)
        result, carry, ac = add_8bit(0x29, 0x47, 0)
        assert result == 0x70

    def test_carry_in_increments(self):
        result, _, _ = add_8bit(0x05, 0x05, 1)
        assert result == 11


class TestAdd16bit:
    def test_zero_plus_zero(self):
        result, carry = add_16bit(0, 0, 0)
        assert result == 0
        assert carry == 0

    def test_basic(self):
        result, carry = add_16bit(0x100, 0x200, 0)
        assert result == 0x300
        assert carry == 0

    def test_overflow(self):
        result, carry = add_16bit(0xFFFF, 1, 0)
        assert result == 0
        assert carry == 1

    def test_carry_in(self):
        result, carry = add_16bit(0xFFFF, 0, 1)
        assert result == 0
        assert carry == 1

    def test_pc_increment(self):
        # Typical use: incrementing PC by 1
        result, _ = add_16bit(0x1234, 1, 0)
        assert result == 0x1235

    def test_max_plus_max(self):
        result, carry = add_16bit(0xFFFF, 0xFFFF, 0)
        assert result == 0xFFFE
        assert carry == 1


class TestInvert8bit:
    def test_zero_becomes_max(self):
        assert invert_8bit(0x00) == 0xFF

    def test_max_becomes_zero(self):
        assert invert_8bit(0xFF) == 0x00

    def test_alternating(self):
        # 0xAA = 0b10101010 → invert → 0b01010101 = 0x55
        assert invert_8bit(0xAA) == 0x55
        assert invert_8bit(0x55) == 0xAA

    def test_roundtrip(self):
        for v in range(256):
            assert invert_8bit(invert_8bit(v)) == v

    def test_specific(self):
        # 0xF0 = 0b11110000 → 0b00001111 = 0x0F
        assert invert_8bit(0xF0) == 0x0F


class TestComputeParity:
    def test_zero_has_even_parity(self):
        # 0 ones → even → parity bit = 0
        assert compute_parity([0, 0, 0, 0, 0, 0, 0, 0]) == 0

    def test_one_bit_has_odd_parity(self):
        # 1 one → odd → parity bit = 1
        assert compute_parity([1, 0, 0, 0, 0, 0, 0, 0]) == 1

    def test_two_bits_have_even_parity(self):
        assert compute_parity([1, 1, 0, 0, 0, 0, 0, 0]) == 0

    def test_all_ones_has_even_parity(self):
        # 8 ones → even → parity bit = 0
        assert compute_parity([1, 1, 1, 1, 1, 1, 1, 1]) == 0

    def test_seven_ones_has_odd_parity(self):
        assert compute_parity([1, 1, 1, 1, 1, 1, 1, 0]) == 1

    def test_empty_list(self):
        assert compute_parity([]) == 0

    def test_known_value_0x96(self):
        # 0x96 = 0b10010110 → 4 ones → even parity → P = 0
        bits = int_to_bits(0x96, 8)
        assert compute_parity(bits) == 0

    def test_known_value_0x01(self):
        # 0x01 = 0b00000001 → 1 one → odd parity → P = 1
        bits = int_to_bits(0x01, 8)
        assert compute_parity(bits) == 1


class TestComputeZero:
    def test_all_zeros_returns_one(self):
        assert compute_zero([0, 0, 0, 0, 0, 0, 0, 0]) == 1

    def test_any_one_returns_zero(self):
        assert compute_zero([1, 0, 0, 0, 0, 0, 0, 0]) == 0
        assert compute_zero([0, 0, 0, 0, 0, 0, 0, 1]) == 0

    def test_all_ones_returns_zero(self):
        assert compute_zero([1, 1, 1, 1, 1, 1, 1, 1]) == 0

    def test_single_zero(self):
        assert compute_zero([0]) == 1

    def test_single_one(self):
        assert compute_zero([1]) == 0

    def test_empty(self):
        assert compute_zero([]) == 1
