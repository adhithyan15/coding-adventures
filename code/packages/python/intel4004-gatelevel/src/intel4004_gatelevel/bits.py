"""Bit conversion helpers — the bridge between integers and gate-level bits.

=== Why this module exists ===

The gate-level simulator operates on individual bits (lists of 0s and 1s),
because that's what real hardware does. But the outside world (test programs,
the behavioral simulator) works with integers. This module converts between
the two representations.

=== Bit ordering: LSB first ===

All bit lists use LSB-first ordering, matching the logic-gates and arithmetic
packages. Index 0 is the least significant bit.

    int_to_bits(5, width=4)  →  [1, 0, 1, 0]
    #                           bit0=1(×1) + bit1=0(×2) + bit2=1(×4) + bit3=0(×8) = 5

This convention is used throughout the computing stack because it maps
naturally to how adders chain: bit 0 feeds the first full adder, bit 1
feeds the second, and so on.
"""

from __future__ import annotations


def int_to_bits(value: int, width: int) -> list[int]:
    """Convert an integer to a list of bits (LSB first).

    Args:
        value: Non-negative integer to convert.
        width: Number of bits in the output list.

    Returns:
        List of 0s and 1s, length = width, LSB at index 0.

    Examples:
        >>> int_to_bits(5, 4)
        [1, 0, 1, 0]
        >>> int_to_bits(0, 4)
        [0, 0, 0, 0]
        >>> int_to_bits(15, 4)
        [1, 1, 1, 1]
        >>> int_to_bits(0xABC, 12)
        [0, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1]
    """
    # Mask to width to handle negative or oversized values
    value = value & ((1 << width) - 1)
    return [(value >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert a list of bits (LSB first) to an integer.

    Args:
        bits: List of 0s and 1s, LSB at index 0.

    Returns:
        Non-negative integer.

    Examples:
        >>> bits_to_int([1, 0, 1, 0])
        5
        >>> bits_to_int([0, 0, 0, 0])
        0
        >>> bits_to_int([1, 1, 1, 1])
        15
    """
    result = 0
    for i, bit in enumerate(bits):
        result |= bit << i
    return result
