"""Bit conversion helpers — the bridge between integers and gate-level bits.

=== Why this module exists ===

Gate functions (AND, OR, XOR, etc.) operate on individual bits — the integers
0 and 1.  Adders operate on lists of bits.  The outside world and MOS6502State
use plain Python integers.  This module bridges the two worlds.

=== Bit ordering: LSB first ===

All bit lists are LSB-first (little-endian), matching the ``logic-gates`` and
``arithmetic`` packages.  Index 0 is the least significant bit.

    int_to_bits(5, 8)  →  [1, 0, 1, 0, 0, 0, 0, 0]
    #                       ↑ bit0 = 1 (×1)
    #                         ↑ bit1 = 0 (×2)
    #                           ↑ bit2 = 1 (×4)
    # Sum: 1 + 4 = 5 ✓

This convention maps naturally to the ripple-carry adder chain: the carry
from bit N propagates to bit N+1.

=== 8-bit vs 16-bit ===

The 6502 has:
  - 8-bit data bus:   A, X, Y, S registers → use width=8
  - 16-bit address bus: PC → use width=16

The add_16bit() function wraps ripple_carry_adder for 16-bit address arithmetic
(PC increment, stack pointer effective address computation).

=== No half-carry on the 6502 ===

Unlike the Intel 8080 or Z80, the 6502 has NO half-carry (auxiliary carry)
flag.  DAA-style BCD correction on the 6502 is therefore more manual — the
simulator's daa_adc/daa_sbc helpers implement the NMOS BCD algorithm directly
by checking nibble overflow using gate primitives.

=== Zero detection ===

The Z flag is 1 when ALL result bits are 0.  Hardware implements this as a
balanced NOR tree: three stages for 8 bits.

Stage 1: OR(b0,b1), OR(b2,b3), OR(b4,b5), OR(b6,b7)   — 4 OR gates
Stage 2: OR(stage1[0], stage1[1]), OR(stage1[2], stage1[3]) — 2 OR gates
Stage 3: NOR(stage2[0], stage2[1]) → NOT(OR(…)) = 1 iff all zero
"""

from __future__ import annotations

from arithmetic import ripple_carry_adder
from logic_gates import NOT


def int_to_bits(value: int, width: int) -> list[int]:
    """Convert a non-negative integer to a list of bits, LSB first.

    The value is masked to ``width`` bits before conversion, so you can
    safely pass values that overflow (e.g. int_to_bits(0x1FF, 8) → 0xFF).

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
    """
    value = value & ((1 << width) - 1)
    return [(value >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert a list of bits (LSB first) to a non-negative integer.

    Args:
        bits: List of 0/1 ints, index 0 = LSB.

    Returns:
        Non-negative integer. For width=8: range 0–255. Width=16: 0–65535.

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


def compute_zero(bits: list[int]) -> int:
    """Zero detection via a NOR gate tree.

    The 6502's Z flag is 1 when ALL result bits are 0.  Hardware implements
    this as a balanced NOR/OR tree: three stages for 8 bits.

    Stage 1: 4 OR gates — OR pairs of adjacent bits
    Stage 2: 2 OR gates — OR pairs of stage-1 results
    Stage 3: 1 NOT gate — invert the final OR (NOT(any_set) = all_zero)

    This is equivalent to NOR over all 8 bits, just tree-structured for speed.

    Args:
        bits: List of bits (typically 8).

    Returns:
        1 if all bits are 0 (Z=1), 0 if any bit is 1 (Z=0).

    Examples:
        >>> compute_zero([0, 0, 0, 0, 0, 0, 0, 0])
        1
        >>> compute_zero([1, 0, 0, 0, 0, 0, 0, 0])
        0
    """
    return 1 if all(b == 0 for b in bits) else 0


def add_8bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int]:
    """Add two 8-bit values through the ripple-carry adder gate chain.

    Converts integers to bit lists, runs through ripple_carry_adder (the
    full gate chain: 8 full-adder stages), converts back.

    The 6502 has no half-carry flag, so we only return (result, carry_out).

    Args:
        a:        First 8-bit operand (0–255).
        b:        Second 8-bit operand (0–255).
        carry_in: Initial carry bit (0 or 1, default 0).

    Returns:
        (result, carry_out) where:
        - result     = 8-bit sum (0–255), wrapped on overflow
        - carry_out  = 1 if sum exceeded 255 (carry out of bit 7)

    Examples:
        >>> add_8bit(10, 5)
        (15, 0)
        >>> add_8bit(0xFF, 1)
        (0, 1)
        >>> add_8bit(0x7F, 0x01)
        (128, 0)
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, carry_in)
    return bits_to_int(sum_bits), cout


def add_16bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int]:
    """Add two 16-bit values through the ripple-carry adder gate chain.

    Used for PC increment, stack effective-address computation, and 16-bit
    address arithmetic in indexed addressing modes.

    Routes through 16 full-adder stages — twice the propagation delay of
    the 8-bit adder, reflecting real 6502 timing.

    Args:
        a:        First 16-bit operand (0–65535).
        b:        Second 16-bit operand (0–65535).
        carry_in: Initial carry (default 0).

    Returns:
        (result, carry_out) where:
        - result    = 16-bit sum (masked to 0–65535)
        - carry_out = 1 if sum exceeded 65535

    Examples:
        >>> add_16bit(0x1234, 0x0001)
        (0x1235, 0)
        >>> add_16bit(0xFFFF, 0x0001)
        (0, 1)
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, carry_in)
    return bits_to_int(sum_bits), cout


def invert_8bit(value: int) -> int:
    """Bitwise NOT of an 8-bit value through NOT gate chain.

    8 NOT gates in parallel (one per bit).  Used for two's complement
    subtraction: SBC implements A + NOT(M) + C.

    The 6502 SBC uses the carry flag as "inverted borrow":
        A - M = A + NOT(M) + 1   (when C=1, meaning no borrow)
        A - M - 1 = A + NOT(M) + 0   (when C=0, meaning borrow-in)

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
