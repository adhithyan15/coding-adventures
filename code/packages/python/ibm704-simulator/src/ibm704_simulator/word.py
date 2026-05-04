"""IBM 704 word format — sign-magnitude integers, floating-point, and packing.

The IBM 704 operates on 36-bit words. Every word can be interpreted in three
ways depending on what instruction is touching it:

  * **Integer (sign-magnitude):** bit 35 is the sign (1 = negative), bits 0-34
    are a 35-bit magnitude. Two zeros exist: ``+0`` (0x000000000) and
    ``-0`` (0x800000000) are distinct words.
  * **Floating-point:** bit 35 is the sign of the fraction, bits 27-34 are
    an 8-bit excess-128 exponent ("characteristic"), and bits 0-26 are a
    27-bit normalized fraction.
  * **Instruction:** bit-fields decoded by the simulator's instruction loop;
    see ``simulator.py`` for that.

Bit-numbering convention
------------------------
We use **Python's natural bit numbering** throughout this module — bit N has
value ``2 ** N``. The IBM 704 *Reference Manual* numbers bits in the
**opposite** direction (bit 0 = leftmost = MSB, bit 35 = rightmost = LSB).
Every helper that takes IBM bit numbers as inputs converts to Python bit
numbers internally; helpers documented in IBM-bit-number terms are noted as
such.

Sign-magnitude refresher
------------------------
Two's-complement and sign-magnitude diverge whenever you reach for the sign
of a negative number:

================== ===========================  ===========================
Operation          Two's complement             Sign-magnitude (IBM 704)
================== ===========================  ===========================
``+3``             ``0...011``                  ``0...011``
``-3``             ``1...101`` (flip + 1)       ``1...011`` (just flip sign)
``-(+0)``          ``+0`` (only one zero)       ``-0`` (distinct from +0)
``a + b`` (signs differ) Just bit-add and let
                   carry propagate.             Compare magnitudes; subtract
                                                smaller from larger; result
                                                takes sign of larger.
================== ===========================  ===========================

The 704's accumulator has 38 bits (S, Q, P, then 35 magnitude) so that
arithmetic overflow is *representable* — the Q and P bits catch the carry that
would otherwise be silently lost. Programs check the **overflow trigger**
(set when P=1 after an arithmetic op) via the ``TOV`` instruction.

Why this matters for hosted languages
-------------------------------------
FORTRAN I's ``INTEGER`` and ``REAL`` types are these formats verbatim. LISP's
cons cell is a 36-bit word with a 15-bit ``car`` (address field) and a 15-bit
``cdr`` (decrement field). When you compile FORTRAN or LISP to this simulator,
"the bits a 704 would have stored" is the contract — and getting
sign-magnitude wrong means TZE behaves wrong on negative zero and FORTRAN
arithmetic IF goes to the wrong branch.
"""

from __future__ import annotations

import math

# ---------------------------------------------------------------------------
# Word constants
# ---------------------------------------------------------------------------

WORD_BITS = 36
"""Total bits in an IBM 704 word."""

WORD_MASK = (1 << WORD_BITS) - 1
"""Mask to clamp an integer to 36 bits: ``0x000_F_FFFF_FFFF``."""

SIGN_BIT = 1 << 35
"""The sign bit position. A word is negative iff ``word & SIGN_BIT`` is set."""

MAGNITUDE_MASK = SIGN_BIT - 1
"""Mask to extract the 35-bit magnitude: ``0x0_07FF_FFFF_FFF``."""

ADDRESS_MASK = (1 << 15) - 1
"""Mask for 15-bit fields (address and decrement): ``0x7FFF``."""

TAG_MASK = 0b111
"""Mask for the 3-bit tag field."""

# ---------------------------------------------------------------------------
# Sign-magnitude integer helpers
# ---------------------------------------------------------------------------


def make_word(sign: int, magnitude: int) -> int:
    """Build a 36-bit word from a sign bit (0 or 1) and a 35-bit magnitude.

    Examples
    --------
    >>> make_word(0, 0)            # +0
    0
    >>> make_word(1, 0)            # -0 (distinct from +0)
    34359738368
    >>> make_word(0, 7) == 7
    True
    >>> make_word(1, 7) == (1 << 35) | 7
    True
    """
    if sign not in (0, 1):
        raise ValueError(f"sign must be 0 or 1, got {sign}")
    if not 0 <= magnitude <= MAGNITUDE_MASK:
        raise ValueError(
            f"magnitude must be in [0, 2**35-1], got {magnitude}"
        )
    return (sign << 35) | magnitude


def word_sign(word: int) -> int:
    """Return 1 if the word is negative (sign bit set), else 0.

    Note: ``-0`` (a word with sign=1, magnitude=0) has ``word_sign == 1`` even
    though it equals zero numerically. Most arithmetic instructions treat -0
    and +0 as equal in *value* but distinct in *bits*; ``TZE`` for example
    transfers on either zero.
    """
    return 1 if (word & SIGN_BIT) else 0


def word_magnitude(word: int) -> int:
    """Return the 35-bit unsigned magnitude of a word."""
    return word & MAGNITUDE_MASK


def word_to_signed_int(word: int) -> int:
    """Convert a 704 sign-magnitude word to a normal Python signed integer.

    Both ``+0`` and ``-0`` map to ``0`` — Python has no negative zero for
    integers, so the sign is lost on conversion. Use ``word_sign()`` /
    ``word_magnitude()`` if you need to preserve the distinction.

    Examples
    --------
    >>> word_to_signed_int(make_word(0, 5))
    5
    >>> word_to_signed_int(make_word(1, 5))
    -5
    >>> word_to_signed_int(make_word(1, 0))    # -0 → 0
    0
    """
    mag = word_magnitude(word)
    return -mag if word_sign(word) else mag


def signed_int_to_word(value: int) -> int:
    """Convert a Python signed integer to a 704 sign-magnitude word.

    Raises ``ValueError`` if the magnitude exceeds 35 bits (2**35 - 1 =
    34,359,738,367).

    Examples
    --------
    >>> signed_int_to_word(0)
    0
    >>> signed_int_to_word(-5) == make_word(1, 5)
    True
    """
    if value >= 0:
        if value > MAGNITUDE_MASK:
            raise OverflowError(
                f"value {value} exceeds 35-bit magnitude (max "
                f"{MAGNITUDE_MASK})"
            )
        return value
    mag = -value
    if mag > MAGNITUDE_MASK:
        raise OverflowError(
            f"value {value} exceeds 35-bit magnitude (min "
            f"-{MAGNITUDE_MASK})"
        )
    return SIGN_BIT | mag


# ---------------------------------------------------------------------------
# Sign-magnitude addition (the 704's heart)
# ---------------------------------------------------------------------------


def add_sign_magnitude(
    sign_a: int, mag_a: int, sign_b: int, mag_b: int
) -> tuple[int, int, bool]:
    """Add two sign-magnitude numbers, returning (result_sign, result_magnitude, overflow).

    Both operands are 35-bit magnitudes with separate sign bits. The result
    magnitude can be up to 36 bits (one carry beyond 35) — overflow is True
    if the result exceeds the 35-bit magnitude limit.

    Algorithm
    ---------
    * Same sign: ``mag = mag_a + mag_b``; sign keeps. Overflow if mag > 2**35-1.
    * Different signs: subtract the smaller magnitude from the larger; the
      result takes the sign of the larger operand. Cannot overflow because
      ``|a - b| <= max(|a|, |b|) <= 2**35 - 1``.

    Special case: ``+0 + (-0)`` and ``-0 + (+0)`` produce ``+0`` (not -0).
    This matches the IBM 704 *Reference Manual* (1955), p. 20.

    Examples
    --------
    >>> add_sign_magnitude(0, 3, 0, 4)         # 3 + 4 = 7
    (0, 7, False)
    >>> add_sign_magnitude(1, 3, 1, 4)         # -3 + -4 = -7
    (1, 7, False)
    >>> add_sign_magnitude(0, 3, 1, 4)         # 3 + -4 = -1
    (1, 1, False)
    >>> add_sign_magnitude(1, 3, 0, 4)         # -3 + 4 = 1
    (0, 1, False)
    >>> # Same-sign overflow:
    >>> add_sign_magnitude(0, MAGNITUDE_MASK, 0, 1)
    (0, 0, True)
    """
    if sign_a == sign_b:
        result_mag = mag_a + mag_b
        if result_mag > MAGNITUDE_MASK:
            # Overflow: keep low 35 bits, signal overflow.
            return sign_a, result_mag & MAGNITUDE_MASK, True
        if result_mag == 0:
            # 0 + 0 always normalizes to +0 per the manual.
            return 0, 0, False
        return sign_a, result_mag, False

    # Different signs — subtract smaller from larger.
    if mag_a >= mag_b:
        diff = mag_a - mag_b
        result_sign = sign_a if diff != 0 else 0  # canonicalize zero to +0
        return result_sign, diff, False
    diff = mag_b - mag_a
    return sign_b, diff, False


def negate_word(word: int) -> int:
    """Flip the sign bit of a word — 704 negation is just one bit flip.

    Distinct from two's complement, which would also flip every magnitude bit
    and add 1. On the 704, ``-(+3)`` is exactly ``(-3)`` and ``-(+0)`` is
    ``(-0)`` — both perfectly representable.
    """
    return word ^ SIGN_BIT


# ---------------------------------------------------------------------------
# Floating-point: word ↔ Python float
# ---------------------------------------------------------------------------

FP_SIGN_BIT = 1 << 35
FP_CHAR_SHIFT = 27
FP_CHAR_MASK = 0xFF
FP_FRAC_BITS = 27
FP_FRAC_MASK = (1 << FP_FRAC_BITS) - 1
FP_EXCESS = 128
"""Excess-128 bias for the characteristic. Actual exponent = char - 128."""


def fp_to_float(word: int) -> float:
    """Decode a 36-bit IBM 704 floating-point word to a Python float.

    The 704 format::

        bit 35: sign (S)
        bits 27-34: 8-bit characteristic (excess-128 exponent)
        bits 0-26: 27-bit fraction

    The value, if not zero, is::

        (-1)^S * fraction * 2^(characteristic - 128 - 27)

    A word is **floating-point zero** if both the characteristic and fraction
    are zero, regardless of sign. The 704 hardware did not produce -0.0 in
    floating-point arithmetic; the simulator follows that convention.

    Examples
    --------
    >>> fp_to_float(0)                        # +0.0
    0.0
    >>> # 1.0 = 0.5 * 2^1, so frac = 2^26, char = 129
    >>> fp_to_float(float_to_fp(1.0))
    1.0
    >>> fp_to_float(float_to_fp(-2.5))
    -2.5
    """
    sign = (word >> 35) & 1
    char = (word >> FP_CHAR_SHIFT) & FP_CHAR_MASK
    frac = word & FP_FRAC_MASK
    if char == 0 and frac == 0:
        return 0.0
    exponent = char - FP_EXCESS - FP_FRAC_BITS
    value = frac * (2.0 ** exponent)
    return -value if sign else value


def float_to_fp(value: float) -> int:
    """Encode a Python float to a 36-bit IBM 704 floating-point word.

    Uses ``math.frexp`` to find a normalized fraction in [0.5, 1.0) and the
    corresponding power of two. The normalized fraction is multiplied by
    2**27 and rounded to nearest to fit in the 27-bit fraction field.

    Saturates on overflow (characteristic > 255 → maximum representable
    magnitude with sign preserved). Returns ``+0`` on underflow
    (characteristic < 0).

    Examples
    --------
    >>> float_to_fp(0.0) == 0
    True
    >>> # Round-trip for clean powers of two:
    >>> for v in [1.0, 2.0, 0.5, -1.5, 3.25]:
    ...     assert fp_to_float(float_to_fp(v)) == v, v
    """
    if value == 0.0 or math.isnan(value) or math.isinf(value):
        # NaN/Inf are not representable on a 1955 machine; collapse to +0
        # rather than producing nonsense bits. The simulator's overflow
        # trigger is the right place to signal infinities at runtime.
        return 0

    sign_bit = 1 if value < 0 else 0
    mag = abs(value)
    mantissa, exponent = math.frexp(mag)  # 0.5 <= mantissa < 1.0
    char = exponent + FP_EXCESS
    frac = int(round(mantissa * (1 << FP_FRAC_BITS)))
    if frac >= (1 << FP_FRAC_BITS):  # rounded up across the boundary
        frac >>= 1
        char += 1
    if char < 0:
        # Underflow — value too small to represent. Collapse to ±0.
        return sign_bit << 35
    if char > FP_CHAR_MASK:
        # Overflow — saturate to largest magnitude with sign preserved.
        return (
            (sign_bit << 35) | (FP_CHAR_MASK << FP_CHAR_SHIFT) | FP_FRAC_MASK
        )
    return (sign_bit << 35) | (char << FP_CHAR_SHIFT) | (frac & FP_FRAC_MASK)


# ---------------------------------------------------------------------------
# Word transport — packed big-endian 5-byte groups
# ---------------------------------------------------------------------------

WORD_BYTES = 5
"""Bytes per word in the byte-stream encoding used by ``Simulator.load``."""


def pack_word(word: int) -> bytes:
    """Pack one 36-bit word as 5 big-endian bytes (top 4 bits of byte 0 = 0).

    Examples
    --------
    >>> pack_word(0).hex()
    '0000000000'
    >>> pack_word(WORD_MASK).hex()        # all 36 bits set
    '0fffffffff'
    >>> unpack_word(pack_word(0x123456789)) == 0x123456789
    True
    """
    if not 0 <= word <= WORD_MASK:
        raise ValueError(f"word {word!r} does not fit in 36 bits")
    return word.to_bytes(WORD_BYTES, "big")


def unpack_word(b: bytes) -> int:
    """Unpack 5 big-endian bytes into a 36-bit word, ignoring the top 4 bits.

    Raises ``ValueError`` if ``len(b) != 5`` or if the top 4 bits of byte 0
    are non-zero (indicating a malformed transport).
    """
    if len(b) != WORD_BYTES:
        raise ValueError(f"need exactly {WORD_BYTES} bytes, got {len(b)}")
    raw = int.from_bytes(b, "big")
    if raw >> WORD_BITS:
        raise ValueError(
            f"top 4 bits of packed word must be zero; got {b.hex()}"
        )
    return raw


def pack_program(words: list[int]) -> bytes:
    """Pack a list of 36-bit words into a byte stream for ``Simulator.load``.

    Examples
    --------
    >>> packed = pack_program([0, 1, 2])
    >>> len(packed)
    15
    >>> packed.hex()
    '000000000000000000010000000002'
    """
    return b"".join(pack_word(w) for w in words)


def unpack_program(b: bytes) -> list[int]:
    """Unpack a byte stream into a list of 36-bit words.

    Raises ``ValueError`` if the length is not a multiple of 5.
    """
    if len(b) % WORD_BYTES != 0:
        raise ValueError(
            f"program length {len(b)} is not a multiple of {WORD_BYTES}"
        )
    return [
        unpack_word(b[i : i + WORD_BYTES])
        for i in range(0, len(b), WORD_BYTES)
    ]
