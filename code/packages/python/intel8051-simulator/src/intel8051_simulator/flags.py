"""Arithmetic flag helpers for the Intel 8051.

The 8051 flag model differs from the PDP-11 in one important way: the
carry flag is also the borrow flag for subtraction (SUBB).  When the
hardware performs A - B - CY, if a borrow is generated the carry flag is
SET (not cleared).  This is the opposite of "carry = no borrow" convention
used by x86.  In other words:

    CY = 1  ↔  a borrow occurred (unsigned underflow)
    CY = 0  ↔  no borrow (result is non-negative)

Because of this, SUBB must be tested for CY to detect "A < B + old_CY".

Flag bits in the returned tuple are always plain Python booleans (or ints 0/1).
All helpers work on 8-bit values only.

Truth table for ADD8 carry / overflow:
    Carry:    result > 0xFF  (treat inputs as unsigned 8-bit)
    AuxCarry: (a & 0x0F) + (b & 0x0F) + cin > 0x0F
    Overflow: (both inputs positive, result negative)
              OR (both negative, result positive)
              where "sign" = bit 7 of the 8-bit value
"""

from __future__ import annotations

# ── Helpers ────────────────────────────────────────────────────────────────────

def _parity(val: int) -> int:
    """Return 1 if val (8-bit) has an odd number of 1 bits (odd parity → P=1).

    The 8051 PSW.P is the EVEN-parity bit: P=1 when ACC has an odd number of
    1s, making the 9-bit value {P, ACC} have even parity.
    """
    v = val & 0xFF
    v ^= v >> 4
    v ^= v >> 2
    v ^= v >> 1
    return v & 1


# ── 8-bit addition ─────────────────────────────────────────────────────────────

def add8_flags(a: int, b: int, cin: int = 0) -> tuple[int, int, int, int, int]:
    """Compute A + B + cin and return (result8, CY, AC, OV, P).

    Args:
        a:   8-bit addend (0–255)
        b:   8-bit addend (0–255)
        cin: carry-in (0 or 1), used by ADDC

    Returns:
        (result, cy, ac, ov, p)
        result — 8-bit sum (bits 7:0 of the full sum)
        cy     — 1 if unsigned overflow (carry out of bit 7)
        ac     — 1 if carry from bit 3 to bit 4
        ov     — 1 if signed overflow (wrong-sign result)
        p      — even-parity of result (1 if odd popcount)
    """
    a8, b8 = a & 0xFF, b & 0xFF
    full  = a8 + b8 + cin
    result = full & 0xFF
    cy = 1 if full > 0xFF else 0
    ac = 1 if (a8 & 0x0F) + (b8 & 0x0F) + cin > 0x0F else 0
    # Signed overflow: both operands have same sign but result has different sign
    sa, sb, sr = a8 >> 7, b8 >> 7, result >> 7
    ov = 1 if (sa == sb) and (sr != sa) else 0
    p  = _parity(result)
    return result, cy, ac, ov, p


# ── 8-bit subtraction ──────────────────────────────────────────────────────────

def sub8_flags(a: int, b: int, borrow: int = 0) -> tuple[int, int, int, int, int]:
    """Compute A - B - borrow (SUBB) and return (result8, CY, AC, OV, P).

    CY (borrow out) = 1 when unsigned underflow occurs, i.e. when
    a < (b + borrow) treating all as 8-bit unsigned.

    AC (auxiliary borrow) = 1 when a borrow propagated from bit 3 to bit 4,
    i.e. (a & 0x0F) < (b & 0x0F) + borrow.

    OV = 1 when signed overflow: subtracting a negative from a positive gives
    a negative result, or subtracting a positive from a negative gives a
    positive result.

    Args:
        a:      8-bit minuend
        b:      8-bit subtrahend
        borrow: old carry (CY) fed into SUBB
    """
    a8, b8 = a & 0xFF, b & 0xFF
    full   = a8 - b8 - borrow
    result = full & 0xFF
    cy = 1 if full < 0 else 0
    ac = 1 if (a8 & 0x0F) < (b8 & 0x0F) + borrow else 0
    # Signed overflow: subtracting a negative (b with sign=1) from positive a
    # gives negative result, or positive b from negative a gives positive result
    sa, sb, sr = a8 >> 7, b8 >> 7, result >> 7
    ov = 1 if (sa != sb) and (sr == sb) else 0
    p  = _parity(result)
    return result, cy, ac, ov, p


# ── Decimal adjust ─────────────────────────────────────────────────────────────

def da_flags(a: int, cy_in: int, ac_in: int) -> tuple[int, int, int]:
    """Decimal-adjust A after BCD ADD/ADDC.  Returns (result, CY, P).

    The DA A instruction corrects the binary sum of two BCD digits into
    a valid BCD result.  The algorithm (from the 8051 datasheet):

    Step 1: if (A[3:0] > 9) OR AC == 1: add 6 to A
    Step 2: if (A[7:4] > 9) OR CY == 1: add 0x60 to A; set CY

    Note: the adjustment is done on a potentially intermediate value where
    step 1 may have already modified bits 7:4.

    Args:
        a:     8-bit accumulator value after ADD/ADDC
        cy_in: carry flag coming into DA A
        ac_in: auxiliary carry coming into DA A

    Returns:
        (result, new_cy, new_p)
    """
    a8 = a & 0xFF
    new_cy = cy_in

    if (a8 & 0x0F) > 9 or ac_in:
        a8 = (a8 + 0x06) & 0xFF

    if (a8 >> 4) > 9 or cy_in:
        a8 = (a8 + 0x60) & 0xFF
        new_cy = 1

    result = a8 & 0xFF
    return result, new_cy, _parity(result)
