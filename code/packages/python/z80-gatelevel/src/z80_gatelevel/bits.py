"""Bit conversion helpers — the bridge between integers and gate-level bits.

=== Why this module exists ===

Gate functions (AND, OR, XOR, etc.) operate on individual bits — the integers
0 and 1. Adders operate on lists of bits. The outside world and the Z80State
dataclass use plain Python integers. This module bridges the two worlds.

=== Bit ordering: LSB first ===

All bit lists are LSB-first (little-endian), matching the `logic-gates` and
`arithmetic` packages. Index 0 is the least significant bit.

    int_to_bits(5, 8)  →  [1, 0, 1, 0, 0, 0, 0, 0]
    #                       ↑ bit0 = 1 (×1)
    #                         ↑ bit1 = 0 (×2)
    #                           ↑ bit2 = 1 (×4)
    # Sum: 1 + 4 = 5 ✓

This convention maps naturally to the ripple-carry adder chain: the carry
from bit N propagates to bit N+1.

=== 8-bit vs 16-bit ===

The Z80 has:
  - 8-bit data bus:   A, B, C, D, E, H, L registers → use width=8
  - 16-bit address bus: PC, SP, HL pair, IX, IY → use width=16

The Z80 also has alternate register bank (A', F', B', C', D', E', H', L').
These are also stored as 8-bit arrays.

The add_16bit() function wraps ripple_carry_adder for 16-bit arithmetic
(INC/DEC register pairs, ADD HL, rp, ADC HL, rp, SBC HL, rp).

=== Half-carry in Z80 vs Intel 8080 ===

The Z80 half-carry flag (H) has the same meaning as the 8080's AC flag
(auxiliary carry): it records the carry out of bit 3 into bit 4.

For subtraction, H records the borrow from bit 4: H=1 means a borrow
occurred from the high nibble into the low nibble.

=== Z80 parity flag ===

The Z80 uses flag P/V for dual purposes:
  - Logical ops (AND/OR/XOR): P = even parity of result (XOR-tree of 8 bits)
  - Arithmetic ops (ADD/SUB/INC/DEC): P/V = overflow (two's complement)

This module provides compute_parity() for the parity case.
Overflow is computed by the ALU from the carry bits.
"""

from __future__ import annotations

from arithmetic import ripple_carry_adder
from logic_gates import NOT, XOR_N


def int_to_bits(value: int, width: int) -> list[int]:
    """Convert a non-negative integer to a list of bits, LSB first.

    The value is masked to `width` bits before conversion, so you can
    safely pass values that overflow (e.g. int_to_bits(0x1FF, 8) → 0xFF).

    Args:
        value: Integer to convert. Masked to `width` bits.
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


def compute_parity(bits: list[int]) -> int:
    """Compute even parity using an XOR gate tree.

    The Z80's P/V flag is 1 when the result has an EVEN number of 1-bits
    (even parity). This is computed by XOR-ing all bits together — if the
    count of 1-bits is even, the XOR folds to 0 — then inverting.

    Hardware: 7 XOR gates arranged in a balanced tree + 1 NOT gate.
    Three gate delays for an 8-input tree.

    Args:
        bits: List of bits (typically 8, the ALU result).

    Returns:
        1 if even parity (P=1), 0 if odd parity (P=0).

    Examples:
        >>> compute_parity([0] * 8)   # 0 ones → even
        1
        >>> compute_parity([1, 0, 0, 0, 0, 0, 0, 0])  # 1 one → odd
        0
        >>> compute_parity([1, 1, 0, 0, 0, 0, 0, 0])  # 2 ones → even
        1
    """
    if not bits:
        return 1  # vacuously even
    xor_result = XOR_N(*bits)
    return NOT(xor_result)


def compute_zero(bits: list[int]) -> int:
    """Zero detection via a NOR gate tree.

    The Z80's Z flag is 1 when ALL result bits are 0. Hardware implements
    this as a balanced NOR tree: three stages for 8 bits.

    Stage 1: NOR(b0,b1), NOR(b2,b3), NOR(b4,b5), NOR(b6,b7)  — 4 NOR
    Stage 2: AND(stage1[0], stage1[1]), AND(stage1[2], stage1[3])  — 2 AND
    Stage 3: AND(stage2[0], stage2[1])  — 1 AND

    (Equivalent to NOR over all 8 bits, just tree-structured for speed.)

    Args:
        bits: List of bits (typically 8 or 16).

    Returns:
        1 if all bits are 0 (Z=1), 0 if any bit is 1 (Z=0).

    Examples:
        >>> compute_zero([0, 0, 0, 0, 0, 0, 0, 0])
        1
        >>> compute_zero([1, 0, 0, 0, 0, 0, 0, 0])
        0
    """
    # NOR tree: 1 only when all inputs are 0
    return 1 if all(b == 0 for b in bits) else 0


def add_8bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two 8-bit values through the ripple-carry adder gate chain.

    Converts integers to bit lists, runs through ripple_carry_adder (the
    full gate chain: 8 full-adder stages), converts back.

    Args:
        a:        First 8-bit operand (0–255).
        b:        Second 8-bit operand (0–255).
        carry_in: Initial carry bit (0 or 1, default 0).

    Returns:
        (result, carry_out, half_carry) where:
        - result     = 8-bit sum (0–255), wrapped on overflow
        - carry_out  = 1 if sum exceeded 255 (carry out of bit 7)
        - half_carry = carry out of bit 3 (used by H flag and DAA)

    Examples:
        >>> add_8bit(10, 5)
        (15, 0, 0)
        >>> add_8bit(0xFF, 1)
        (0, 1, 1)   # overflow: 256 → 0, carry=1
        >>> add_8bit(0x0F, 0x01)
        (16, 0, 1)  # half carry: low nibble overflowed
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)

    # Full 8-bit ripple-carry adder
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, carry_in)

    # Half-carry: the carry out of bit 3 (into bit 4)
    # Re-add just the low 4 bits to compute this carry precisely
    low_a = bits_a[:4]
    low_b = bits_b[:4]
    _, hc = ripple_carry_adder(low_a, low_b, carry_in)

    return bits_to_int(sum_bits), cout, hc


def add_16bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two 16-bit values through the ripple-carry adder gate chain.

    Used for ADD HL,rp, ADC HL,rp, PC increment, SP ±1/±2 operations.
    Routes through 16 full-adder stages — twice the propagation delay of
    the 8-bit adder, reflecting real Z80 timing.

    The Z80's 16-bit operations set H based on carry from bit 11 (the half-
    carry of the high byte), unlike 8-bit where it's bit 3.

    Args:
        a:        First 16-bit operand (0–65535).
        b:        Second 16-bit operand (0–65535).
        carry_in: Initial carry (default 0).

    Returns:
        (result, carry_out, half_carry_16) where:
        - result        = 16-bit sum (masked to 0–65535)
        - carry_out     = 1 if sum exceeded 65535
        - half_carry_16 = carry out of bit 11 (for H flag in 16-bit ops)

    Examples:
        >>> add_16bit(0x1234, 0x0001)
        (0x1235, 0, 0)
        >>> add_16bit(0xFFFF, 0x0001)
        (0, 1, 1)   # 16-bit overflow
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, carry_in)

    # Half-carry for 16-bit: carry out of bit 11
    low12_a = bits_a[:12]
    low12_b = bits_b[:12]
    _, hc16 = ripple_carry_adder(low12_a, low12_b, carry_in)

    return bits_to_int(sum_bits), cout, hc16


def invert_8bit(value: int) -> int:
    """Bitwise NOT of an 8-bit value through NOT gate chain.

    8 NOT gates in parallel (one per bit). Used for two's complement
    subtraction: SUB implements A + NOT(B) + 1.

    In the Z80 (as in the 8080), subtraction is synthesized from addition:
        A - B = A + NOT(B) + 1

    This is why the Z80 Z80 manual says the half-carry flag H is the
    COMPLEMENT of what you'd expect from a subtractor.

    Args:
        value: 8-bit integer (0–255).

    Returns:
        Bitwise NOT, masked to 8 bits.

    Examples:
        >>> invert_8bit(0xAA)
        85   # 0x55
        >>> invert_8bit(0)
        255
    """
    bits = int_to_bits(value, 8)
    return bits_to_int([NOT(b) for b in bits])


def invert_16bit(value: int) -> int:
    """Bitwise NOT of a 16-bit value through NOT gate chain.

    16 NOT gates in parallel. Used for 16-bit subtraction.

    Args:
        value: 16-bit integer (0–65535).

    Returns:
        Bitwise NOT, masked to 16 bits.
    """
    bits = int_to_bits(value, 16)
    return bits_to_int([NOT(b) for b in bits])
