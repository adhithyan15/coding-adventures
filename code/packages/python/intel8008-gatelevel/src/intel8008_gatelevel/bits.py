"""Bit conversion helpers — the bridge between integers and gate-level bits.

=== Why this module exists ===

The gate-level simulator operates on individual bits (lists of 0s and 1s),
because that's what real hardware does. A transistor is either conducting
(1) or blocking (0). Logic gates take bits and produce bits. Adders chain
bits. So the gate-level simulator works entirely in bit lists.

But the outside world (programs, tests, the behavioral simulator) works with
integers. This module converts between the two representations.

=== Bit ordering: LSB first ===

All bit lists use LSB-first (little-endian) ordering, matching the conventions
of the `logic-gates` and `arithmetic` packages. Index 0 is the least
significant bit.

    int_to_bits(5, width=8)  →  [1, 0, 1, 0, 0, 0, 0, 0]
    #                            ↑ bit0 = 1 (× 1)
    #                              ↑ bit1 = 0 (× 2)
    #                                ↑ bit2 = 1 (× 4)
    #                                  ...
    # Sum: 1 + 0 + 4 = 5 ✓

This convention is used throughout the computing stack because it maps
naturally to ripple-carry adder chains: bit 0 enters the first full adder,
its carry propagates to the next, and so on up to bit 7 for an 8-bit value.

=== 14-bit values for PC and stack ===

The Intel 8008's program counter and stack entries are 14-bit values
(range 0x0000–0x3FFF). Use width=14 to convert these.
"""

from __future__ import annotations

from logic_gates import NOT, XOR_N


def int_to_bits(value: int, width: int) -> list[int]:
    """Convert an integer to a list of bits (LSB first).

    Args:
        value: Non-negative integer to convert. Will be masked to `width` bits.
        width: Number of bits in the output list.

    Returns:
        List of 0s and 1s, length = width, LSB at index 0.

    Examples:
        >>> int_to_bits(5, 8)
        [1, 0, 1, 0, 0, 0, 0, 0]
        >>> int_to_bits(0xFF, 8)
        [1, 1, 1, 1, 1, 1, 1, 1]
        >>> int_to_bits(0x100, 14)  # 14-bit PC value
        [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]
    """
    value = value & ((1 << width) - 1)
    return [(value >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert a list of bits (LSB first) to an integer.

    Args:
        bits: List of 0s and 1s, LSB at index 0.

    Returns:
        Non-negative integer. For an 8-bit list, range is 0–255.
        For a 14-bit list, range is 0–16383.

    Examples:
        >>> bits_to_int([1, 0, 1, 0, 0, 0, 0, 0])
        5
        >>> bits_to_int([1, 1, 1, 1, 1, 1, 1, 1])
        255
    """
    result = 0
    for i, bit in enumerate(bits):
        result |= bit << i
    return result


def compute_parity(bits: list[int]) -> int:
    """Compute even parity via XOR reduction using real logic gates.

    The Intel 8008's P (parity) flag is set when the result has an EVEN
    number of 1-bits. P=1 means even parity; P=0 means odd parity.

    Hardware implementation for 8 bits uses a balanced XOR tree:
        level 1: XOR(b0,b1), XOR(b2,b3), XOR(b4,b5), XOR(b6,b7)  — 4 gates
        level 2: XOR(^01, ^23), XOR(^45, ^67)                     — 2 gates
        level 3: XOR(^^0123, ^^4567)                               — 1 gate
        output:  NOT(level3_result)                                — 1 gate
    Total: 8 XOR gates (via XOR_N chain) + 1 NOT = ~8 gates.

    The XOR reduction tells us if there's an ODD number of 1-bits. We
    invert it to get the EVEN parity flag (P=1 when even, P=0 when odd).

    Args:
        bits: List of 0s and 1s (typically 8 bits for an ALU result).

    Returns:
        1 if even parity (even number of 1-bits, P flag = 1 in 8008).
        0 if odd parity (odd number of 1-bits, P flag = 0 in 8008).

    Examples:
        >>> compute_parity([0, 0, 0, 0, 0, 0, 0, 0])  # 0 ones: even
        1
        >>> compute_parity([1, 0, 0, 0, 0, 0, 0, 0])  # 1 one: odd
        0
        >>> compute_parity([1, 1, 0, 0, 0, 0, 0, 0])  # 2 ones: even
        1
        >>> compute_parity([1, 1, 1, 1, 1, 1, 1, 1])  # 8 ones: even
        1
    """
    if not bits:
        return 1  # zero ones = even parity
    # XOR_N returns 1 if ODD number of 1-bits, 0 if EVEN
    xor_result = XOR_N(*bits)
    # Invert: P=1 means EVEN parity
    return NOT(xor_result)
