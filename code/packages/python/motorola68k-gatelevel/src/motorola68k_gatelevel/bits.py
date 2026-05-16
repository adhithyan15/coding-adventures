"""Bit conversion helpers for the Motorola 68000 gate-level simulator.

=== Why this module exists ===

Gate functions (AND, OR, XOR, NOT) operate on individual bits — integers 0 and 1.
Adders operate on lists of bits.  The outside world uses plain Python integers.
This module bridges the two worlds.

=== Bit ordering: LSB first ===

All bit lists are LSB-first (little-endian), matching the ``logic-gates`` and
``arithmetic`` packages.  Index 0 is the least significant bit.

    int_to_bits(5, 8)  →  [1, 0, 1, 0, 0, 0, 0, 0]
    #                       ↑ bit0 = 1 (×1)
    #                         ↑ bit1 = 0 (×2)
    #                           ↑ bit2 = 1 (×4)
    # Sum: 1 + 4 = 5 ✓

=== 8-bit vs 16-bit vs 32-bit ===

The Motorola 68000 has:
  - 8-bit (byte) data ops  → width=8
  - 16-bit (word) data ops → width=16
  - 32-bit (long) data ops → width=32

=== Zero detection ===

ZF = 1 when ALL result bits are 0.  Hardware: a balanced NOR tree.

    Stage 1: OR pairs  → half as many outputs
    Stage 2: OR pairs of stage-1 results
    ...
    Final: NOT of final OR  (1 iff all zero)

=== Parity detection ===

PF = 1 when the low 8 bits of the result contain an even number of 1s.
Hardware: XOR tree over bits 0–7.  (68k does not use PF, but the helper
is included for completeness.)
"""

from __future__ import annotations

from arithmetic import ripple_carry_adder
from logic_gates import NOT, XOR


def int_to_bits(value: int, width: int) -> list[int]:
    """Convert a non-negative integer to a list of bits, LSB first.

    The value is masked to ``width`` bits before conversion, so you can
    safely pass values that overflow (e.g. int_to_bits(0x1FFFF, 16) → 0xFFFF).

    Args:
        value: Integer to convert. Masked to ``width`` bits.
        width: Number of output bits.

    Returns:
        List of 0/1 ints, length = width, index 0 = LSB.

    Examples:
        >>> int_to_bits(5, 8)
        [1, 0, 1, 0, 0, 0, 0, 0]
        >>> int_to_bits(0xFF, 8)
        [1, 1, 1, 1, 1, 1, 1, 1]
        >>> int_to_bits(0x0100, 16)
        [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]
        >>> int_to_bits(0x80000000, 32)[31]
        1
    """
    value = value & ((1 << width) - 1)
    return [(value >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert a list of bits (LSB first) to a non-negative integer.

    Args:
        bits: List of 0/1 ints, index 0 = LSB.

    Returns:
        Non-negative integer. For width=8: 0–255. Width=16: 0–65535.
        Width=32: 0–4294967295.

    Examples:
        >>> bits_to_int([1, 0, 1, 0, 0, 0, 0, 0])
        5
        >>> bits_to_int([1, 1, 1, 1, 1, 1, 1, 1])
        255
        >>> bits_to_int([0] * 32)
        0
    """
    result = 0
    for i, bit in enumerate(bits):
        result |= bit << i
    return result


def add_8bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two 8-bit values through the ripple-carry adder gate chain.

    Routes through 8 full-adder stages.  Returns the auxiliary carry
    (carry out of bit 3) as the third tuple element.  The 68000 doesn't
    use auxiliary carry, but it's returned for completeness.

    Args:
        a:        First 8-bit operand (0–255).
        b:        Second 8-bit operand (0–255).
        carry_in: Initial carry bit (0 or 1, default 0).

    Returns:
        (result, carry_out, aux_carry) where:
        - result     = 8-bit sum (0–255), wrapped on overflow
        - carry_out  = 1 if sum exceeded 255
        - aux_carry  = 1 if carry out of bit 3

    Examples:
        >>> add_8bit(10, 5)
        (15, 0, 0)
        >>> add_8bit(0xFF, 1)
        (0, 1, 1)
        >>> add_8bit(0x0F, 0x01)
        (16, 0, 1)
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, carry_in)
    # Aux carry = carry out of bit 3 (into bit 4).
    _, cout3 = ripple_carry_adder(bits_a[:4], bits_b[:4], carry_in)
    return bits_to_int(sum_bits), cout, cout3


def add_16bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two 16-bit values through the ripple-carry adder gate chain.

    Routes through 16 full-adder stages.

    Args:
        a:        First 16-bit operand (0–65535).
        b:        Second 16-bit operand (0–65535).
        carry_in: Initial carry (default 0).

    Returns:
        (result, carry_out, aux_carry) where:
        - result    = 16-bit sum (masked to 0–65535)
        - carry_out = 1 if sum exceeded 65535
        - aux_carry = 1 if carry out of bit 3

    Examples:
        >>> add_16bit(0x1234, 0x0001)
        (4661, 0, 0)
        >>> add_16bit(0xFFFF, 0x0001)
        (0, 1, 1)
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, carry_in)
    _, cout3 = ripple_carry_adder(bits_a[:4], bits_b[:4], carry_in)
    return bits_to_int(sum_bits), cout, cout3


def add_32bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int]:
    """Add two 32-bit values through the ripple-carry adder gate chain.

    The primary adder for the 68000.  Routes through 32 full-adder stages.

    Args:
        a:        First 32-bit operand (0–0xFFFFFFFF).
        b:        Second 32-bit operand (0–0xFFFFFFFF).
        carry_in: Initial carry (default 0).

    Returns:
        (result, carry_out) where:
        - result    = 32-bit sum (masked to 0–0xFFFFFFFF)
        - carry_out = 1 if sum exceeded 0xFFFFFFFF

    Examples:
        >>> add_32bit(5, 3)
        (8, 0)
        >>> add_32bit(0xFFFFFFFF, 1)
        (0, 1)
        >>> add_32bit(0x7FFFFFFF, 1)
        (2147483648, 0)
    """
    bits_a = int_to_bits(a, 32)
    bits_b = int_to_bits(b, 32)
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, carry_in)
    return bits_to_int(sum_bits), cout


def invert_8bit(value: int) -> int:
    """Bitwise NOT of an 8-bit value through NOT gate chain.

    8 NOT gates in parallel.  Used for two's-complement subtraction:
    SUB implements A + NOT(B) + 1.

    Args:
        value: 8-bit integer (0–255).

    Returns:
        Bitwise NOT, masked to 8 bits.

    Examples:
        >>> invert_8bit(0xAA)
        85
        >>> invert_8bit(0)
        255
        >>> invert_8bit(0xFF)
        0
    """
    bits = int_to_bits(value, 8)
    return bits_to_int([NOT(b) for b in bits])


def invert_16bit(value: int) -> int:
    """Bitwise NOT of a 16-bit value through 16 NOT gates.

    Args:
        value: 16-bit integer (0–65535).

    Returns:
        Bitwise NOT, masked to 16 bits.

    Examples:
        >>> invert_16bit(0xAAAA)
        21845
        >>> invert_16bit(0)
        65535
        >>> invert_16bit(0xFFFF)
        0
    """
    bits = int_to_bits(value, 16)
    return bits_to_int([NOT(b) for b in bits])


def invert_32bit(value: int) -> int:
    """Bitwise NOT of a 32-bit value through 32 NOT gates.

    32 NOT gates in parallel.  Used for 32-bit SUB/NEG via two's complement.

    Args:
        value: 32-bit integer (0–0xFFFFFFFF).

    Returns:
        Bitwise NOT, masked to 32 bits.

    Examples:
        >>> invert_32bit(0)
        4294967295
        >>> invert_32bit(0xFFFFFFFF)
        0
        >>> invert_32bit(0xAAAAAAAA)
        1431655765
    """
    bits = int_to_bits(value, 32)
    return bits_to_int([NOT(b) for b in bits])


def compute_parity(bits: list[int]) -> int:
    """Parity detection via XOR tree over the low 8 bits.

    PF = 1 when the low 8 bits of the result contain an even number of 1-bits.
    The 68000 doesn't have a parity flag, but this helper is kept for
    completeness and to match the interface expected by tests.

    Hardware implementation: a balanced XOR tree over 8 bits.

    Args:
        bits: List of bits (at least 8).  Only bits[0:8] are used.

    Returns:
        1 if even parity (even number of 1s in low 8 bits), 0 if odd.

    Examples:
        >>> compute_parity([1,0,0,0, 0,0,0,0])
        0
        >>> compute_parity([1,1,0,0, 0,0,0,0])
        1
        >>> compute_parity([0,0,0,0, 0,0,0,0])
        1
    """
    low8 = bits[:8]
    s0 = XOR(low8[0], low8[1])
    s1 = XOR(low8[2], low8[3])
    s2 = XOR(low8[4], low8[5])
    s3 = XOR(low8[6], low8[7])
    t0 = XOR(s0, s1)
    t1 = XOR(s2, s3)
    parity_odd = XOR(t0, t1)
    return NOT(parity_odd)  # 1 means even parity


def compute_zero(bits: list[int]) -> int:
    """Zero detection via NOR tree.

    ZF = 1 when ALL result bits are 0.  Hardware: OR pairs, OR the ORs,
    NOT the final result.

    Args:
        bits: List of bits (any length).

    Returns:
        1 if all bits are 0 (ZF=1), 0 if any bit is 1 (ZF=0).

    Examples:
        >>> compute_zero([0, 0, 0, 0, 0, 0, 0, 0])
        1
        >>> compute_zero([1, 0, 0, 0, 0, 0, 0, 0])
        0
        >>> compute_zero([0]*32)
        1
        >>> compute_zero([0]*31 + [1])
        0
    """
    return 1 if all(b == 0 for b in bits) else 0
