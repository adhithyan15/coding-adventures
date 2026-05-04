"""Tests for bit conversion helpers."""

from __future__ import annotations

from intel8080_gatelevel.bits import (
    add_8bit,
    add_16bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_8bit,
)


class TestIntToBits:
    def test_zero(self) -> None:
        assert int_to_bits(0, 8) == [0] * 8

    def test_one(self) -> None:
        assert int_to_bits(1, 8) == [1, 0, 0, 0, 0, 0, 0, 0]

    def test_five(self) -> None:
        assert int_to_bits(5, 8) == [1, 0, 1, 0, 0, 0, 0, 0]

    def test_max_8bit(self) -> None:
        assert int_to_bits(0xFF, 8) == [1] * 8

    def test_16bit_zero(self) -> None:
        assert int_to_bits(0, 16) == [0] * 16

    def test_16bit_value(self) -> None:
        bits = int_to_bits(0x0100, 16)
        assert bits[8] == 1   # bit 8
        assert bits[0] == 0   # bit 0

    def test_overflow_masked(self) -> None:
        # 0x1FF masked to 8 bits = 0xFF
        assert int_to_bits(0x1FF, 8) == [1] * 8


class TestBitsToInt:
    def test_zero(self) -> None:
        assert bits_to_int([0] * 8) == 0

    def test_one(self) -> None:
        assert bits_to_int([1, 0, 0, 0, 0, 0, 0, 0]) == 1

    def test_five(self) -> None:
        assert bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) == 5

    def test_max(self) -> None:
        assert bits_to_int([1] * 8) == 255

    def test_round_trip(self) -> None:
        for v in [0, 1, 42, 127, 128, 255]:
            assert bits_to_int(int_to_bits(v, 8)) == v


class TestComputeParity:
    def test_zero_is_even(self) -> None:
        assert compute_parity([0] * 8) == 1

    def test_one_is_odd(self) -> None:
        assert compute_parity([1, 0, 0, 0, 0, 0, 0, 0]) == 0

    def test_two_ones_even(self) -> None:
        assert compute_parity([1, 1, 0, 0, 0, 0, 0, 0]) == 1

    def test_all_ones_even(self) -> None:
        # 8 ones → even
        assert compute_parity([1] * 8) == 1

    def test_three_ones_odd(self) -> None:
        assert compute_parity([1, 1, 1, 0, 0, 0, 0, 0]) == 0

    def test_0x03_is_even(self) -> None:
        # 0x03 = 0b00000011: 2 ones → even
        assert compute_parity(int_to_bits(0x03, 8)) == 1


class TestComputeZero:
    def test_all_zero(self) -> None:
        assert compute_zero([0] * 8) == 1

    def test_one_set(self) -> None:
        assert compute_zero([1, 0, 0, 0, 0, 0, 0, 0]) == 0

    def test_all_ones(self) -> None:
        assert compute_zero([1] * 8) == 0

    def test_single_msb(self) -> None:
        assert compute_zero([0, 0, 0, 0, 0, 0, 0, 1]) == 0


class TestAdd8bit:
    def test_simple(self) -> None:
        result, cy, ac = add_8bit(10, 5)
        assert result == 15
        assert cy == 0
        assert ac == 0

    def test_overflow(self) -> None:
        result, cy, ac = add_8bit(0xFF, 1)
        assert result == 0
        assert cy == 1

    def test_aux_carry(self) -> None:
        result, cy, ac = add_8bit(0x0F, 0x01)
        assert result == 0x10
        assert ac == 1

    def test_with_carry(self) -> None:
        result, cy, ac = add_8bit(5, 3, carry_in=1)
        assert result == 9


class TestAdd16bit:
    def test_simple(self) -> None:
        result, cy = add_16bit(0x1234, 1)
        assert result == 0x1235
        assert cy == 0

    def test_overflow(self) -> None:
        result, cy = add_16bit(0xFFFF, 1)
        assert result == 0
        assert cy == 1

    def test_pc_increment_by_2(self) -> None:
        result, _ = add_16bit(0x0100, 2)
        assert result == 0x0102

    def test_sp_decrement(self) -> None:
        # SP - 2 = SP + 0xFFFE (two's complement)
        result, _ = add_16bit(0x0200, 0xFFFE)
        assert result == 0x01FE


class TestInvert8bit:
    def test_zero_is_max(self) -> None:
        assert invert_8bit(0) == 0xFF

    def test_ff_is_zero(self) -> None:
        assert invert_8bit(0xFF) == 0

    def test_aa_is_55(self) -> None:
        assert invert_8bit(0xAA) == 0x55

    def test_55_is_aa(self) -> None:
        assert invert_8bit(0x55) == 0xAA

    def test_involution(self) -> None:
        for v in [0, 1, 42, 127, 128, 255]:
            assert invert_8bit(invert_8bit(v)) == v
