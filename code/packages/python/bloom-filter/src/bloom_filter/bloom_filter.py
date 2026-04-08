"""
bloom_filter.py — Bloom filter for probabilistic set membership testing.

A Bloom filter answers "Have I seen this element before?" with two possible
answers:

  "Definitely NO"  — zero false negatives. Trust it completely. If the filter
                     says NO, the element was never added.

  "Probably YES"   — small, tunable probability of false positives. The filter
                     says YES, but occasionally the element was never added.

This asymmetry is extremely useful as a pre-flight check before expensive
operations (disk reads, network requests, cache lookups). If the filter says
NO, skip the expensive operation entirely. If it says YES, do the operation
(and occasionally discover a false positive, which is acceptable).

Real-world deployments:
  - LevelDB / RocksDB / Cassandra — avoid disk seeks for missing SSTable keys
  - Chrome Safe Browsing — local check before network call to Google's servers
  - Akamai CDN — only cache URLs seen at least twice (avoids one-hit pollution)
  - Bitcoin SPV clients — filter transactions by address without revealing all
    watched addresses to the full node

How it works: bit array + multiple hash functions
-------------------------------------------------

The filter is a bit array of m bits, all initially 0.

  To ADD an element:    compute k bit positions; set those k bits to 1.
  To CHECK an element:  compute the same k positions; if ALL bits are 1,
                        return "probably yes"; if ANY bit is 0, return "no".

  Empty filter (m=16 bits, k=3 hash functions):
    index: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
    bits:  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0

  Add "alice" (h1→3, h2→7, h3→11):
    index: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
    bits:  0  0  0  1  0  0  0  1  0  0  0  1  0  0  0  0
                   ↑              ↑              ↑
             set by h1      set by h2      set by h3

  Add "bob" (h1→1, h2→5, h3→11):
    index: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
    bits:  0  1  0  1  0  1  0  1  0  0  0  1  0  0  0  0
               ↑          ↑                    ↑
          set by h1  set by h2       already 1 (alice & bob share bit 11)

  Check "carol" (h1→2, ...): bit 2 is 0 → "Definitely NO" ✓ correct
  Check "dave"  (h1→1, h2→5, h3→11): all 1 → "Probably YES" — FALSE POSITIVE!
    dave was never added, but all three of his bits were set by alice + bob.

Why deletion is impossible
--------------------------

Bit 11 was set by both "alice" and "bob". If we tried to "delete" bob by
clearing bits 1, 5, 11 — we'd clear bit 11, which alice needs. Deletion
is not supported in a standard Bloom filter. See the Counting Bloom Filter
extension for a solution (4-bit counters instead of bits).

The math: optimal parameters
-----------------------------

Given expected number of items n and desired false positive rate p:

  m = ceil(-n × ln(p) / ln(2)²)    ← optimal number of bits
  k = round((m / n) × ln(2))       ← optimal number of hash functions

Memory comparison for n = 1,000,000 elements:

  FPR    Bits/elem   Total bits    Memory
  -----  ---------   ----------    --------
  10%    4.79        4,792,536     585 KB
   1%    9.58        9,585,059     1.14 MB
  0.1%   14.38      14,377,588     1.72 MB
  vs. exact hash set: ~40 MB (35× larger!)

Double hashing: k hash functions from two
-----------------------------------------

Computing k truly independent hash functions is expensive. Instead, use the
"double hashing" trick: given two hash functions h1 and h2,

  g_i(x) = (h1(x) + i × h2(x)) mod m   for i = 0, 1, ..., k-1

This generates k distinct bit positions using only two hash computations.
In practice it performs equivalently to truly independent functions.

We use fnv1a_32 for h1 and djb2 for h2, both from the hash_functions package.

Bit array storage
-----------------

Bits are packed into a bytearray, 8 bits per byte:

  bit index i → byte index = i // 8
              → bit offset = i % 8

  Set bit i:    bytes[i // 8] |= (1 << (i % 8))
  Test bit i:   bytes[i // 8]  & (1 << (i % 8))  != 0

Example: m=16 stored in 2 bytes
  byte 0: bits  0–7   byte 1: bits 8–15
  Set bit 3:  byte 0 |= 0b00001000
  Set bit 11: byte 1 |= 0b00001000
"""

from __future__ import annotations

import math
from typing import Any

from hash_functions import djb2, fnv1a_32

_MASK32: int = 0xFFFFFFFF


def _fmix32(h: int) -> int:
    """MurmurHash3 32-bit finalizer — bijective bit mixer.

    Applies strict avalanche mixing so every output bit depends on every
    input bit. This breaks the correlation between fnv1a_32 and djb2 when
    both are used in the double-hashing scheme: without mixing, similar
    input strings produce correlated h1/h2 values, which causes probe
    sequences to cluster and pushes the real FPR above the theoretical
    value.

    The three constants are from Austin Appleby's MurmurHash3 (public domain).
    """
    h ^= h >> 16
    h = (h * 0x85EBCA6B) & _MASK32
    h ^= h >> 13
    h = (h * 0xC2B2AE35) & _MASK32
    h ^= h >> 16
    return h


class BloomFilter:
    """
    Space-efficient probabilistic set membership filter.

    Never has false negatives: if contains() returns False, the element is
    guaranteed to not be in the set.

    May have false positives: if contains() returns True, the element is
    probably in the set, but occasionally it was never added. The probability
    of this is controlled by expected_items and false_positive_rate.

    Usage:

        >>> bf = BloomFilter(expected_items=1000, false_positive_rate=0.01)
        >>> bf.add("hello")
        >>> "hello" in bf
        True
        >>> "world" in bf   # probably False (definitely not added)
        False
    """

    # ------------------------------------------------------------------
    # Construction
    # ------------------------------------------------------------------

    def __init__(
        self,
        expected_items: int = 1000,
        false_positive_rate: float = 0.01,
    ) -> None:
        """
        Create a Bloom filter optimised for the given parameters.

        Automatically computes optimal bit array size m and number of
        hash functions k using the standard formulas:

          m = ceil(-n * ln(p) / ln(2)^2)
          k = max(1, round((m / n) * ln(2)))

        expected_items: how many distinct elements you plan to add (n).
        false_positive_rate: target FP rate, e.g. 0.01 = 1%.
        """
        if expected_items <= 0:
            raise ValueError(f"expected_items must be a positive integer, got {expected_items!r}")
        if not (0.0 < false_positive_rate < 1.0):
            raise ValueError(
                f"false_positive_rate must be in the open interval (0, 1), got {false_positive_rate!r}"
            )

        n = expected_items
        p = false_positive_rate

        # Optimal bit count: derived by minimising false positive rate over m.
        m = math.ceil(-n * math.log(p) / (math.log(2) ** 2))

        # Optimal hash count: k = (m/n) * ln(2). At least 1.
        k = max(1, round((m / n) * math.log(2)))

        self._m: int = m          # total number of bits
        self._k: int = k          # number of hash functions
        self._n_expected: int = n # elements we sized for

        # Compact bit array: (m + 7) // 8 bytes covers all m bits.
        # All bits start at 0 (the "definitely not seen" state).
        self._bits: bytearray = bytearray((m + 7) // 8)

        # Track how many bits are currently 1 for fill_ratio and FP estimate.
        self._bits_set: int = 0

        # Count of elements added (for over-capacity detection).
        self._n: int = 0

    @classmethod
    def from_params(cls, bit_count: int, hash_count: int) -> BloomFilter:
        """
        Create a filter with explicit bit count and hash count.

        Bypasses the auto-sizing formula. Useful when you know the exact
        parameters you want (e.g., replicating a specific implementation,
        or tuning for a particular hardware cache line size).

        bit_count:  total number of bits m in the bit array.
        hash_count: number of hash functions k.
        """
        if bit_count <= 0:
            raise ValueError(f"bit_count must be a positive integer, got {bit_count!r}")
        if hash_count <= 0:
            raise ValueError(f"hash_count must be a positive integer, got {hash_count!r}")

        # Use __new__ to skip __init__ and set attributes directly.
        bf: BloomFilter = cls.__new__(cls)
        bf._m = bit_count
        bf._k = hash_count
        bf._n_expected = 0          # not sized for a specific n
        bf._bits = bytearray((bit_count + 7) // 8)
        bf._bits_set = 0
        bf._n = 0
        return bf

    # ------------------------------------------------------------------
    # Core operations
    # ------------------------------------------------------------------

    def _hash_indices(self, element: Any) -> list[int]:
        """
        Generate k bit indices for element using double hashing.

        Double hashing trick: given two independent hash functions h1 and h2,
        derive k hash functions as:

          g_i(x) = (h1(x) + i * h2(x)) mod m   for i = 0, 1, ..., k-1

        This gives k distinct, well-spread positions using only two hash
        computations. The approach is used by Google's Guava library, Redis,
        and most production Bloom filter implementations.

        h1 = fnv1a_32 (Fowler-Noll-Vo 1a, 32-bit) after fmix32 finalizer
        h2 = djb2     (Dan Bernstein's classic)     after fmix32 finalizer

        Both accept bytes; we UTF-8-encode the string representation of element.

        === Why the fmix32 finalizer? ===

        fnv1a_32 and djb2 process bytes in the same left-to-right order. For
        strings sharing a common prefix (e.g. "unknown_0"..."unknown_9999"),
        the two hash values are CORRELATED — they accumulate the same prefix
        state and only diverge in the final bytes. When h1 and h2 are
        correlated, probe positions cluster rather than spreading uniformly,
        pushing the real FPR well above the theoretical value.

        The MurmurHash3 fmix32 finalizer is a bijective (invertible) mixing
        function: every output bit depends on every input bit. Applying it
        independently to h1 and h2 breaks their correlation without changing
        the probability space (it is a permutation of [0, 2^32)).

        For djb2, which returns an unbounded integer, we fold the high 32 bits
        into the low 32 bits (XOR) before applying fmix32, so we use all the
        entropy without overflow.

        h2 is forced odd (| 1) to guarantee gcd(h2, m) can be at most
        gcd(odd, m). If m is odd (typical for non-power-of-2 bit arrays), an
        odd step covers all m positions before repeating — maximising spread.
        """
        raw: bytes = str(element).encode("utf-8")
        h1: int = _fmix32(fnv1a_32(raw))
        # djb2 returns an unbounded integer: fold high bits in before mixing.
        h2_raw: int = djb2(raw)
        h2: int = _fmix32((h2_raw ^ (h2_raw >> 32)) & _MASK32)
        h2 |= 1  # ensure non-zero and odd → all m positions reachable
        return [(h1 + i * h2) % self._m for i in range(self._k)]

    def add(self, element: Any) -> None:
        """
        Add element to the filter. Sets up to k bits in the bit array.

        If some bits were already 1 (set by previous elements), _bits_set
        only increments for newly set bits — a bit is counted once.

        O(k) hash computations and O(k) bit-set operations.
        """
        for idx in self._hash_indices(element):
            byte_idx: int = idx // 8
            bit_mask: int = 1 << (idx % 8)
            # Only count the bit if it wasn't already set.
            if not (self._bits[byte_idx] & bit_mask):
                self._bits[byte_idx] |= bit_mask
                self._bits_set += 1
        self._n += 1

    def contains(self, element: Any) -> bool:
        """
        Check if element might be in the filter.

        Returns False → DEFINITELY not in filter. Zero false negatives.
                        If we never added the element, at least one of its
                        k bit positions will be 0.

        Returns True  → PROBABLY in filter. False positive rate is bounded
                        by the parameters supplied at construction time.

        O(k) operations — fast even for very large bit arrays.
        """
        for idx in self._hash_indices(element):
            byte_idx: int = idx // 8
            bit_mask: int = 1 << (idx % 8)
            if not (self._bits[byte_idx] & bit_mask):
                return False   # at least one bit is 0 → definitely absent
        return True            # all k bits are 1 → probably present

    def __contains__(self, element: object) -> bool:
        """
        Alias for contains(), enabling the natural `element in bf` syntax.

        Example:
            if "alice" in bloom_filter:
                print("probably seen before")
        """
        return self.contains(element)

    # ------------------------------------------------------------------
    # Properties and statistics
    # ------------------------------------------------------------------

    @property
    def bit_count(self) -> int:
        """Total number of bits in the filter (m). Fixed at construction."""
        return self._m

    @property
    def hash_count(self) -> int:
        """Number of hash functions used (k). Fixed at construction."""
        return self._k

    @property
    def bits_set(self) -> int:
        """Number of bits currently set to 1."""
        return self._bits_set

    @property
    def fill_ratio(self) -> float:
        """
        Fraction of bits currently set to 1: bits_set / bit_count.

        Starts at 0.0 for an empty filter.
        Approaches 1.0 as the filter fills up.
        At fill_ratio ≈ 0.5 (half the bits set), the filter is near its
        optimal operating point.

        Note: this is the *actual* ratio, counting set bits directly.
        The theoretical expected fill ratio is 1 - e^(-k*n/m).
        """
        return self._bits_set / self._m

    @property
    def estimated_false_positive_rate(self) -> float:
        """
        Estimated current false positive rate based on fill_ratio.

        Formula: (fill_ratio)^k

        This is an approximation. The theoretically more accurate formula
        uses kn/m, but fill_ratio gives the same value once the filter is
        used as intended (k bits set per element, no duplicate adds).

        When the filter is empty, fill_ratio = 0 so this returns 0.0.
        When the filter is at capacity, this approaches the target FPR.
        When over capacity, this rises toward 1.0.
        """
        if self._bits_set == 0:
            return 0.0
        return self.fill_ratio ** self._k

    def is_over_capacity(self) -> bool:
        """
        True if more elements than expected_items have been added.

        When over capacity, the actual false positive rate rises above the
        target rate specified at construction. The filter still works correctly
        (no false negatives), but false positives become more frequent.

        Returns False if the filter was created via from_params() (no capacity
        was specified).
        """
        if self._n_expected == 0:
            return False
        return self._n > self._n_expected

    def size_bytes(self) -> int:
        """Memory usage of the bit array in bytes: ceil(m / 8)."""
        return len(self._bits)

    # ------------------------------------------------------------------
    # Static utility methods
    # ------------------------------------------------------------------

    @staticmethod
    def optimal_m(n: int, p: float) -> int:
        """
        Optimal bit array size for n elements and false positive rate p.

        Formula: m = ceil(-n * ln(p) / ln(2)^2)

        Examples:
          optimal_m(1_000_000, 0.01)  → 9,585,059   (~1.14 MB)
          optimal_m(1_000_000, 0.001) → 14,377,588   (~1.72 MB)
        """
        return math.ceil(-n * math.log(p) / (math.log(2) ** 2))

    @staticmethod
    def optimal_k(m: int, n: int) -> int:
        """
        Optimal number of hash functions for m bits and n elements.

        Formula: k = max(1, round((m / n) * ln(2)))

        Intuition: k_optimal ≈ 0.693 * (m/n). At this k, each bit has about
        a 50% chance of being set, which minimises the false positive rate.

        Examples:
          optimal_k(9_585_059, 1_000_000) → 7
          optimal_k(14_377_588, 1_000_000) → 10
        """
        return max(1, round((m / n) * math.log(2)))

    @staticmethod
    def capacity_for_memory(memory_bytes: int, p: float) -> int:
        """
        How many elements can be stored in memory_bytes at false positive rate p?

        This is the inverse of optimal_m:
          m = -n * ln(p) / ln(2)^2
          n = -m * ln(2)^2 / ln(p)

        Example:
          capacity_for_memory(1_000_000, 0.01) → ~877,000 elements
          (about 1 million elements in 1 MB at 1% FPR)
        """
        m = memory_bytes * 8   # bytes to bits
        return int(-m * (math.log(2) ** 2) / math.log(p))

    # ------------------------------------------------------------------
    # String representation
    # ------------------------------------------------------------------

    def __repr__(self) -> str:
        """
        Human-readable summary of the filter's current state.

        Example:
          BloomFilter(m=9585059, k=7, bits_set=6873/9585059 (0.07%), ~fp=0.00%)
        """
        pct_set = self.fill_ratio * 100
        est_fp = self.estimated_false_positive_rate * 100
        return (
            f"BloomFilter("
            f"m={self._m}, "
            f"k={self._k}, "
            f"bits_set={self._bits_set}/{self._m} ({pct_set:.2f}%), "
            f"~fp={est_fp:.4f}%)"
        )
