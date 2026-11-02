"""rng — LCG, Xorshift64, and PCG32 pseudorandom number generators.

This module implements three classic PRNGs that all produce identical output
to the Go reference implementation for the same seed.

# The Three Algorithms

## LCG — Linear Congruential Generator (Knuth 1948)

    state_{n+1} = (state_n × a + c)  mod 2^64

Where a = 6364136223846793005 and c = 1442695040888963407 (Knuth/Numerical
Recipes constants; satisfy Hull-Dobell theorem → full period 2^64).

Output: upper 32 bits of state (lower bits have shorter sub-periods).

## Xorshift64 (Marsaglia 2003)

    x ^= x << 13
    x ^= x >> 7
    x ^= x << 17

Period: 2^64 − 1. Seed 0 replaced with 1 (0 is a fixed point). Output: lower
32 bits.

## PCG32 (O'Neill 2014)

Same LCG recurrence plus XSH RR output permutation (XOR-Shift High / Random
Rotate). Passes all known statistical test suites (TestU01 BigCrush,
PractRand).

# Reference values for seed = 1

| Call | LCG        | Xorshift64 | PCG32      |
|------|------------|------------|------------|
| 1st  | 1817669548 | 1082269761 | 1412771199 |
| 2nd  | 2187888307 | 201397313  | 1791099446 |
| 3rd  | 2784682393 | 1854285353 | 124312908  |

# Python-specific notes

Python integers are arbitrary precision. We mask with `& _MASK64` after every
multiplication/addition to emulate 64-bit unsigned wrapping, and with
`& _MASK32` for 32-bit outputs — exactly what C would do with uint64_t/uint32_t.

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
"""

from __future__ import annotations

from collections.abc import Callable

__version__ = "0.2.0"

# ── Constants ──────────────────────────────────────────────────────────────────

#: Knuth/Numerical Recipes LCG multiplier.
_LCG_MULTIPLIER: int = 6364136223846793005

#: LCG increment — must be odd for full period (already is).
_LCG_INCREMENT: int = 1442695040888963407

#: Bitmask to keep arithmetic in 64-bit unsigned range.
_MASK64: int = (1 << 64) - 1

#: Bitmask for 32-bit unsigned output.
_MASK32: int = (1 << 32) - 1

#: Normalisation divisor: float output = u32 / 2^32.
_FLOAT_DIV: float = 2**32


# ── LCG ───────────────────────────────────────────────────────────────────────


# ── Bounded draws (spec rng.md, "Rejection Sampling", v2) ─────────────────────

#: The signed 64-bit domain every lane shares for ``next_int_in_range``.
_I64_MIN: int = -(1 << 63)
_I64_MAX: int = (1 << 63) - 1


def _int_in_range(
    next_u32: Callable[[], int], next_u64: Callable[[], int], min_val: int, max_val: int
) -> int:
    """Uniform integer in ``[min_val, max_val]``, identical in every language.

    ``span = max_val - min_val + 1`` is between 1 and 2^64. Taking ``r % span``
    of a raw draw over-samples small offsets whenever the number of possible
    draws is not a multiple of ``span``; rejecting the ``threshold`` lowest
    draws removes exactly the excess::

        span <= 2^32      draw 32 bits,  threshold = (2^32 - span) % span
        span <  2^64      draw 64 bits,  threshold = (2^64 - span) % span
        span == 2^64      every 64-bit draw is a valid offset: no rejection

    Example, span = 3 in the 32-bit regime: 2^32 = 3 * 1431655765 + 1, so
    threshold = 1. Rejecting r = 0 leaves 2^32 - 1 draws, an exact multiple
    of 3, and each offset 0, 1, 2 then has the same 1431655765 chances.

    The 32-bit regime is the original (v1) algorithm, so ranges of up to 2^32
    values give the same sequence as before. v1 used it for every span, which
    could never reach more than 2^32 - 1 above ``min_val``.

    Bounds outside the signed 64-bit domain raise :class:`ValueError`, as
    ``min_val > max_val`` does; Python's integers are unbounded, but the
    spec's are not, and every lane must agree. Errors are raised before any
    draw, so a rejected call leaves the generator untouched.
    """
    if not (_I64_MIN <= min_val <= _I64_MAX and _I64_MIN <= max_val <= _I64_MAX):
        raise ValueError(
            f"next_int_in_range bounds must be signed 64-bit integers, "
            f"got [{min_val}, {max_val}]"
        )
    if min_val > max_val:
        raise ValueError(
            f"next_int_in_range requires min_val <= max_val, got {min_val} > {max_val}"
        )
    span = max_val - min_val + 1
    if span <= 1 << 32:
        threshold = ((1 << 32) - span) % span
        while True:
            r = next_u32()
            if r >= threshold:
                return min_val + r % span
    if span < 1 << 64:
        threshold = ((1 << 64) - span) % span
        while True:
            r = next_u64()
            if r >= threshold:
                return min_val + r % span
    return min_val + next_u64()


class LCG:
    """Linear Congruential Generator (Knuth 1948).

    Recurrence: ``state = (state × a + c) mod 2^64``

    - **Period:** 2^64 — every 64-bit value appears exactly once per cycle.
    - **Output:** upper 32 bits (lower bits have shorter sub-periods).
    - **Weakness:** consecutive outputs are linearly correlated.

    Example::

        g = LCG(1)
        g.next_u32()  # → 1817669548
        g.next_u32()  # → 2187888307
    """

    __slots__ = ("_state",)

    def __init__(self, seed: int) -> None:
        """Seed the generator. Any non-negative integer is valid."""
        self._state: int = seed & _MASK64

    def next_u32(self) -> int:
        """Advance state; return upper 32 bits as an ``int`` in ``[0, 2^32)``."""
        self._state = (self._state * _LCG_MULTIPLIER + _LCG_INCREMENT) & _MASK64
        return self._state >> 32

    def next_u64(self) -> int:
        """Return a 64-bit value: ``(hi << 32) | lo`` from two ``next_u32`` calls."""
        hi = self.next_u32()
        lo = self.next_u32()
        return (hi << 32) | lo

    def next_float(self) -> float:
        """Return a ``float`` uniformly distributed in ``[0.0, 1.0)``."""
        return self.next_u32() / _FLOAT_DIV

    def next_int_in_range(self, min_val: int, max_val: int) -> int:
        """Return a uniform random integer in ``[min_val, max_val]`` inclusive.

        Both bounds must be signed 64-bit integers with ``min_val <= max_val``.
        See :func:`_int_in_range` for the three rejection-sampling regimes.
        """
        return _int_in_range(self.next_u32, self.next_u64, min_val, max_val)


# ── Xorshift64 ────────────────────────────────────────────────────────────────


class Xorshift64:
    """Xorshift64 generator (Marsaglia 2003).

    Three XOR-shift operations permute all 64 bits of state without any
    multiplication — making this the fastest of the three generators::

        x ^= x << 13
        x ^= x >> 7
        x ^= x << 17

    - **Period:** 2^64 − 1.
    - **Fixed point:** state 0 → always produces 0. Seed 0 is replaced with 1.
    - **Output:** lower 32 bits.

    Example::

        g = Xorshift64(1)
        g.next_u32()  # → 1082269761
    """

    __slots__ = ("_state",)

    def __init__(self, seed: int) -> None:
        """Seed the generator. Seed 0 is replaced with 1."""
        s = seed & _MASK64
        self._state: int = s if s != 0 else 1

    def next_u32(self) -> int:
        """Apply three XOR-shifts; return lower 32 bits."""
        x = self._state
        x ^= (x << 13) & _MASK64
        x ^= x >> 7
        x ^= (x << 17) & _MASK64
        self._state = x
        return x & _MASK32

    def next_u64(self) -> int:
        """Return a 64-bit value: ``(hi << 32) | lo`` from two ``next_u32`` calls."""
        hi = self.next_u32()
        lo = self.next_u32()
        return (hi << 32) | lo

    def next_float(self) -> float:
        """Return a ``float`` uniformly distributed in ``[0.0, 1.0)``."""
        return self.next_u32() / _FLOAT_DIV

    def next_int_in_range(self, min_val: int, max_val: int) -> int:
        """Return a uniform random integer in ``[min_val, max_val]`` inclusive.

        Both bounds must be signed 64-bit integers with ``min_val <= max_val``.
        See :func:`_int_in_range` for the three rejection-sampling regimes.
        """
        return _int_in_range(self.next_u32, self.next_u64, min_val, max_val)


# ── PCG32 ─────────────────────────────────────────────────────────────────────


class PCG32:
    """Permuted Congruential Generator (O'Neill 2014).

    Uses the same LCG recurrence as :class:`LCG` but applies an XSH RR
    (XOR-Shift High / Random Rotate) output permutation::

        xorshifted = ((old >> 18) ^ old) >> 27   # mix high bits down
        rot        = old >> 59                    # 5-bit rotation amount
        output     = rotr32(xorshifted, rot)      # scatter all bits

    The permutation is applied to the state *before* advancing (output-before-
    advance), which breaks the linear correlation between outputs.

    - **Period:** 2^64.
    - **Quality:** passes all known statistical test suites.
    - **initseq warm-up:** state=0 → advance → add seed → advance.

    Example::

        g = PCG32(1)
        g.next_u32()  # → 1412771199
    """

    __slots__ = ("_state", "_increment")

    def __init__(self, seed: int) -> None:
        """Seed with initseq warm-up: advance twice around adding seed."""
        inc = _LCG_INCREMENT | 1  # must be odd (already is)
        self._increment: int = inc
        # Step 1: advance once from state=0
        state = (0 * _LCG_MULTIPLIER + inc) & _MASK64
        # Step 2: mix seed in
        state = (state + (seed & _MASK64)) & _MASK64
        # Step 3: advance once more to scatter seed bits
        state = (state * _LCG_MULTIPLIER + inc) & _MASK64
        self._state: int = state

    def next_u32(self) -> int:
        """Advance LCG; return XSH RR permuted output of old state."""
        old = self._state
        self._state = (old * _LCG_MULTIPLIER + self._increment) & _MASK64

        # XSH RR permutation ──────────────────────────────────────────────
        # Step 1: mix high bits into lower 32.
        xorshifted = (((old >> 18) ^ old) >> 27) & _MASK32

        # Step 2: 5-bit rotation amount from top 5 bits.
        rot = old >> 59

        # Step 3: rotate right by rot positions.
        #   rotr32(x, n) = (x >> n) | (x << (32-n))
        # We compute the left-rotate amount as (-rot) & 31, which equals
        # (32 - rot) when rot != 0, and 0 when rot == 0.
        left_rot = (-rot) & 31
        return ((xorshifted >> rot) | ((xorshifted << left_rot) & _MASK32)) & _MASK32

    def next_u64(self) -> int:
        """Return a 64-bit value: ``(hi << 32) | lo`` from two ``next_u32`` calls."""
        hi = self.next_u32()
        lo = self.next_u32()
        return (hi << 32) | lo

    def next_float(self) -> float:
        """Return a ``float`` uniformly distributed in ``[0.0, 1.0)``."""
        return self.next_u32() / _FLOAT_DIV

    def next_int_in_range(self, min_val: int, max_val: int) -> int:
        """Return a uniform random integer in ``[min_val, max_val]`` inclusive.

        Both bounds must be signed 64-bit integers with ``min_val <= max_val``.
        See :func:`_int_in_range` for the three rejection-sampling regimes.
        """
        return _int_in_range(self.next_u32, self.next_u64, min_val, max_val)
