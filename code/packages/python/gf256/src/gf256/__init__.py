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
# field arithmetic, GF256Field accepts any primitive polynomial and uses
# Russian peasant (shift-and-XOR) multiplication — an algorithm that works
# correctly for ANY irreducible polynomial without assuming a specific generator.
#
# Why not log/antilog tables? The log table approach requires a *primitive*
# element g such that g, g^2, ..., g^255 cycles through all 255 non-zero
# elements. g = 2 (the polynomial x) works for 0x11D but is NOT a primitive
# element for 0x11B (AES uses g = 0x03 = x+1 per FIPS 197 §4.1). Using g=2
# with 0x11B produces incomplete tables and wrong results for most elements.
#
# Russian peasant multiplication needs no generator assumption:
#
#   gf_mul(a, b, reduce):
#     result = 0
#     for each bit of b (LSB first):
#       if bit is set: result ^= a
#       carry = a & 0x80
#       a = (a << 1) & 0xFF
#       if carry: a ^= reduce       # reduce mod p(x): XOR low byte of polynomial
#     return result
#
# 'reduce' is the low byte of the polynomial (e.g. 0x11B & 0xFF = 0x1B).
#
# Usage:
#   aes_field = GF256Field(0x11B)
#   rs_field  = GF256Field(0x11D)   # same as module-level functions
#
#   aes_field.multiply(0x53, 0xCA)  # → 0x01 in AES GF(2^8)
#   aes_field.inverse(0x53)         # → 0xCA (AES S-box input)
#
# The module-level functions remain the canonical Reed-Solomon API and are
# not changed by creating a GF256Field instance.


class GF256Field:
    """A Galois Field GF(2^8) configured for a specific primitive polynomial.

    The module-level functions are fixed to the Reed-Solomon polynomial 0x11D.
    This class lets you instantiate a field with any polynomial — most notably
    0x11B for AES.

    Operations use Russian peasant multiplication — no log/antilog tables.
    This approach works correctly for any irreducible polynomial regardless of
    which element is a primitive generator.

    Args:
        polynomial: The irreducible polynomial for modular reduction,
                    represented as an integer. The degree-8 term is included:
                    AES uses 0x11B (x^8+x^4+x^3+x+1),
                    Reed-Solomon uses 0x11D (x^8+x^4+x^3+x^2+1).

    Examples:
        >>> f = GF256Field(0x11B)
        >>> f.multiply(0x53, 0xCA)   # AES field: 0x53 and 0xCA are inverses
        1
        >>> f.multiply(0x57, 0x83)   # FIPS 197 Appendix B
        193
    """

    def __init__(self, polynomial: int) -> None:
        self.polynomial = polynomial
        # Store only the low byte for the reduction step in Russian peasant.
        # When a << 1 overflows 8 bits we XOR with this constant to stay in GF(2^8).
        self._reduce = polynomial & 0xFF

    @staticmethod
    def _gf_mul(a: int, b: int, reduce: int) -> int:
        """Multiply a and b in GF(2^8) using Russian peasant (shift-and-XOR)."""
        result = 0
        aa = a
        for _ in range(8):
            if b & 1:
                result ^= aa
            hi = aa & 0x80
            aa = (aa << 1) & 0xFF
            if hi:
                aa ^= reduce
            b >>= 1
        return result

    @staticmethod
    def _gf_pow(base: int, exp: int, reduce: int) -> int:
        """Raise base to exp in GF(2^8) via repeated squaring."""
        if base == 0:
            return 1 if exp == 0 else 0
        if exp == 0:
            return 1
        result = 1
        while exp > 0:
            if exp & 1:
                result = GF256Field._gf_mul(result, base, reduce)
            base = GF256Field._gf_mul(base, base, reduce)
            exp >>= 1
        return result

    # add/subtract are polynomial-independent (always XOR), but included for
    # API symmetry so callers only need one object.

    def add(self, a: GF256, b: GF256) -> GF256:
        """Add two field elements: a XOR b (characteristic-2 addition)."""
        return a ^ b

    def subtract(self, a: GF256, b: GF256) -> GF256:
        """Subtract two field elements: a XOR b (same as add in GF(2^8))."""
        return a ^ b

    def multiply(self, a: GF256, b: GF256) -> GF256:
        """Multiply two field elements using Russian peasant multiplication."""
        return self._gf_mul(a, b, self._reduce)

    def divide(self, a: GF256, b: GF256) -> GF256:
        """Divide a by b.  Raises ValueError if b is 0."""
        if b == 0:
            raise ValueError("GF256Field: division by zero")
        return self._gf_mul(a, self._gf_pow(b, 254, self._reduce), self._reduce)

    def power(self, base: GF256, exp: int) -> GF256:
        """Raise base to a non-negative integer power."""
        if exp < 0:
            raise ValueError("GF256Field: exponent must be non-negative")
        if exp > 0xFFFF_FFFF:
            raise ValueError("GF256Field: exponent too large (max 2^32 - 1)")
        return self._gf_pow(base, exp, self._reduce)

    def inverse(self, a: GF256) -> GF256:
        """Return the multiplicative inverse of a.  Raises ValueError if a is 0."""
        if a == 0:
            raise ValueError("GF256Field: zero has no multiplicative inverse")
        # a^254 = a^(-1) since a^255 = 1 in GF(2^8) (Fermat's little theorem).
        return self._gf_pow(a, 254, self._reduce)
