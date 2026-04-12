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


# =============================================================================
# GF256Field — Parameterizable Field Factory
# =============================================================================
#
# The functions above are bound to the Reed-Solomon polynomial 0x11D.
# AES uses the polynomial 0x11B instead. Rather than duplicating all the
# field arithmetic, GF256Field accepts any primitive polynomial and builds
# its own independent log/antilog tables.
#
# Usage:
#   aes_field = GF256Field(0x11B)
#   rs_field  = GF256Field(0x11D)   # same as module-level functions
#
#   aes_field.multiply(0x53, 0x8C)  # → 0x01 in AES GF(2^8)
#   aes_field.inverse(0x53)         # → AES S-box input
#
# The module-level functions remain the canonical Reed-Solomon API and are
# not changed by creating a GF256Field instance.


class GF256Field:
    """A Galois Field GF(2^8) configured for a specific primitive polynomial.

    The module-level functions are fixed to the Reed-Solomon polynomial 0x11D.
    This class lets you instantiate a field with any primitive polynomial —
    most notably 0x11B for AES.

    The log/antilog tables are built once in __init__ and cached as instance
    attributes. All field operations are O(1) table lookups.

    Args:
        polynomial: The irreducible polynomial for modular reduction,
                    represented as an integer. The degree-8 term is included:
                    AES uses 0x11B (x^8+x^4+x^3+x+1),
                    Reed-Solomon uses 0x11D (x^8+x^4+x^3+x^2+1).

    Examples:
        >>> f = GF256Field(0x11B)
        >>> f.multiply(0x53, 0x8C)   # AES field: 0x53 and 0x8C are inverses
        1
        >>> f.multiply(0x57, 0x83)   # FIPS 197 Appendix B
        193
    """

    def __init__(self, polynomial: int) -> None:
        self.polynomial = polynomial
        self._log, self._alog = self._build_tables(polynomial)

    @staticmethod
    def _build_tables(poly: int) -> tuple[tuple[int, ...], tuple[int, ...]]:
        """Build log/antilog tables for the given polynomial."""
        log_table = [0] * 256
        alog_table = [0] * 256
        val = 1
        for i in range(255):
            alog_table[i] = val
            log_table[val] = i
            val <<= 1
            if val >= 256:
                val ^= poly
        alog_table[255] = 1  # g^255 = g^0 = 1 (group wraps)
        return tuple(log_table), tuple(alog_table)

    # add/subtract are polynomial-independent (always XOR), but included for
    # API symmetry so callers only need one object.

    def add(self, a: GF256, b: GF256) -> GF256:
        """Add two field elements: a XOR b (characteristic-2 addition)."""
        return a ^ b

    def subtract(self, a: GF256, b: GF256) -> GF256:
        """Subtract two field elements: a XOR b (same as add in GF(2^8))."""
        return a ^ b

    def multiply(self, a: GF256, b: GF256) -> GF256:
        """Multiply two field elements using log/antilog tables."""
        if a == 0 or b == 0:
            return 0
        return self._alog[(self._log[a] + self._log[b]) % 255]

    def divide(self, a: GF256, b: GF256) -> GF256:
        """Divide a by b.  Raises ValueError if b is 0."""
        if b == 0:
            raise ValueError("GF256Field: division by zero")
        if a == 0:
            return 0
        return self._alog[(self._log[a] - self._log[b] + 255) % 255]

    def power(self, base: GF256, exp: int) -> GF256:
        """Raise base to a non-negative integer power."""
        if base == 0:
            return 1 if exp == 0 else 0
        if exp == 0:
            return 1
        return self._alog[(self._log[base] * exp % 255 + 255) % 255]

    def inverse(self, a: GF256) -> GF256:
        """Return the multiplicative inverse of a.  Raises ValueError if a is 0."""
        if a == 0:
            raise ValueError("GF256Field: zero has no multiplicative inverse")
        return self._alog[255 - self._log[a]]
