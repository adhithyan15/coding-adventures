"""Flag computation helpers for the Zilog Z80 processor.

The Z80 F register has more named flags than the Intel 8080:

    Bit 7  S   Sign       — bit 7 of result (negative in two's complement)
    Bit 6  Z   Zero       — result == 0
    Bit 5  Y   Undocumented (copy of result bit 5)
    Bit 4  H   Half-carry — carry from bit 3→4 (set) / borrow bit 4→3 (set)
    Bit 3  X   Undocumented (copy of result bit 3)
    Bit 2  P/V Parity (after logical ops) / Overflow (after arithmetic)
    Bit 1  N   Add/Subtract — cleared by ADD, set by SUB
    Bit 0  C   Carry

Key differences from the Intel 8080:
- H (half-carry) is a NAMED, tested flag in Z80 (it's present in 8080 too
  but as an unnamed/less-prominent bit)
- N (subtract) is unique to Z80 — required for correct DAA behaviour
- P/V unifies two concepts: parity after logical ops, overflow after arithmetic
"""

from __future__ import annotations


def compute_sz(result: int) -> tuple[bool, bool]:
    """Return (S, Z) from an 8-bit result.

    S = bit 7 set (negative in two's complement).
    Z = result is zero.

    Examples::

        compute_sz(0x00) → (False, True)   # zero
        compute_sz(0xFF) → (True,  False)  # -1
        compute_sz(0x42) → (False, False)  # positive
        compute_sz(0x80) → (True,  False)  # -128
    """
    v = result & 0xFF
    return bool(v & 0x80), v == 0


def compute_parity(value: int) -> bool:
    """Return True if the number of set bits in value (0–255) is even.

    The Z80 P/V flag is set to parity-even after logical operations
    (AND, OR, XOR).  An 8-bit value has even parity when the XOR of all
    its bits is 0.

    Examples::

        compute_parity(0x00) → True   # 0 ones → even
        compute_parity(0x01) → False  # 1 one  → odd
        compute_parity(0x03) → True   # 2 ones → even
        compute_parity(0xFF) → True   # 8 ones → even
    """
    v = value & 0xFF
    # XOR all bits together
    v ^= v >> 4
    v ^= v >> 2
    v ^= v >> 1
    return not bool(v & 1)   # True = even parity


def compute_overflow_add(a: int, b: int, result: int) -> bool:
    """V (overflow) flag for addition: did signed result overflow?

    Overflow when two same-sign inputs produce a different-sign output.

    Formula: V = NOT(A7 XOR B7) AND (A7 XOR Result7)
    "Same sign inputs produced different sign output."

    Arguments are raw unsigned 8-bit values (0–255).
    """
    a7 = (a >> 7) & 1
    b7 = (b >> 7) & 1
    r7 = (result >> 7) & 1
    return bool((~(a7 ^ b7)) & (a7 ^ r7) & 1)


def compute_overflow_sub(a: int, b: int, result: int) -> bool:
    """V (overflow) flag for subtraction: did signed result overflow?

    SBC/SUB internally compute A + (~B) + carry, so overflow uses ADC formula
    with inverted operand.

    Arguments are raw unsigned 8-bit values (0–255).
    """
    return compute_overflow_add(a, (~b) & 0xFF, result)


def compute_half_carry_add(a: int, b: int, carry: int = 0) -> bool:
    """H flag for addition: carry from bit 3 to bit 4.

    H = ((a & 0x0F) + (b & 0x0F) + carry) > 0x0F

    Examples::

        compute_half_carry_add(0x0F, 0x01) → True   # 0x0F + 1 = 0x10
        compute_half_carry_add(0x07, 0x08) → False  # 0x07 + 8 = 0x0F, no carry
    """
    return ((a & 0x0F) + (b & 0x0F) + carry) > 0x0F


def compute_half_carry_sub(a: int, b: int, borrow: int = 0) -> bool:
    """H flag for subtraction: borrow from bit 4 into bit 3.

    H = (a & 0x0F) < (b & 0x0F) + borrow

    Examples::

        compute_half_carry_sub(0x10, 0x01) → False  # low nibble 0 >= 1? No, H=1
        compute_half_carry_sub(0x00, 0x01) → True   # 0 < 1
    """
    return (a & 0x0F) < (b & 0x0F) + borrow


def pack_f(s: bool, z: bool, h: bool, pv: bool, n: bool, c: bool) -> int:
    """Pack six main flags into the Z80 F register byte.

    Bits 5 (Y) and 3 (X) are set to 0 (undocumented; not tested by this impl).

    Truth table::

        7  6  5  4  3  2  1  0
        S  Z  0  H  0  PV N  C
    """
    return (
        (int(s)  << 7)
        | (int(z)  << 6)
        | (int(h)  << 4)
        | (int(pv) << 2)
        | (int(n)  << 1)
        | int(c)
    )


def unpack_f(f: int) -> tuple[bool, bool, bool, bool, bool, bool]:
    """Unpack F byte into (S, Z, H, PV, N, C).

    Example::

        unpack_f(0x00) → (False, False, False, False, False, False)
        unpack_f(0xFF) → (True,  True,  True,  True,  True,  True)
        unpack_f(0x44) → (False, True,  False, True,  False, False)  # Z=1, PV=1
    """
    s  = bool(f & 0x80)
    z  = bool(f & 0x40)
    h  = bool(f & 0x10)
    pv = bool(f & 0x04)
    n  = bool(f & 0x02)
    c  = bool(f & 0x01)
    return s, z, h, pv, n, c


def daa(
    a: int, flag_n: bool, flag_h: bool, flag_c: bool
) -> tuple[int, bool, bool, bool]:
    """Decimal Adjust Accumulator after ADD or SUB on BCD values.

    The Z80 DAA instruction corrects the accumulator after an ADD or SUB
    on two BCD (Binary-Coded Decimal) values.  N flag tells us whether the
    last operation was addition (N=0) or subtraction (N=1).

    Returns (new_a, new_h, new_pv, new_c).
    S and Z are computed from new_a by the caller.

    Z80 DAA algorithm:
    ------------------
    If N=0 (after ADD):
      - If C=1 or A > 0x99: A += 0x60; C=1
      - If H=1 or (A & 0x0F) > 9: A += 0x06
    If N=1 (after SUB):
      - If C=1: A -= 0x60; C=1 (C stays set)
      - If H=1: A -= 0x06

    H flag after DAA:
      - After ADD: H = bit 4 carry (depends on low nibble correction)
      - After SUB: H = was H set and low nibble < 6?

    Examples::

        daa(0x09, False, False, False) after ADD A,1 (A was 8):
            low nibble 9 ≤ 9, C=0, A ≤ 0x99 → no correction → (0x09, ...)
        daa(0x0A, False, False, False) after adding BCD 5+5:
            low nibble 0x0A > 9 → add 6 → 0x10 (BCD 10) correct
    """
    c_out = flag_c
    correction = 0

    if not flag_n:
        # After addition
        if flag_h or (a & 0x0F) > 9:
            correction |= 0x06
        if flag_c or a > 0x99:
            correction |= 0x60
            c_out = True
        new_a = (a + correction) & 0xFF
        new_h = bool((a & 0x0F) + (correction & 0x0F) > 0x0F)
    else:
        # After subtraction
        if flag_h or (a & 0x0F) > 9:
            correction |= 0x06
        if flag_c or a > 0x99:
            correction |= 0x60
            c_out = True
        new_a = (a - correction) & 0xFF
        new_h = flag_h and (a & 0x0F) < 6

    new_pv = compute_parity(new_a)
    return new_a, new_h, new_pv, c_out
