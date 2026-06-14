"""bits.py — Integer ↔ bit-list bridge for the 8051 gate-level simulator.

This module is the *only* place in the gate-level simulator that uses Python
integer arithmetic (+, -, &, |, ^, ~) for pure data-conversion purposes.
Every other module works exclusively with 0/1 bit lists and gate functions.

Why bit lists?
--------------
Real hardware works on individual wires, not Python integers.  An 8-bit adder
is literally 8 wires going in, 8 wires coming out, with 8 full-adder cells
processing one bit pair each.  We represent this as list[int] where each
element is 0 or 1.

LSB-first ordering
------------------
We store bits LSB (least significant bit) first.  This matches how the
ripple_carry_adder from the arithmetic package expects its inputs:
  bit[0] = carry-in for the first full adder (2^0 position)
  bit[7] = carry-in for the eighth full adder (2^7 position)

Example:
  int_to_bits(5, 8) → [1, 0, 1, 0, 0, 0, 0, 0]
  because 5 = 2^0 + 2^2 = 0b00000101

  bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) → 5
"""

from __future__ import annotations

from arithmetic import ripple_carry_adder
from logic_gates import NOT, XOR


def int_to_bits(value: int, width: int) -> list[int]:
    """Convert an integer to an LSB-first bit list of the specified width.

    This is the bridge from the "Python integer world" (program inputs,
    memory addresses, constants) into the "gate world" (bit arrays).

    Args:
        value: The integer to convert (masked to fit in `width` bits).
        width: Number of bits in the output list (e.g., 8 for a byte).

    Returns:
        A list of `width` integers, each 0 or 1, LSB first.

    Example:
        int_to_bits(0b10110010, 8) → [0, 1, 0, 0, 1, 1, 0, 1]
        Position 0 (LSB) = 0, position 7 (MSB) = 1.
    """
    # Mask to prevent negative numbers from causing issues with >> on signed ints
    mask = (1 << width) - 1
    value = value & mask
    return [(value >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert an LSB-first bit list to an unsigned integer.

    The inverse of int_to_bits.  Uses shifting rather than power() so it
    works uniformly for any width without floating-point.

    Args:
        bits: A list of 0s and 1s, LSB first.

    Returns:
        The unsigned integer represented by the bit list.

    Example:
        bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) → 5
    """
    result = 0
    for i, bit in enumerate(bits):
        result |= (bit & 1) << i
    return result


def add_8bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two 8-bit values using a ripple-carry adder (gate-level).

    This routes through the arithmetic package's ripple_carry_adder, which
    in turn calls full_adder → half_adder → XOR/AND gates.  The full chain:

        a, b → int_to_bits (bridge)
             → ripple_carry_adder (8 full adders, ~40 gate calls)
             → bits_to_int (bridge back)

    The auxiliary carry (AC) is the carry out of bit position 3 into position 4.
    On the 8051, AC is used by the Decimal Adjust (DA A) instruction to detect
    BCD overflow in the low nibble.

    Args:
        a:        First operand, 0–255.
        b:        Second operand, 0–255.
        carry_in: Initial carry-in (0 or 1); used for ADDC instruction.

    Returns:
        (result, carry_out, aux_carry) where:
          result    = (a + b + carry_in) mod 256
          carry_out = 1 if (a + b + carry_in) > 255
          aux_carry = carry from bit 3 to bit 4 (for BCD support)
    """
    a_bits = int_to_bits(a, 8)
    b_bits = int_to_bits(b, 8)

    # Full 8-bit addition — carry propagates through all 8 bit positions
    result_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)

    # Auxiliary carry: run a 4-bit add to extract the carry from bit 3→4.
    # This is how the real hardware works: the AC flip-flop is driven by
    # the carry wire between the bit-3 and bit-4 full adders.
    lo_a = a_bits[:4]  # low nibble of a
    lo_b = b_bits[:4]  # low nibble of b
    _, ac = ripple_carry_adder(lo_a, lo_b, carry_in)

    return bits_to_int(result_bits), carry_out, ac


def add_16bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int]:
    """Add two 16-bit values using a ripple-carry adder (gate-level).

    Used for 16-bit PC arithmetic (increment_pc) and DPTR operations.
    The 16-bit adder is 16 full adders in series — twice as deep as the
    8-bit adder but the same gate topology.

    Args:
        a:        First operand, 0–65535.
        b:        Second operand, 0–65535.
        carry_in: Initial carry-in (0 or 1).

    Returns:
        (result, carry_out) where result is (a + b + carry_in) mod 65536.
    """
    a_bits = int_to_bits(a, 16)
    b_bits = int_to_bits(b, 16)
    result_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)
    return bits_to_int(result_bits), carry_out


def invert_8bit(value: int) -> int:
    """Bitwise NOT of an 8-bit value using 8 NOT gates.

    On real hardware, NOT is a single inverting transistor per bit.
    Eight of them in parallel give us 8-bit bitwise complement.

    Used by the gate-level SUBB (subtract with borrow) implementation:
    A - B = A + NOT(B) + 1  (two's complement negation).

    Args:
        value: Integer, 0–255.

    Returns:
        ~value & 0xFF (8-bit bitwise complement).
    """
    bits = int_to_bits(value, 8)
    # Apply NOT gate to each wire independently — 8 inverter gates
    inverted = [NOT(b) for b in bits]
    return bits_to_int(inverted)


def compute_parity(bits: list[int]) -> int:
    """Compute even parity using an XOR gate tree.

    The 8051 PSW.P bit is 1 when ACC has an odd number of '1' bits
    (so that ACC + P always has an even count of '1' bits — even parity).

    Hardware implementation: a binary tree of XOR gates.  For 8 bits:
      Level 1: 4 XOR gates on pairs (bits 0-1, 2-3, 4-5, 6-7)
      Level 2: 2 XOR gates (combining level 1 results)
      Level 3: 1 XOR gate (final result)

    Total: 7 XOR gates for 8 bits = log2(8) levels.

    Args:
        bits: A list of 0s and 1s (any width).

    Returns:
        1 if the number of '1' bits is ODD (PSW.P = 1 means odd count),
        0 if the number of '1' bits is EVEN.

    Note:
        The 8051 defines P=1 when ACC has an ODD number of set bits.
        This makes the system "even parity": ACC bits + P = even count.
    """
    if not bits:
        return 0
    # XOR tree: start with bit 0, XOR in each subsequent bit.
    # This is equivalent to a balanced tree but we unfold left-to-right.
    result = bits[0]
    for b in bits[1:]:
        result = XOR(result, b)
    return result


def compute_zero(bits: list[int]) -> int:
    """Zero detection: returns 1 if all bits are 0.

    Used for JZ/JNZ branch conditions.  Hardware implementation:
    an OR tree followed by a NOT:

      z0 = OR(bit0, bit1)
      z1 = OR(z0, bit2)
      ...
      z6 = OR(z5, bit7)
      output = NOT(z6)

    If ANY bit is 1, the OR chain produces 1, NOT gives 0.
    Only if ALL bits are 0 does the OR chain stay at 0, NOT gives 1.

    Args:
        bits: A list of 0s and 1s.

    Returns:
        1 if all bits are 0, 0 otherwise.
    """
    from logic_gates import OR

    if not bits:
        return 1
    combined = bits[0]
    for b in bits[1:]:
        combined = OR(combined, b)
    return NOT(combined)
