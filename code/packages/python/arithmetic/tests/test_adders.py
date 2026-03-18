"""Tests for adder circuits."""

import pytest

from arithmetic import full_adder, half_adder, ripple_carry_adder


def _int_to_bits(n: int, width: int) -> list[int]:
    """Convert integer to LSB-first bit list."""
    return [(n >> i) & 1 for i in range(width)]


def _bits_to_int(bits: list[int]) -> int:
    """Convert LSB-first bit list to integer."""
    return sum(bit << i for i, bit in enumerate(bits))


class TestHalfAdder:
    def test_0_plus_0(self) -> None:
        assert half_adder(0, 0) == (0, 0)

    def test_0_plus_1(self) -> None:
        assert half_adder(0, 1) == (1, 0)

    def test_1_plus_0(self) -> None:
        assert half_adder(1, 0) == (1, 0)

    def test_1_plus_1(self) -> None:
        assert half_adder(1, 1) == (0, 1)


class TestFullAdder:
    def test_0_0_0(self) -> None:
        assert full_adder(0, 0, 0) == (0, 0)

    def test_0_0_1(self) -> None:
        assert full_adder(0, 0, 1) == (1, 0)

    def test_0_1_0(self) -> None:
        assert full_adder(0, 1, 0) == (1, 0)

    def test_0_1_1(self) -> None:
        assert full_adder(0, 1, 1) == (0, 1)

    def test_1_0_0(self) -> None:
        assert full_adder(1, 0, 0) == (1, 0)

    def test_1_0_1(self) -> None:
        assert full_adder(1, 0, 1) == (0, 1)

    def test_1_1_0(self) -> None:
        assert full_adder(1, 1, 0) == (0, 1)

    def test_1_1_1(self) -> None:
        assert full_adder(1, 1, 1) == (1, 1)


class TestRippleCarryAdder:
    def test_0_plus_0(self) -> None:
        a = [0, 0, 0, 0]
        b = [0, 0, 0, 0]
        result, carry = ripple_carry_adder(a, b)
        assert _bits_to_int(result) == 0
        assert carry == 0

    def test_1_plus_2(self) -> None:
        """The target program: x = 1 + 2 = 3."""
        a = _int_to_bits(1, 4)  # [1, 0, 0, 0]
        b = _int_to_bits(2, 4)  # [0, 1, 0, 0]
        result, carry = ripple_carry_adder(a, b)
        assert _bits_to_int(result) == 3
        assert carry == 0

    def test_5_plus_3(self) -> None:
        a = _int_to_bits(5, 4)
        b = _int_to_bits(3, 4)
        result, carry = ripple_carry_adder(a, b)
        assert _bits_to_int(result) == 8
        assert carry == 0

    def test_15_plus_1_overflow(self) -> None:
        """4-bit overflow: 15 + 1 = 16, which doesn't fit in 4 bits."""
        a = _int_to_bits(15, 4)  # [1, 1, 1, 1]
        b = _int_to_bits(1, 4)  # [1, 0, 0, 0]
        result, carry = ripple_carry_adder(a, b)
        assert _bits_to_int(result) == 0  # wraps around
        assert carry == 1

    def test_with_carry_in(self) -> None:
        a = _int_to_bits(1, 4)
        b = _int_to_bits(1, 4)
        result, carry = ripple_carry_adder(a, b, carry_in=1)
        assert _bits_to_int(result) == 3  # 1 + 1 + carry = 3
        assert carry == 0

    def test_8_bit_addition(self) -> None:
        a = _int_to_bits(100, 8)
        b = _int_to_bits(155, 8)
        result, carry = ripple_carry_adder(a, b)
        assert _bits_to_int(result) == 255
        assert carry == 0

    def test_mismatched_lengths(self) -> None:
        with pytest.raises(ValueError, match="same length"):
            ripple_carry_adder([0, 1], [0, 1, 0])

    def test_empty_bits(self) -> None:
        with pytest.raises(ValueError, match="must not be empty"):
            ripple_carry_adder([], [])
