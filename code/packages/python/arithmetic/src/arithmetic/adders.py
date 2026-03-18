"""Adder circuits built from logic gates.

Half adder: adds two bits.
Full adder: adds two bits + carry-in.
Ripple carry adder: chains full adders for N-bit addition.
"""

from logic_gates import AND, OR, XOR


def half_adder(a: int, b: int) -> tuple[int, int]:
    """Add two single bits.

    Returns (sum, carry).

    Truth table:
        0 + 0 = 0, carry 0
        0 + 1 = 1, carry 0
        1 + 0 = 1, carry 0
        1 + 1 = 0, carry 1  (1+1 = 10 in binary)
    """
    sum_bit = XOR(a, b)
    carry = AND(a, b)
    return sum_bit, carry


def full_adder(a: int, b: int, carry_in: int) -> tuple[int, int]:
    """Add two bits plus a carry-in from a previous addition.

    Returns (sum, carry_out).

    Built from two half adders and an OR gate:
        1. Half-add a and b → partial_sum, partial_carry
        2. Half-add partial_sum and carry_in → sum, carry2
        3. carry_out = OR(partial_carry, carry2)
    """
    partial_sum, partial_carry = half_adder(a, b)
    sum_bit, carry2 = half_adder(partial_sum, carry_in)
    carry_out = OR(partial_carry, carry2)
    return sum_bit, carry_out


def ripple_carry_adder(
    a: list[int], b: list[int], carry_in: int = 0
) -> tuple[list[int], int]:
    """Add two N-bit numbers using a chain of full adders.

    Args:
        a: First number as list of bits, LSB first (index 0 = least significant).
        b: Second number as list of bits, LSB first.
        carry_in: Initial carry (default 0).

    Returns:
        (sum_bits, carry_out) where sum_bits is LSB first.

    Example:
        5 + 3 = 8
        a = [1, 0, 1, 0]  # 5 in binary (LSB first: 1*1 + 0*2 + 1*4 + 0*8)
        b = [1, 1, 0, 0]  # 3 in binary (LSB first: 1*1 + 1*2 + 0*4 + 0*8)
        result = [0, 0, 0, 1], carry=0  # 8 in binary
    """
    if len(a) != len(b):
        msg = f"a and b must have the same length, got {len(a)} and {len(b)}"
        raise ValueError(msg)
    if len(a) == 0:
        msg = "bit lists must not be empty"
        raise ValueError(msg)

    sum_bits: list[int] = []
    carry = carry_in

    for i in range(len(a)):
        sum_bit, carry = full_adder(a[i], b[i], carry)
        sum_bits.append(sum_bit)

    return sum_bits, carry
