"""Flag computation helpers for the Intel 8086.

──────────────────────────────────────────────────────────────────────────────
OVERVIEW
──────────────────────────────────────────────────────────────────────────────

The 8086 updates six arithmetic/logical flags after most instructions:

    CF  Carry — unsigned overflow/borrow out of the MSB.
    PF  Parity — 1 if the low byte of the result has an even number of 1-bits.
    AF  Auxiliary carry — carry/borrow out of bit 3 (BCD arithmetic).
    ZF  Zero — result is zero.
    SF  Sign — copy of the MSB of the result.
    OF  Overflow — signed result lies outside the representable range.

Two additional flags are not set by arithmetic:
    DF  Direction — controlled by CLD/STD; affects string op direction.
    IF  Interrupt enable — controlled by CLI/STI.

These helpers are pure functions with no side effects, making them easy to
test in isolation and compose inside the simulator's execute loop.

──────────────────────────────────────────────────────────────────────────────
CARRY FLAG — CF
──────────────────────────────────────────────────────────────────────────────

    ADD/ADC (8-bit):  CF = (result > 0xFF)
    ADD/ADC (16-bit): CF = (result > 0xFFFF)
    SUB/SBB/CMP (8-bit):  CF = (minuend < subtrahend + borrow)  — borrow!
    SUB/SBB/CMP (16-bit): CF = (minuend < subtrahend + borrow)

──────────────────────────────────────────────────────────────────────────────
OVERFLOW FLAG — OF
──────────────────────────────────────────────────────────────────────────────

Signed overflow occurs when:
    ADD: (+) + (+) = (−)  or  (−) + (−) = (+)
    SUB: (−) − (+) = (+)  or  (+) − (−) = (−)

For ADD (8-bit): OF = (a < 0x80 and b < 0x80 and result >= 0x80)
               or (a >= 0x80 and b >= 0x80 and result < 0x80)

Equivalently: OF = MSB(a) == MSB(b) and MSB(result) != MSB(a)
(If both inputs have the same sign but the output has a different sign,
overflow occurred.)

For SUB (compute as a − b = a + (−b)):
    OF = MSB(a) != MSB(b) and MSB(result) != MSB(a)
    (Different-sign operands, result sign doesn't match minuend sign.)

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

# ── Size constants ────────────────────────────────────────────────────────────
_BYTE_MASK = 0xFF
_WORD_MASK = 0xFFFF
_BYTE_MSB = 0x80
_WORD_MSB = 0x8000


# ── Carry ─────────────────────────────────────────────────────────────────────

def compute_cf_add(result: int, *, word: bool) -> bool:
    """Carry flag for ADD/ADC.

    The raw (unmasked) result exceeds the representable unsigned range.

    >>> compute_cf_add(0x100, word=False)   # 256 > 255
    True
    >>> compute_cf_add(0xFF, word=False)    # 255 == 255, no carry
    False
    >>> compute_cf_add(0x10000, word=True)  # 65536 > 65535
    True
    """
    limit = _WORD_MASK if word else _BYTE_MASK
    return result > limit


def compute_cf_sub(minuend: int, subtrahend: int, borrow: int = 0) -> bool:
    """Carry flag for SUB/SBB/CMP (as a borrow indicator).

    CF=1 when the subtraction requires a borrow (minuend < subtrahend + borrow).
    Operands must already be in unsigned range.

    >>> compute_cf_sub(5, 3)    # 5 >= 3, no borrow
    False
    >>> compute_cf_sub(3, 5)    # 3 < 5, borrow!
    True
    >>> compute_cf_sub(5, 5, 1) # 5 < 5+1=6, borrow!
    True
    """
    return minuend < subtrahend + borrow


# ── Auxiliary carry (BCD) ─────────────────────────────────────────────────────

def compute_af_add(a: int, b: int, carry_in: int = 0) -> bool:
    """Auxiliary carry flag for ADD/ADC: carry from bit 3 to bit 4.

    >>> compute_af_add(0x0F, 0x01)   # 0F + 01 = 10, carry from nibble
    True
    >>> compute_af_add(0x01, 0x01)   # 01 + 01 = 02, no nibble carry
    False
    """
    return ((a & 0xF) + (b & 0xF) + carry_in) > 0xF


def compute_af_sub(a: int, b: int, borrow: int = 0) -> bool:
    """Auxiliary carry flag for SUB/SBB/CMP: borrow from bit 4 into bit 3.

    >>> compute_af_sub(0x10, 0x01)   # low nibble 0 < 1, borrow
    True
    >>> compute_af_sub(0x05, 0x03)   # low nibble 5 >= 3, no borrow
    False
    """
    return (a & 0xF) < (b & 0xF) + borrow


# ── Overflow ──────────────────────────────────────────────────────────────────

def compute_of_add(a: int, b: int, result: int, *, word: bool) -> bool:
    """Overflow flag for ADD/ADC.

    Signed overflow: both operands have the same sign, but the result sign
    differs.  We check via MSB of the masked values.

    >>> compute_of_add(0x7F, 0x01, 0x80, word=False)   # +127 + 1 = -128
    True
    >>> compute_of_add(0x01, 0x01, 0x02, word=False)   # no overflow
    False
    """
    msb = _WORD_MSB if word else _BYTE_MSB
    mask = _WORD_MASK if word else _BYTE_MASK
    a_sign = a & msb
    b_sign = b & msb
    r_sign = result & msb & mask
    return a_sign == b_sign and r_sign != a_sign


def compute_of_sub(a: int, b: int, result: int, *, word: bool) -> bool:
    """Overflow flag for SUB/SBB/CMP/NEG.

    Signed overflow: operands have different signs and result sign != minuend's.

    >>> compute_of_sub(0x80, 0x01, 0x7F, word=False)   # -128 - 1 = +127?!
    True
    >>> compute_of_sub(0x05, 0x03, 0x02, word=False)   # 5 - 3 = 2, no overflow
    False
    """
    msb = _WORD_MSB if word else _BYTE_MSB
    mask = _WORD_MASK if word else _BYTE_MASK
    a_sign = a & msb
    b_sign = b & msb
    r_sign = result & msb & mask
    return a_sign != b_sign and r_sign != a_sign


# ── Sign, Zero, Parity ────────────────────────────────────────────────────────

def compute_sf(result: int, *, word: bool) -> bool:
    """Sign flag: copy of the MSB of the (masked) result.

    >>> compute_sf(0x80, word=False)   # bit 7 set
    True
    >>> compute_sf(0x7F, word=False)   # bit 7 clear
    False
    >>> compute_sf(0x8000, word=True)
    True
    """
    msb = _WORD_MSB if word else _BYTE_MSB
    mask = _WORD_MASK if word else _BYTE_MASK
    return bool(result & mask & msb)


def compute_zf(result: int, *, word: bool) -> bool:
    """Zero flag: result is zero after masking.

    >>> compute_zf(0, word=False)
    True
    >>> compute_zf(1, word=False)
    False
    >>> compute_zf(0x10000, word=True)  # 65536 & 0xFFFF = 0
    True
    """
    mask = _WORD_MASK if word else _BYTE_MASK
    return (result & mask) == 0


def compute_parity(result: int) -> bool:
    """Parity flag: 1 if the LOW BYTE of result has an even number of 1-bits.

    >>> compute_parity(0)      # 0 ones — even → PF=1
    True
    >>> compute_parity(1)      # 1 one — odd → PF=0
    False
    >>> compute_parity(3)      # 2 ones — even → PF=1
    True
    >>> compute_parity(0x100)  # high byte ignored; low byte=0 → PF=1
    True
    """
    low = result & _BYTE_MASK
    # Count 1-bits in low byte; PF=1 when count is even
    ones = bin(low).count("1")
    return ones % 2 == 0


def compute_szp(result: int, *, word: bool) -> tuple[bool, bool, bool]:
    """Compute (SF, ZF, PF) together from a result value.

    Returns a 3-tuple: (sf, zf, pf).

    >>> compute_szp(0, word=False)
    (False, True, True)
    >>> compute_szp(0xFF, word=False)
    (True, False, True)
    >>> compute_szp(1, word=False)
    (False, False, False)
    """
    return (
        compute_sf(result, word=word),
        compute_zf(result, word=word),
        compute_parity(result),
    )


# ── FLAGS register pack/unpack ────────────────────────────────────────────────

def pack_flags(
    *,
    cf: bool,
    pf: bool,
    af: bool,
    zf: bool,
    sf: bool,
    tf: bool,
    if_: bool,
    df: bool,
    of: bool,
) -> int:
    """Pack individual flag booleans into a 16-bit FLAGS register value.

    Bit  1 is always 1 (reserved, always set on real 8086).

    >>> pack_flags(cf=True, pf=False, af=False, zf=False,
    ...            sf=False, tf=False, if_=False, df=False, of=False)
    3
    """
    return (
        (int(cf) << 0)
        | (1 << 1)
        | (int(pf) << 2)
        | (int(af) << 4)
        | (int(zf) << 6)
        | (int(sf) << 7)
        | (int(tf) << 8)
        | (int(if_) << 9)
        | (int(df) << 10)
        | (int(of) << 11)
    )


def unpack_flags(f: int) -> dict[str, bool]:
    """Unpack a 16-bit FLAGS value into individual booleans.

    Returns a dict with keys: cf, pf, af, zf, sf, tf, if_, df, of.

    >>> flags = unpack_flags(0b0000_0000_0100_0010)   # PF + always-1
    >>> flags["pf"]
    True
    >>> flags["cf"]
    False
    """
    return {
        "cf":  bool(f & (1 << 0)),
        "pf":  bool(f & (1 << 2)),
        "af":  bool(f & (1 << 4)),
        "zf":  bool(f & (1 << 6)),
        "sf":  bool(f & (1 << 7)),
        "tf":  bool(f & (1 << 8)),
        "if_": bool(f & (1 << 9)),
        "df":  bool(f & (1 << 10)),
        "of":  bool(f & (1 << 11)),
    }
