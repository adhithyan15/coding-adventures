"""Deterministic randomness and hashing for test runs (TF01 §5.1, VER05).

Pure Python implementation of SplitMix64 PRNG and 64-bit FNV-1a hash.
Zero external dependencies.
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

MASK64 = 0xFFFF_FFFF_FFFF_FFFF
GOLDEN_GAMMA = 0x9E37_79B9_7F4A_7C15
MIX_MUL1 = 0xBF58_476D_1CE4_E5B9
MIX_MUL2 = 0x94D0_49BB_1331_11EB
FNV_OFFSET_BASIS = 0xCBF2_9CE4_8422_2325
FNV_PRIME = 0x0000_0100_0000_01B3


def _check_strict_int(val: Any, name: str) -> None:
    if isinstance(val, bool) or not isinstance(val, int):
        raise TypeError(f"{name} must be an integer, not {type(val).__name__}")


class SplitMix64:
    """SplitMix64 pseudorandom generator.

    Provides exact 64-bit reproducible pseudo-random numbers and unbiased
    rejection-sampled bounded integers.
    """

    __slots__ = ("_state",)

    def __init__(self, seed: int = 0) -> None:
        _check_strict_int(seed, "seed")
        self._state: int = seed & MASK64

    @property
    def state(self) -> int:
        """Current internal 64-bit state."""
        return self._state

    @state.setter
    def state(self, value: int) -> None:
        _check_strict_int(value, "state")
        self._state = value & MASK64

    def next_u64(self) -> int:
        """Produce the next 64-bit pseudo-random unsigned integer."""
        self._state = (self._state + GOLDEN_GAMMA) & MASK64
        z = self._state
        z = ((z ^ (z >> 30)) * MIX_MUL1) & MASK64
        z = ((z ^ (z >> 27)) * MIX_MUL2) & MASK64
        return (z ^ (z >> 31)) & MASK64

    def next_float(self) -> float:
        """Produce a float in [0.0, 1.0) with 53 bits of precision."""
        return (self.next_u64() >> 11) * (1.0 / (1 << 53))

    def bounded(self, n: int) -> int:
        """Produce an unbiased random integer in range [0, n).

        Uses rejection sampling to ensure uniform distribution across all n.
        Enforces 1 <= n <= 2^64.
        """
        _check_strict_int(n, "bounded(n)")
        if n <= 0:
            raise ValueError(f"bounded(n) requires n >= 1, got {n}")
        if n > (1 << 64):
            raise ValueError(f"bounded(n) requires n <= 2^64, got {n}")
        if n == 1:
            return 0
        if n == (1 << 64):
            return self.next_u64()

        # Rejection sampling threshold t = (2^64 - n) mod n.
        #
        # Why not `(-n) % n`?  In C or Rust, `-n` on an unsigned 64-bit
        # integer wraps to 2^64 - n, so `(-n) % n` is the threshold.  In
        # Python `-n` is the negative number, and floor-modulo makes
        # `(-n) % n` exactly 0 for every n: no draw would ever be rejected
        # and every span that does not divide 2^64 would be biased.  Spell the
        # 2^64 out.
        #
        # Worked example, n = 3: 2^64 = 3 * 6148914691236517205 + 1, so t = 1.
        # The single value x = 0 is thrown away and the
        # remaining 2^64 - 1 values split evenly into 3 residue classes.
        threshold = ((1 << 64) - n) % n
        while True:
            x = self.next_u64()
            if x >= threshold:
                return x % n

    def bounded_range(self, min_val: int, max_val: int) -> int:
        """Produce an unbiased integer in closed range [min_val, max_val]."""
        _check_strict_int(min_val, "min_val")
        _check_strict_int(max_val, "max_val")
        if min_val > max_val:
            raise ValueError(f"min_val ({min_val}) cannot exceed max_val ({max_val})")
        span = max_val - min_val + 1
        return min_val + self.bounded(span)


def fnv1a64(data: bytes | str) -> int:
    """Compute 64-bit FNV-1a hash of bytes or UTF-8 text."""
    if isinstance(data, str):
        buf = data.encode("utf-8")
    elif isinstance(data, (bytes, bytearray)):
        buf = bytes(data)
    else:
        raise TypeError(f"fnv1a64 expected bytes or str, got {type(data).__name__}")

    h = FNV_OFFSET_BASIS
    for b in buf:
        h = ((h ^ b) * FNV_PRIME) & MASK64
    return h


def shuffle[T](items: Sequence[T], rng: SplitMix64) -> list[T]:
    """Deterministically shuffle items using Fisher-Yates driven by SplitMix64."""
    out = list(items)
    for i in range(len(out) - 1, 0, -1):
        j = rng.bounded(i + 1)
        out[i], out[j] = out[j], out[i]
    return out
