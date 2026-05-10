"""Flag computation helpers for the MOS 6502 processor.

The 6502 has 7 active flag bits in the processor status register (P):

    Bit 7  N  Negative   — bit 7 of result
    Bit 6  V  Overflow   — signed overflow
    Bit 5  -  (always 1)
    Bit 4  B  Break      — set only in stack copy during BRK/PHP
    Bit 3  D  Decimal    — BCD mode
    Bit 2  I  IRQ disable
    Bit 1  Z  Zero       — result == 0
    Bit 0  C  Carry      — carry out / not-borrow

Unlike the Intel 8080, the 6502 does NOT have an auxiliary carry (AC)
or parity (P) flag. The overflow flag V uses a different formula:
  V = (A7 ^ result7) & (M7 ^ result7)
where 7 = bit 7. This detects signed overflow in a single expression.
"""

from __future__ import annotations


def compute_nz(result: int) -> tuple[bool, bool]:
    """Return (N, Z) flags for an 8-bit result.

    N is set when bit 7 is 1 (result would be negative in two's complement).
    Z is set when result is zero.

    Examples::

        compute_nz(0x00)  → (False, True)   # zero
        compute_nz(0xFF)  → (True,  False)  # 0xFF = -1 in two's complement
        compute_nz(0x42)  → (False, False)  # ordinary positive
        compute_nz(0x80)  → (True,  False)  # -128 in two's complement
    """
    value = result & 0xFF
    return bool(value & 0x80), value == 0


def compute_overflow_add(a: int, b: int, result: int) -> bool:
    """Compute the V (overflow) flag for addition (ADC).

    Signed overflow occurs when two same-sign operands produce a
    different-sign result:

        +  +  +  → can't overflow  (two positives can't give negative)
        -  +  -  → can't overflow
        +  +  -  → overflow        (e.g. 127 + 1 = 128 = -128)
        -  +  +  → overflow        (e.g. -128 + (-1) = -129 = 127)

    The single-expression form:

        V = NOT(A[7] XOR B[7]) AND (A[7] XOR result[7])

    "Inputs had the same sign AND result has a different sign."

    Arguments are raw 8-bit integers (0–255); sign is inferred from bit 7.
    """
    a7 = (a >> 7) & 1
    b7 = (b >> 7) & 1
    r7 = (result >> 7) & 1
    # Same sign inputs that produced different sign output
    return bool((~(a7 ^ b7)) & (a7 ^ r7) & 1)


def compute_overflow_sub(a: int, b: int, result: int) -> bool:
    """Compute the V flag for subtraction (SBC).

    SBC internally computes A + ~B + C, so the overflow check is the
    same as for ADC but with the operand inverted:

        b_inv = (~b) & 0xFF
        V = compute_overflow_add(a, b_inv, result)

    This is equivalent to checking whether subtracting b from a (as
    signed bytes) produced a result outside −128..127.
    """
    return compute_overflow_add(a, (~b) & 0xFF, result)


def pack_p(n: bool, v: bool, b: bool, d: bool, i: bool, z: bool, c: bool) -> int:
    """Pack 7 flag booleans into the P status register byte.

    Bit 5 (unused) is always 1 in the 6502.

    Truth table::

        7 6 5 4 3 2 1 0
        N V 1 B D I Z C
    """
    return (
        (int(n) << 7)
        | (int(v) << 6)
        | 0x20              # bit 5 always 1
        | (int(b) << 4)
        | (int(d) << 3)
        | (int(i) << 2)
        | (int(z) << 1)
        | int(c)
    )


def unpack_p(p: int) -> tuple[bool, bool, bool, bool, bool, bool, bool]:
    """Unpack a P byte into (N, V, B, D, I, Z, C) flag booleans.

    Example::

        unpack_p(0x24)  →  (False, False, False, False, True, False, False)
        #  0x24 = 0b00100100 = bit5=1, I=1
    """
    n = bool(p & 0x80)
    v = bool(p & 0x40)
    b = bool(p & 0x10)
    d = bool(p & 0x08)
    i = bool(p & 0x04)
    z = bool(p & 0x02)
    c = bool(p & 0x01)
    return n, v, b, d, i, z, c


def bcd_add(a: int, b: int, carry_in: bool) -> tuple[int, bool]:
    """BCD (decimal mode) addition: a + b + carry_in.

    The 6502 NMOS chip performs BCD correction *after* the binary add,
    which means that in decimal mode the N, V, Z flags still reflect the
    *binary* result (not the BCD-corrected result). Only C is computed
    correctly from the BCD result.

    Returns (bcd_result, carry_out).

    Algorithm (NMOS 6502 behaviour):
    1. Add the low nibbles. If > 9, add 6 (carry into high nibble).
    2. Add the high nibbles + carry from step 1. If > 9, add 6.
    3. Final carry = 1 if high nibble carried out.

    Example::

        bcd_add(0x09, 0x01, False)  → (0x10, False)  # 9 + 1 = 10 in BCD
        bcd_add(0x99, 0x01, False)  → (0x00, True)   # 99 + 1 = 100, carry
    """
    # Low nibble
    low = (a & 0x0F) + (b & 0x0F) + int(carry_in)
    carry_low = low > 9
    if carry_low:
        low = (low + 6) & 0x0F

    # High nibble
    high = (a >> 4) + (b >> 4) + int(carry_low)
    carry_out = high > 9
    if carry_out:
        high = (high + 6) & 0x0F

    return ((high << 4) | low) & 0xFF, carry_out


def bcd_sub(a: int, b: int, carry_in: bool) -> tuple[int, bool]:
    """BCD subtraction: a - b - (1 - carry_in).

    The 6502 implements SBC as A + ~B + C. In decimal mode this still
    uses BCD correction on the subtraction path.

    Returns (bcd_result, carry_out) where carry_out = 1 means no borrow.

    Example::

        bcd_sub(0x10, 0x01, True)   → (0x09, True)   # 10 - 1 = 9, no borrow
        bcd_sub(0x00, 0x01, True)   → (0x99, False)  # 0 - 1 = 99, borrow
    """
    # Low nibble
    low = (a & 0x0F) - (b & 0x0F) - int(not carry_in)
    borrow_low = low < 0
    if borrow_low:
        low = (low - 6) & 0x0F

    # High nibble
    high = (a >> 4) - (b >> 4) - int(borrow_low)
    borrow_out = high < 0
    if borrow_out:
        high = (high - 6) & 0x0F

    carry_out = not borrow_out
    return ((high << 4) | low) & 0xFF, carry_out
