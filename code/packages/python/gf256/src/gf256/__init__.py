"""
gf256 — Galois Field GF(2^8) arithmetic.

GF(256) is the finite field with 256 elements. Elements are integers 0..255.
Arithmetic uses the primitive polynomial:

    p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285

Applications:
  - Reed-Solomon error correction (QR codes, CDs, hard drives)
  - AES encryption (SubBytes and MixColumns use GF(2^8))
  - General error-correcting codes

Key insight: In GF(2^8), addition IS XOR. Subtraction equals addition.
Multiplication uses log/antilog tables: O(1) cost.
"""

from __future__ import annotations

VERSION = "0.1.0"

# The type of a GF(256) element: an integer in [0, 255].
GF256 = int

# Additive identity.
ZERO: GF256 = 0

# Multiplicative identity.
ONE: GF256 = 1

# Primitive polynomial: x^8 + x^4 + x^3 + x^2 + 1.
# Binary: 1_0001_1101 = 0x11D = 285.
# Irreducible over GF(2) — ensures every non-zero element has an inverse.
# Primitive — the element g=2 generates the full multiplicative group of order 255.
PRIMITIVE_POLYNOMIAL: int = 0x11D


# =============================================================================
# Log/Antilog Table Construction
# =============================================================================
#
# Two lookup tables are built at import time:
#
#   ALOG[i] = g^i mod p(x)   where g = 2
#   LOG[x]  = i such that g^i = x
#
# Construction: start with val=1; each step shift left 1 (= multiply by g=x);
# if val >= 256 (bit 8 set), XOR with 0x11D to reduce modulo the primitive poly.


def _build_tables() -> tuple[tuple[int, ...], tuple[int, ...]]:
    """Build and return (LOG, ALOG) tables.

    ALOG has 256 entries: indices 0..254 are the standard table;
    ALOG[255] = 1 because g^255 = g^0 = 1 (the group has order 255).
    This allows inverse(1) = ALOG[255 - LOG[1]] = ALOG[255] = 1 to work.
    """
    log_table = [0] * 256
    alog_table = [0] * 256  # extra slot for ALOG[255] = 1
    val = 1
    for i in range(255):
        alog_table[i] = val
        log_table[val] = i
        val <<= 1  # multiply by g = 2
        if val >= 256:  # overflow: reduce mod primitive polynomial
            val ^= PRIMITIVE_POLYNOMIAL
    alog_table[255] = 1  # g^255 = g^0 = 1 (group wraps)
    return tuple(log_table), tuple(alog_table)


LOG: tuple[int, ...] = ()
ALOG: tuple[int, ...] = ()
LOG, ALOG = _build_tables()


# =============================================================================
# Field Operations
# =============================================================================


def add(a: GF256, b: GF256) -> GF256:
    """Add two GF(256) elements: returns a XOR b.

    In characteristic 2, addition is XOR. Each bit represents a GF(2)
    coefficient, and 1 + 1 = 0 (mod 2). No carry, no tables needed.

    Examples:
        >>> add(0x53, 0xCA)
        249
        >>> add(5, 5)
        0
        >>> add(0, 7)
        7
    """
    return a ^ b


def subtract(a: GF256, b: GF256) -> GF256:
    """Subtract two GF(256) elements: returns a XOR b.

    In characteristic 2, -1 = 1, so subtraction equals addition.
    This simplifies error-correction algorithms considerably.

    Examples:
        >>> subtract(7, 7)
        0
        >>> subtract(5, 3)
        6
    """
    return a ^ b


def multiply(a: GF256, b: GF256) -> GF256:
    """Multiply two GF(256) elements using log/antilog tables.

    a × b = g^(LOG[a] + LOG[b]) = ALOG[(LOG[a] + LOG[b]) % 255]

    Special case: if either operand is 0, the result is 0.
    (Zero has no logarithm; the formula does not apply.)

    Time complexity: O(1) — two table lookups and one addition.

    Examples:
        >>> multiply(0x53, 0xCA)
        1
        >>> multiply(5, 0)
        0
        >>> multiply(1, 42)
        42
    """
    if a == 0 or b == 0:
        return 0
    return ALOG[(LOG[a] + LOG[b]) % 255]


def divide(a: GF256, b: GF256) -> GF256:
    """Divide a by b in GF(256).

    a / b = ALOG[(LOG[a] - LOG[b] + 255) % 255]

    The +255 before % 255 ensures a non-negative result when LOG[a] < LOG[b].

    Special case: a = 0 → result is 0.

    Args:
        a: Dividend.
        b: Divisor (must not be 0).

    Raises:
        ValueError: If b is 0.

    Examples:
        >>> divide(0, 5)
        0
        >>> divide(1, 1)
        1
    """
    if b == 0:
        raise ValueError("GF256: division by zero")
    if a == 0:
        return 0
    return ALOG[(LOG[a] - LOG[b] + 255) % 255]


def power(base: GF256, exp: int) -> GF256:
    """Raise a GF(256) element to a non-negative integer power.

    base^exp = ALOG[(LOG[base] * exp) % 255]

    The % 255 reflects the order of the multiplicative group:
    every non-zero element satisfies g^255 = 1.

    Special cases:
      - 0^0 = 1 by convention
      - 0^n = 0 for n > 0

    Examples:
        >>> power(2, 0)
        1
        >>> power(2, 255)
        1
        >>> power(0, 5)
        0
    """
    if base == 0:
        return 1 if exp == 0 else 0
    if exp == 0:
        return 1
    return ALOG[(LOG[base] * exp % 255 + 255) % 255]


def inverse(a: GF256) -> GF256:
    """Return the multiplicative inverse of a GF(256) element.

    a × inverse(a) = 1.

    By the cyclic group property:
        log(a) + log(a^-1) ≡ 0 (mod 255)
        log(a^-1) = 255 - log(a)
        a^-1 = ALOG[255 - LOG[a]]

    Args:
        a: Field element (must not be 0).

    Raises:
        ValueError: If a is 0 (zero has no multiplicative inverse).

    Examples:
        >>> inverse(1)
        1
        >>> multiply(0x53, inverse(0x53))
        1
    """
    if a == 0:
        raise ValueError("GF256: zero has no multiplicative inverse")
    return ALOG[255 - LOG[a]]


def zero() -> GF256:
    """Return the additive identity (0)."""
    return 0


def one() -> GF256:
    """Return the multiplicative identity (1)."""
    return 1
