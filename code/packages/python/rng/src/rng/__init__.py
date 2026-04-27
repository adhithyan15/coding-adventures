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

__version__ = "0.1.0"

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

        Uses rejection sampling to eliminate modulo bias.
        ``threshold = (-range_size) % (1 << 32) % range_size``

        Any draw below ``threshold`` is discarded; expected extra draws < 2.
        """
        if min_val > max_val:
            raise ValueError(
                f"next_int_in_range requires min_val <= max_val, "
                f"got {min_val} > {max_val}"
            )
        range_size = max_val - min_val + 1
        # Rejection threshold eliminates modulo bias.
        # (-range_size) % (1 << 32) % range_size gives the number of values
        # we must reject so the remaining values divide evenly into range_size
        # bins.
        threshold = (-range_size) % (1 << 32) % range_size
        while True:
            r = self.next_u32()
            if r >= threshold:
                return min_val + (r % range_size)


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

        Identical rejection-sampling algorithm to :meth:`LCG.next_int_in_range`.
        """
        if min_val > max_val:
            raise ValueError(
                f"next_int_in_range requires min_val <= max_val, "
                f"got {min_val} > {max_val}"
            )
        range_size = max_val - min_val + 1
        threshold = (-range_size) % (1 << 32) % range_size
        while True:
            r = self.next_u32()
            if r >= threshold:
                return min_val + (r % range_size)


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

        Identical rejection-sampling algorithm to :meth:`LCG.next_int_in_range`.
        """
        if min_val > max_val:
            raise ValueError(
                f"next_int_in_range requires min_val <= max_val, "
                f"got {min_val} > {max_val}"
            )
        range_size = max_val - min_val + 1
        threshold = (-range_size) % (1 << 32) % range_size
        while True:
            r = self.next_u32()
            if r >= threshold:
                return min_val + (r % range_size)
