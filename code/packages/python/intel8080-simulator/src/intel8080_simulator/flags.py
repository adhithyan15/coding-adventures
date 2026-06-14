"""Flag computation helpers for the Intel 8080.

The 8080 has five condition flags: Sign (S), Zero (Z), Auxiliary Carry (AC),
Parity (P), and Carry (CY).  Each is updated by different subsets of
instructions, so this module provides targeted helpers rather than one
monolithic "update all flags" function.

Flag bit positions in the packed flags byte
--------------------------------------------
  bit 7 = S    bit 6 = Z    bit 5 = 0
  bit 4 = AC   bit 3 = 0    bit 2 = P
  bit 1 = 1    bit 0 = CY

The 0-bits and 1-bit (bit 1) are fixed values documented by Intel.

Carry semantics
---------------
For addition:  CY = 1 if result > 0xFF (carry out of bit 7)
For subtraction: CY = 1 if result < 0 (borrow out — A < subtrahend)

Auxiliary carry
---------------
AC is set when there is a carry (or borrow) out of bit 3 into bit 4.
This is used only by the DAA (Decimal Adjust Accumulator) instruction.

Parity
------
P = 1 when the result byte has an even number of 1-bits (even parity).
P = 0 when the result byte has an odd number of 1-bits (odd parity).
"""

from __future__ import annotations

__all__ = [
    "compute_s",
    "compute_z",
    "compute_p",
    "compute_cy_add",
    "compute_cy_sub",
    "compute_ac_add",
    "compute_ac_sub",
    "flags_from_byte",
    "szp_flags",
]


def compute_s(result: int) -> bool:
    """Sign flag: bit 7 of result is 1.

    In two's complement, bit 7 being 1 means the value is "negative"
    when interpreted as a signed 8-bit integer.  The 8080 uses unsigned
    storage but sets S so that signed comparisons work correctly.
    """
    return bool(result & 0x80)


def compute_z(result: int) -> bool:
    """Zero flag: result (after masking to 8 bits) is exactly 0x00."""
    return (result & 0xFF) == 0


def compute_p(result: int) -> bool:
    """Parity flag: True (even parity) when result byte has an even number of set bits.

    Bit-trick: XOR all 8 bits together.  If the count of 1-bits is even,
    the XOR folds to 0; if odd, it folds to 1.  We want True for even, so
    we invert the XOR result.

    Example: 0b01010101 → 4 ones → even → True
             0b01010111 → 5 ones → odd  → False
    """
    v = result & 0xFF
    v ^= v >> 4
    v ^= v >> 2
    v ^= v >> 1
    return not (v & 1)


def compute_cy_add(result: int) -> bool:
    """Carry flag for addition: set if result does not fit in 8 bits.

    After adding two 8-bit values (and an optional carry), the sum may
    exceed 255.  The carry flag captures the overflow into a hypothetical
    9th bit.
    """
    return result > 0xFF


def compute_cy_sub(a: int, b: int, borrow: int = 0) -> bool:
    """Carry (borrow) flag for subtraction: set if a < (b + borrow).

    In the 8080, CY=1 after subtraction means "borrow occurred" — the
    result would have been negative if interpreted as signed.
    """
    return a < (b + borrow)


def compute_ac_add(a: int, b: int, cin: int = 0) -> bool:
    """Auxiliary carry for addition: carry out of bit 3 (low nibble → high nibble).

    The Intel 8080 System Reference Manual defines AC as set when there is
    a carry out of bit position 3 during the addition.  This is equivalent
    to asking whether the sum of the low nibbles exceeds 0x0F.
    """
    return ((a & 0x0F) + (b & 0x0F) + cin) > 0x0F


def compute_ac_sub(a: int, b: int, borrow: int = 0) -> bool:
    """Auxiliary carry for subtraction: borrow out of bit 3.

    For subtraction, AC is set when there is a borrow from bit 4 into bit 3
    — equivalently, when the low nibble of a is less than the low nibble of b
    plus the incoming borrow.
    """
    return (a & 0x0F) < ((b & 0x0F) + borrow)


def compute_ac_ana(a: int, b: int) -> bool:
    """Auxiliary carry for ANA/ANI (AND instruction).

    Per the Intel 8080 System Reference Manual, ANA sets AC to the logical OR
    of bit 3 of the two operands.  This is the documented-but-quirky behavior
    of the 8080 AND instruction, which differs from the 8085.
    """
    return bool(((a | b) >> 3) & 1)


def szp_flags(result: int) -> tuple[bool, bool, bool]:
    """Return (S, Z, P) flags for a result byte.

    Convenience helper used by most ALU operations.
    """
    return compute_s(result), compute_z(result), compute_p(result)


def flags_from_byte(byte: int) -> tuple[bool, bool, bool, bool, bool]:
    """Unpack a flags byte into (S, Z, AC, P, CY).

    Used by POP PSW to restore flags from the stack.

    Bit layout:  S=7, Z=6, (fixed 0)=5, AC=4, (fixed 0)=3, P=2, (fixed 1)=1, CY=0
    """
    s = bool(byte & 0x80)
    z = bool(byte & 0x40)
    ac = bool(byte & 0x10)
    p = bool(byte & 0x04)
    cy = bool(byte & 0x01)
    return s, z, ac, p, cy
