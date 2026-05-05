"""Condition code helpers for the DEC PDP-11.

──────────────────────────────────────────────────────────────────────────────
PDP-11 CONDITION CODES
──────────────────────────────────────────────────────────────────────────────

The PSW (Processor Status Word) stores four condition codes in bits 3–0:

  N (bit 3) — Negative: MSB of result is 1
  Z (bit 2) — Zero:     result is exactly 0
  V (bit 1) — oVerflow: signed result outside representable range
  C (bit 0) — Carry:    unsigned overflow/borrow from MSB

Key differences from the Motorola 68000:
  • No X (extend) flag — multi-precision arithmetic uses ADC/SBC instead.
  • NEG carry rule: C = (result != 0), i.e. C=0 only when NEG 0 = 0.
  • COM always sets C=1 (bitwise NOT; by convention "no borrow").
  • CLR clears all four flags (N=0, Z=1, V=0, C=0).
  • TST clears V and C, sets N and Z from operand.

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

# ── Size constants ────────────────────────────────────────────────────────────
_BYTE_MASK = 0xFF
_WORD_MASK = 0xFFFF
_BYTE_MSB  = 0x80
_WORD_MSB  = 0x8000


def _msb(*, word: bool) -> int:
    """MSB mask for the given size.

    >>> _msb(word=False)
    128
    >>> _msb(word=True)
    32768
    """
    return _WORD_MSB if word else _BYTE_MSB


def _mask(*, word: bool) -> int:
    """Unsigned mask for the given size.

    >>> _mask(word=False)
    255
    >>> _mask(word=True)
    65535
    """
    return _WORD_MASK if word else _BYTE_MSB | (_BYTE_MASK ^ _BYTE_MSB) | _BYTE_MSB


# Simpler implementation
def _umask(*, word: bool) -> int:
    """Unsigned mask for word (16-bit) or byte (8-bit).

    >>> _umask(word=True)
    65535
    >>> _umask(word=False)
    255
    """
    return _WORD_MASK if word else _BYTE_MASK


def compute_n(result: int, *, word: bool) -> bool:
    """Negative flag: MSB of (masked) result.

    >>> compute_n(0x80, word=False)
    True
    >>> compute_n(0x7F, word=False)
    False
    >>> compute_n(0x8000, word=True)
    True
    >>> compute_n(0x7FFF, word=True)
    False
    """
    return bool(result & _msb(word=word))


def compute_z(result: int, *, word: bool) -> bool:
    """Zero flag: result is zero after masking.

    >>> compute_z(0, word=False)
    True
    >>> compute_z(1, word=False)
    False
    >>> compute_z(0x100, word=False)   # 256 & 0xFF = 0
    True
    """
    return (result & _umask(word=word)) == 0


def compute_v_add(a: int, b: int, result: int, *, word: bool) -> bool:
    """Overflow for ADD: both inputs same sign, result differs.

        V = (~(a ^ b)) & (a ^ result)   [MSB only]

    >>> compute_v_add(0x7F, 0x01, 0x80, word=False)   # +127+1 → -128
    True
    >>> compute_v_add(0x01, 0x01, 0x02, word=False)
    False
    >>> compute_v_add(0x7FFF, 0x0001, 0x8000, word=True)
    True
    """
    msb = _msb(word=word)
    return bool((~(a ^ b)) & (a ^ result) & msb)


def compute_v_sub(a: int, b: int, result: int, *, word: bool) -> bool:
    """Overflow for SUB/CMP (a − b): operands differ in sign, result flips a.

        V = (a ^ b) & (a ^ result)   [MSB only]

    >>> compute_v_sub(0x80, 0x01, 0x7F, word=False)   # -128-1 → +127
    True
    >>> compute_v_sub(0x05, 0x03, 0x02, word=False)
    False
    >>> compute_v_sub(0x8000, 0x0001, 0x7FFF, word=True)
    True
    """
    msb = _msb(word=word)
    return bool((a ^ b) & (a ^ result) & msb)


def compute_c_add(raw: int, *, word: bool) -> bool:
    """Carry for ADD: unsigned result exceeds representable range.

    >>> compute_c_add(0x100, word=False)
    True
    >>> compute_c_add(0xFF, word=False)
    False
    >>> compute_c_add(0x10000, word=True)
    True
    """
    return raw > _umask(word=word)


def compute_c_sub(a: int, b: int, borrow: int = 0) -> bool:
    """Carry (borrow) for SUB: a < b + borrow.

    >>> compute_c_sub(5, 3)
    False
    >>> compute_c_sub(3, 5)
    True
    >>> compute_c_sub(5, 5, 1)
    True
    """
    return a < b + borrow


def nzvc_add(a: int, b: int, carry_in: int = 0, *, word: bool) -> tuple[bool, bool, bool, bool]:
    """Compute (N, Z, V, C) for ADD/ADC.

    Parameters
    ----------
    a, b      : unsigned operands (masked to size)
    carry_in  : 0 or 1 for ADC
    word      : True for 16-bit, False for 8-bit

    >>> nzvc_add(0x7F, 0x01, word=False)   # overflow case
    (True, False, True, False)
    >>> nzvc_add(0xFF, 0x01, word=False)   # carry case
    (False, True, False, True)
    """
    m      = _umask(word=word)
    raw    = a + b + carry_in
    result = raw & m
    return (
        compute_n(result, word=word),
        compute_z(result, word=word),
        compute_v_add(a, b, result, word=word),
        compute_c_add(raw, word=word),
    )


def nzvc_sub(a: int, b: int, borrow: int = 0, *, word: bool) -> tuple[bool, bool, bool, bool]:
    """Compute (N, Z, V, C) for SUB/SBC/CMP (result = a − b − borrow).

    >>> nzvc_sub(0x05, 0x03, word=False)
    (False, False, False, False)
    >>> nzvc_sub(0x00, 0x01, word=False)   # 0 - 1 = 0xFF
    (True, False, False, True)
    """
    m      = _umask(word=word)
    raw    = a - b - borrow
    result = raw & m
    return (
        compute_n(result, word=word),
        compute_z(result, word=word),
        compute_v_sub(a, b, result, word=word),
        compute_c_sub(a, b, borrow),
    )


def nzvc_logic(result: int, *, word: bool) -> tuple[bool, bool, bool, bool]:
    """(N, Z, V=0, C=0) for BIT/BIC/BIS/MOV (V and C always cleared).

    >>> nzvc_logic(0, word=False)
    (False, True, False, False)
    >>> nzvc_logic(0x80, word=False)
    (True, False, False, False)
    """
    return (
        compute_n(result, word=word),
        compute_z(result, word=word),
        False,
        False,
    )


def pack_psw(n: bool, z: bool, v: bool, c: bool) -> int:
    """Pack four condition code bits into PSW bits 3–0.

    >>> pack_psw(True, False, False, False)
    8
    >>> pack_psw(False, True, False, False)
    4
    >>> pack_psw(True, False, True, True)
    11
    """
    return (int(n) << 3) | (int(z) << 2) | (int(v) << 1) | int(c)
