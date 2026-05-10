"""Condition Code Register (CCR) computation helpers for the Motorola 68000.

──────────────────────────────────────────────────────────────────────────────
OVERVIEW
──────────────────────────────────────────────────────────────────────────────

The 68000 CCR has five condition codes:

    C   Carry   — unsigned overflow/borrow out of the MSB.
    V   oVerflow — signed result outside representable range.
    Z   Zero    — result is zero.
    N   Negative — copy of the MSB of the result.
    X   eXtend  — set the same as C by ADD/SUB; used by ADDX/SUBX.

Key differences from the Intel 8086:
  • No AF (Auxiliary Carry) — the 68000 handles BCD differently.
  • No PF (Parity flag) — not present on the 68000.
  • X flag — absent on the 8086; lets extended-precision arithmetic chains
    carry borrow/carry through without disturbing C.
  • Logic ops (AND, OR, EOR, NOT) clear V and C, but leave X unchanged.
    On the 8086, logic ops always clear CF.
  • CLR/TST: set N/Z from result, clear V/C, do not touch X.

──────────────────────────────────────────────────────────────────────────────
CARRY FLAG — C
──────────────────────────────────────────────────────────────────────────────

    ADD (byte, 8-bit):  C = (raw_result > 0xFF)
    ADD (word, 16-bit): C = (raw_result > 0xFFFF)
    ADD (long, 32-bit): C = (raw_result > 0xFFFFFFFF)
    SUB/CMP (any):      C = (minuend < subtrahend)  ← borrow indicator

──────────────────────────────────────────────────────────────────────────────
OVERFLOW FLAG — V
──────────────────────────────────────────────────────────────────────────────

Same rule as on the 8086:

    ADD: V = (signs of both operands are equal) AND (sign of result differs)
    SUB: V = (signs of operands differ) AND (sign of result differs from minuend)

Equivalently, using the MSBs:
    ADD: V = (~(a ^ b)) & (a ^ result)  [both same sign, result different]
    SUB: V =  (a ^ b)   & (a ^ result)  [different signs, result flips minuend sign]

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

# ── Size constants ────────────────────────────────────────────────────────────
_BYTE_MASK = 0xFF
_WORD_MASK = 0xFFFF
_LONG_MASK = 0xFFFF_FFFF
_BYTE_MSB  = 0x80
_WORD_MSB  = 0x8000
_LONG_MSB  = 0x8000_0000


def _msb(value: int, *, word: bool = False, long: bool = False) -> int:
    """Return the mask for the most-significant bit given the operand size.

    >>> _msb(0, word=False, long=False)  # byte
    128
    >>> _msb(0, word=True)               # word
    32768
    >>> _msb(0, long=True)               # long
    2147483648
    """
    if long:  return _LONG_MSB
    if word:  return _WORD_MSB
    return _BYTE_MSB


def _mask(*, word: bool = False, long: bool = False) -> int:
    """Return the unsigned mask for the given size.

    >>> _mask(word=False, long=False)
    255
    >>> _mask(word=True)
    65535
    >>> _mask(long=True)
    4294967295
    """
    if long:  return _LONG_MASK
    if word:  return _WORD_MASK
    return _BYTE_MASK


# ── Carry ─────────────────────────────────────────────────────────────────────

def compute_c_add(raw_result: int, *, word: bool = False, long: bool = False) -> bool:
    """Carry for ADD/ADDI/ADDQ — unsigned result exceeds representable range.

    >>> compute_c_add(0x100, word=False)    # 256 > 255, carry
    True
    >>> compute_c_add(0xFF, word=False)     # 255 == 255, no carry
    False
    >>> compute_c_add(0x10000, word=True)   # 65536 > 65535
    True
    >>> compute_c_add(0x100000000, long=True)  # 2^32 > 2^32-1
    True
    """
    return raw_result > _mask(word=word, long=long)


def compute_c_sub(a: int, b: int, borrow: int = 0) -> bool:
    """Carry (borrow) for SUB/SUBI/SUBQ/CMP — a < b + borrow.

    Note: operands must already be in unsigned range (masked to size).

    >>> compute_c_sub(5, 3)    # 5 >= 3, no borrow
    False
    >>> compute_c_sub(3, 5)    # 3 < 5, borrow
    True
    >>> compute_c_sub(5, 5, 1) # 5 < 6, borrow
    True
    """
    return a < b + borrow


# ── Overflow ──────────────────────────────────────────────────────────────────

def compute_v_add(a: int, b: int, result: int,
                  *, word: bool = False, long: bool = False) -> bool:
    """Overflow for ADD/ADDI/ADDQ.

    Signed overflow when both inputs have the same sign but the result sign
    differs.  Using bitwise approach:
        V = (~(a ^ b)) & (a ^ result)  [MSBs only]

    >>> compute_v_add(0x7F, 0x01, 0x80, word=False)  # +127 + 1 → -128
    True
    >>> compute_v_add(0x01, 0x01, 0x02, word=False)  # no overflow
    False
    >>> compute_v_add(0x7FFF, 0x0001, 0x8000, word=True)  # word overflow
    True
    """
    msb = _msb(0, word=word, long=long)
    return bool((~(a ^ b)) & (a ^ result) & msb)


def compute_v_sub(a: int, b: int, result: int,
                  *, word: bool = False, long: bool = False) -> bool:
    """Overflow for SUB/SUBI/SUBQ/CMP (a − b).

    Signed overflow when operands have different signs and result sign ≠ a.
        V = (a ^ b) & (a ^ result)  [MSBs only]

    >>> compute_v_sub(0x80, 0x01, 0x7F, word=False)  # -128 - 1 → +127
    True
    >>> compute_v_sub(0x05, 0x03, 0x02, word=False)  # no overflow
    False
    >>> compute_v_sub(0x8000, 0x0001, 0x7FFF, word=True)  # word overflow
    True
    """
    msb = _msb(0, word=word, long=long)
    return bool((a ^ b) & (a ^ result) & msb)


# ── N and Z ───────────────────────────────────────────────────────────────────

def compute_n(result: int, *, word: bool = False, long: bool = False) -> bool:
    """Negative flag — copy of the MSB of the (masked) result.

    >>> compute_n(0x80, word=False)    # bit 7 set
    True
    >>> compute_n(0x7F, word=False)    # bit 7 clear
    False
    >>> compute_n(0x8000, word=True)   # bit 15 set
    True
    >>> compute_n(0x80000000, long=True)
    True
    """
    msb = _msb(0, word=word, long=long)
    m   = _mask(word=word, long=long)
    return bool(result & m & msb)


def compute_z(result: int, *, word: bool = False, long: bool = False) -> bool:
    """Zero flag — result is zero after masking.

    >>> compute_z(0, word=False)
    True
    >>> compute_z(1, word=False)
    False
    >>> compute_z(0x10000, word=True)   # 65536 & 0xFFFF = 0
    True
    """
    return (result & _mask(word=word, long=long)) == 0


# ── Combined helpers ──────────────────────────────────────────────────────────

def compute_nzvc_add(
    a: int, b: int, raw: int, carry_in: int = 0,
    *, word: bool = False, long: bool = False,
) -> tuple[bool, bool, bool, bool, bool]:
    """Compute (N, Z, V, C, X) for ADD/ADDI/ADDQ/ADDX.

    Parameters
    ----------
    a, b    : unsigned operands (already masked to size)
    raw     : a + b + carry_in (raw unmasked sum)
    carry_in: 0 or 1 (for ADDX)
    word, long: operand size

    Returns (N, Z, V, C, X).  X is always set the same as C.

    >>> compute_nzvc_add(0x7F, 0x01, 0x80, word=False)
    (True, False, True, False, False)
    >>> compute_nzvc_add(0xFF, 0x01, 0x100, word=False)
    (False, True, False, True, True)
    """
    m   = _mask(word=word, long=long)
    result = raw & m
    n = compute_n(result, word=word, long=long)
    z = compute_z(result, word=word, long=long)
    v = compute_v_add(a, b, result, word=word, long=long)
    c = compute_c_add(raw, word=word, long=long)
    return (n, z, v, c, c)  # X = C


def compute_nzvc_sub(
    a: int, b: int, raw: int, borrow: int = 0,
    *, word: bool = False, long: bool = False,
) -> tuple[bool, bool, bool, bool, bool]:
    """Compute (N, Z, V, C, X) for SUB/SUBI/SUBQ/SUBX/CMP/NEG.

    Parameters
    ----------
    a, b    : unsigned operands (already masked to size); result = a - b
    raw     : a - b - borrow (raw unmasked difference, may be negative)
    borrow  : 0 or 1 (for SUBX/SBB)
    word, long: operand size

    Returns (N, Z, V, C, X).

    >>> compute_nzvc_sub(0x05, 0x03, 0x02, word=False)
    (False, False, False, False, False)
    >>> compute_nzvc_sub(0x00, 0x01, -1, word=False)
    (True, False, False, True, True)
    """
    m      = _mask(word=word, long=long)
    result = raw & m
    n = compute_n(result, word=word, long=long)
    z = compute_z(result, word=word, long=long)
    v = compute_v_sub(a, b, result, word=word, long=long)
    c = compute_c_sub(a, b, borrow)
    return (n, z, v, c, c)  # X = C


def compute_nz_logic(
    result: int, *, word: bool = False, long: bool = False
) -> tuple[bool, bool]:
    """Compute (N, Z) for AND/OR/EOR/NOT/CLR.  V and C are always cleared.

    Returns (N, Z).

    >>> compute_nz_logic(0x00, word=False)
    (False, True)
    >>> compute_nz_logic(0x80, word=False)
    (True, False)
    >>> compute_nz_logic(0xFF, word=False)
    (True, False)
    """
    return (
        compute_n(result, word=word, long=long),
        compute_z(result, word=word, long=long),
    )


def compute_nzvc_neg(
    src: int, result: int, *, word: bool = False, long: bool = False
) -> tuple[bool, bool, bool, bool, bool]:
    """Compute (N, Z, V, C, X) for NEG (result = 0 − src).

    NEG carry semantics differ from SUB:
        C = (result != 0)   [i.e., C = (src != 0)]
    NEG overflow semantics:
        V = (src == MSB)    [i.e. NEG 0x80 → 0x80 = overflow]

    Returns (N, Z, V, C, X).

    >>> compute_nzvc_neg(0x00, 0x00, word=False)   # NEG 0 = 0, no carry
    (False, True, False, False, False)
    >>> compute_nzvc_neg(0x01, 0xFF, word=False)   # NEG 1 = -1 (0xFF), carry
    (True, False, False, True, True)
    >>> compute_nzvc_neg(0x80, 0x80, word=False)   # NEG -128 = -128, overflow
    (True, False, True, True, True)
    """
    m      = _mask(word=word, long=long)
    msb    = _msb(0, word=word, long=long)
    result = result & m
    n = bool(result & msb)
    z = (result == 0)
    v = (src == msb)   # overflow if negating the most-negative value
    c = (result != 0)  # carry if result is non-zero
    return (n, z, v, c, c)  # X = C


def pack_ccr(
    *, x: bool, n: bool, z: bool, v: bool, c: bool
) -> int:
    """Pack the five CCR bits into a 5-bit integer (bits 4–0 of SR).

    >>> pack_ccr(x=False, n=False, z=True, v=False, c=False)
    4
    >>> pack_ccr(x=True, n=True, z=False, v=True, c=True)
    27
    """
    return (
        (int(x) << 4)
        | (int(n) << 3)
        | (int(z) << 2)
        | (int(v) << 1)
        | (int(c) << 0)
    )


def unpack_ccr(ccr: int) -> dict[str, bool]:
    """Unpack a CCR byte into a dict of flag booleans.

    >>> flags = unpack_ccr(0b10100)  # X=1, N=0, Z=1, V=0, C=0
    >>> flags["x"]
    True
    >>> flags["z"]
    True
    >>> flags["c"]
    False
    """
    return {
        "x": bool(ccr & (1 << 4)),
        "n": bool(ccr & (1 << 3)),
        "z": bool(ccr & (1 << 2)),
        "v": bool(ccr & (1 << 1)),
        "c": bool(ccr & (1 << 0)),
    }
