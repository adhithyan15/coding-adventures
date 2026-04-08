"""
HyperLogLog — probabilistic cardinality estimator.

HyperLogLog answers "how many distinct elements have I seen?" using a tiny,
fixed amount of memory — typically 12 KB — regardless of stream size.

This is an *approximate* answer. With 12 KB (the Redis default, precision=14),
the standard error is ±0.81%. For most real-world use cases — counting unique
visitors, unique queries, unique IP addresses — this is more than sufficient.

Memory comparison:
  Hash Set:       [  "alice", "bob", "carol", ..., 1M entries  ]  ~32 MB
  HyperLogLog:    [  16,384 registers × 6 bits each  ]            ~12 KB
                  Estimate: 1,000,000 ± 0.81%                    660,000× smaller

Redis implements HyperLogLog with PFADD and PFCOUNT. ("PF" honours Philippe
Flajolet, the mathematician who invented the algorithm in 2007.)

== The Core Intuition ==

Hash each element to a 64-bit integer. Because we use a quality hash function,
each bit is equally likely to be 0 or 1. The probability of k leading zeros is
1/2^(k+1). If we've seen n distinct elements, the maximum leading-zero count
across all their hashes will be roughly log₂(n). So 2^max_zeros estimates n.

Problem: this estimator has very high variance. The fix: use multiple registers.

== The HyperLogLog Algorithm ==

Split the 64-bit hash into two parts:
  - First b bits  → bucket (register) index  j ∈ [0, m)  where m = 2^b
  - Remaining bits → count leading zeros + 1 → ρ

Each register M[j] stores the maximum ρ seen for that bucket. After processing
all elements, combine the registers using the harmonic mean (which is robust
to outliers) and multiply by a bias-correction constant α:

  Z    = Σ 2^(-M[j])  for j in 0..m-1
  E    = α × m² / Z

Two range corrections are applied:
  - Small n (< 2.5m):  use LinearCounting (counts empty registers)
  - Large n (> 2^32/30): apply logarithmic correction (rare in practice)

Diagram of hash splitting (b=14, m=16384):

  64-bit hash:
  ┌────────────────┬─────────────────────────────────────────────────────┐
  │  bits 63..50   │                   bits 49..0                        │
  │  (top 14 bits) │               (bottom 50 bits)                      │
  │  register idx j│              count leading zeros → ρ                │
  └────────────────┴─────────────────────────────────────────────────────┘

Error rate vs memory:
  b   m=2^b    Memory    Standard error
  4   16       96 bits   26.0%
  8   256      1.5 KB    6.5%
  10  1,024    6 KB      3.25%
  14  16,384   ~12 KB    0.81%   ← Redis default
  16  65,536   ~393 KB   0.41%
"""

from __future__ import annotations

import math
from typing import Any

from hash_functions import fnv1a_64


# ---------------------------------------------------------------------------
# Helper: count leading zeros
# ---------------------------------------------------------------------------

def _count_leading_zeros(value: int, bit_width: int) -> int:
    """
    Count the number of leading zero bits in `value` within `bit_width` bits.

    Because Python integers have arbitrary precision, we must cap the
    interpretation at exactly `bit_width` bits.

    Examples:
      _count_leading_zeros(0b0010, 4)  → 2   (binary: 0010)
      _count_leading_zeros(0,       8) → 8   (all zeros)
      _count_leading_zeros(0b1000, 4)  → 0   (leading bit is 1)

    Efficient implementation using Python's built-in bit_length():
      bit_length() returns the number of bits needed to represent value
      (0 for value==0). The leading zero count is therefore:
        bit_width - value.bit_length()

    Bit length examples:
      (0b00101).bit_length() == 3   ← needs 3 bits
      leading zeros in 5 bits: 5 - 3 = 2 ✓

    Args:
        value:     Non-negative integer whose leading zeros we count.
        bit_width: Total number of bits to consider.

    Returns:
        Number of leading zero bits (0 to bit_width inclusive).
    """
    if value == 0:
        return bit_width
    return bit_width - value.bit_length()


# ---------------------------------------------------------------------------
# Helper: bias-correction constant alpha_m
# ---------------------------------------------------------------------------

def _alpha(m: int) -> float:
    """
    Bias-correction constant α for the harmonic mean estimator.

    The raw harmonic mean estimator overestimates the true cardinality by
    a factor of 1/α. These constants were derived analytically by Philippe
    Flajolet using complex analysis (generating functions).

    For m ≥ 128, the formula 0.7213 / (1 + 1.079/m) is used. This formula
    converges to 0.7213 as m → ∞ (the constant for the continuous limit).

    m     α
    ----  -----
    16    0.673
    32    0.697
    64    0.709
    128+  0.7213 / (1 + 1.079/m)  ≈ 0.7213

    Args:
        m: Number of registers (must be a power of 2 ≥ 16).

    Returns:
        Correction factor α in approximately [0.673, 0.7213].
    """
    if m == 16:
        return 0.673
    if m == 32:
        return 0.697
    if m == 64:
        return 0.709
    # General formula for m >= 128; also used for any m not exactly 16/32/64
    return 0.7213 / (1.0 + 1.079 / m)


# ---------------------------------------------------------------------------
# Public class: HyperLogLog
# ---------------------------------------------------------------------------

class HyperLogLog:
    """
    Probabilistic cardinality estimator. O(1) memory regardless of stream size.

    Uses the HyperLogLog algorithm (Flajolet, Fusy, Gandouet, Meunier — 2007)
    to estimate the number of distinct elements added to the sketch.

    Standard error ≈ 1.04 / sqrt(2^precision)
    Memory usage   = 2^precision bytes (one byte per register, naive packing)

    Quick start:

        >>> hll = HyperLogLog(precision=14)
        >>> for user_id in user_stream:
        ...     hll.add(user_id)
        >>> print(f"~{hll.count():,} unique users")

    Redis analogue:
        PFADD   → hll.add(element)
        PFCOUNT → hll.count()
        PFMERGE → hll1.merge(hll2)

    Merging two sketches gives the union cardinality (distinct elements in
    either set) at the same error rate — no re-processing required:

        jan_hll.merge(feb_hll).count()  # unique users in Jan OR Feb
    """

    # Precision must be in [4, 16] inclusive.
    # Below 4: only 16 registers → 26% error, practically useless.
    # Above 16: 65536 registers → fine, but most applications don't need it.
    _MIN_PRECISION: int = 4
    _MAX_PRECISION: int = 16

    def __init__(self, precision: int = 14) -> None:
        """
        Create an empty HyperLogLog sketch.

        Args:
            precision: Number of bits used for the register index.
                       m = 2^precision registers are allocated.

                       precision=14 → 16,384 registers → ~12 KB → ±0.81% error
                       precision=10 → 1,024  registers → ~768 B  → ±3.25% error

                       Valid range: 4 (±26%) to 16 (±0.41%).

        Raises:
            ValueError: If precision is outside [4, 16].
        """
        if not (self._MIN_PRECISION <= precision <= self._MAX_PRECISION):
            raise ValueError(
                f"precision must be between {self._MIN_PRECISION} and "
                f"{self._MAX_PRECISION}, got {precision}"
            )
        self._precision: int = precision
        self._num_registers: int = 1 << precision   # 2^precision
        # Each register stores the maximum ρ seen for its bucket.
        # ρ is in [1, 64-b], so values fit in a single byte.
        # Initially all zeros — no elements added yet.
        self._registers: list[int] = [0] * self._num_registers

    # ------------------------------------------------------------------
    # Core mutating operation: add
    # ------------------------------------------------------------------

    def add(self, element: Any) -> None:
        """
        Add an element to the HyperLogLog sketch.

        The element is serialised to UTF-8 bytes and hashed with FNV-1a 64-bit.
        Any Python object with a meaningful str() representation works:
        strings, integers, floats, tuples, etc.

        Algorithm:
          1. Hash element → 64-bit integer h
          2. Top b bits of h → register index j
          3. Remaining 64-b bits → count leading zeros → ρ = clz + 1
          4. registers[j] = max(registers[j], ρ)

        Diagram for b=14:
          h (64 bits):
          ┌──────────────┬──────────────────────────────────────────────┐
          │  bits 63..50 │                bits 49..0                    │
          │  → j (index) │  → ρ = count_leading_zeros(this part) + 1   │
          └──────────────┴──────────────────────────────────────────────┘

        Why add 1 to the leading zero count?
          ρ must be at least 1 (never 0) so that a register value of 0
          can be used as a sentinel for "this register has never been updated".
          Without +1, an element whose remaining bits are all-ones would
          produce ρ=0, indistinguishable from an empty register.

        Args:
            element: Any value. Converted to str, then UTF-8 encoded.

        Time complexity: O(1)
        """
        # Serialise and hash.
        raw: bytes = str(element).encode("utf-8")
        h: int = fnv1a_64(raw)

        b: int = self._precision
        # Top b bits select the register.
        #   h >> (64 - b) keeps only the most significant b bits.
        bucket: int = h >> (64 - b)

        # Remaining 64-b bits are used to estimate the max run of leading zeros.
        #   Mask: (1 << (64-b)) - 1 is a bitmask of 64-b ones.
        remaining_bits: int = 64 - b
        remaining: int = h & ((1 << remaining_bits) - 1)

        # ρ = position of leftmost 1-bit (1-indexed), i.e. leading zeros + 1.
        rho: int = _count_leading_zeros(remaining, remaining_bits) + 1

        # Update the register only if we saw more leading zeros than before.
        if rho > self._registers[bucket]:
            self._registers[bucket] = rho

    # ------------------------------------------------------------------
    # Core query: count
    # ------------------------------------------------------------------

    def count(self) -> int:
        """
        Estimate the number of distinct elements added to the sketch.

        Algorithm (three phases):

        Phase 1 — Raw harmonic mean estimate:
          Z    = Σ 2^(-M[j])  for each register j
          E    = α × m² / Z

          The harmonic mean de-emphasises outlier registers (a single
          register with a very high value doesn't dominate the estimate).

          Why harmonic mean?
            Arithmetic mean of [1, 2, 3, 100] = 26.5   ← dominated by 100
            Harmonic mean of  [1, 2, 3, 100] = 2.14    ← resistant to outlier

        Phase 2 — Small range correction (LinearCounting):
          When E ≤ 2.5m, many registers are still 0. In this regime, the
          harmonic mean estimator is inaccurate. Instead we use LinearCounting:
            E = m × ln(m / V)   where V = number of empty registers

          This is the "balls into bins" approximation: if n balls are thrown
          uniformly into m bins, the expected number of empty bins is
            V = m × e^(-n/m)
          Solving for n: n = m × ln(m / V).

        Phase 3 — Large range correction:
          When E > 2^32 / 30, hash collisions cause systematic underestimation.
          Apply: E = -2^32 × ln(1 - E/2^32)
          In practice, this threshold (≈ 143 million) is rarely exceeded for
          64-bit hashes; included for spec compliance.

        Returns:
            Rounded integer estimate of the number of distinct elements.

        Time complexity: O(m) where m = 2^precision
        """
        m: int = self._num_registers

        # Phase 1: harmonic mean estimator.
        #   Z = Σ 2^(-M[j]) is the denominator of the harmonic mean.
        #   We avoid 2.0 ** (-r) style for clarity; same result.
        z_sum: float = sum(2.0 ** (-r) for r in self._registers)
        alpha: float = _alpha(m)
        estimate: float = alpha * m * m / z_sum

        # Phase 2: small range correction via LinearCounting.
        if estimate <= 2.5 * m:
            zeros: int = self._registers.count(0)
            if zeros > 0:
                # ln(m / V) — natural log of the ratio of registers to empties.
                estimate = m * math.log(m / zeros)

        # Phase 3: large range correction.
        two_32: float = 2.0 ** 32
        if estimate > two_32 / 30.0:
            estimate = -two_32 * math.log(1.0 - estimate / two_32)

        return round(estimate)

    # ------------------------------------------------------------------
    # Merge: union of two sketches
    # ------------------------------------------------------------------

    def merge(self, other: "HyperLogLog") -> "HyperLogLog":
        """
        Return a new HyperLogLog representing the union of self and other.

        The union contains all distinct elements from either sketch.
        The merge is O(m) and requires no access to the original elements.

        Because each register M[j] stores the *maximum* ρ ever seen for
        bucket j, the union is simply the element-wise maximum:

          result.M[j] = max(self.M[j], other.M[j])  for all j

        This works because:
          - If element x maps to bucket j in self but not other: self.M[j]
            already captures x; taking the max preserves it.
          - If x appears in both sketches: both registers reflect x's ρ; the
            max is correct.
          - If x only appears in other: other.M[j] captures it.

        Visualisation:
          Jan:    [3, 1, 5, 2, 4, ...]
          Feb:    [2, 4, 3, 1, 6, ...]
          Merged: [3, 4, 5, 2, 6, ...]   ← element-wise max

        Note: There is NO intersection operation for HyperLogLog — only union.
        Intersection via inclusion-exclusion amplifies the error badly.

        Args:
            other: Another HyperLogLog sketch with the same precision.

        Returns:
            New HyperLogLog sketch representing self ∪ other.

        Raises:
            ValueError: If self and other have different precision values.

        Time complexity: O(m)
        """
        if self._precision != other._precision:
            raise ValueError(
                f"Cannot merge HyperLogLog sketches with different precisions: "
                f"{self._precision} vs {other._precision}"
            )
        result = HyperLogLog(precision=self._precision)
        result._registers = [
            max(a, b) for a, b in zip(self._registers, other._registers)
        ]
        return result

    # ------------------------------------------------------------------
    # Dunder methods
    # ------------------------------------------------------------------

    def __len__(self) -> int:
        """
        Return the estimated distinct count. Same as count().

        Allows: len(hll) as a shorthand for hll.count().
        """
        return self.count()

    def __repr__(self) -> str:
        """
        Human-readable representation showing key parameters.

        Example:
            HyperLogLog(precision=14, registers=16384, error_rate=0.81%)
        """
        er_pct: float = self.error_rate * 100.0
        return (
            f"HyperLogLog(precision={self._precision}, "
            f"registers={self._num_registers}, "
            f"error_rate={er_pct:.2f}%)"
        )

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def precision(self) -> int:
        """Number of bits used for the register index (b)."""
        return self._precision

    @property
    def num_registers(self) -> int:
        """
        Number of registers = 2^precision.

        Each register holds an integer in [0, 64-precision], stored as one byte.
        Total naive memory: num_registers bytes.
        Packed representation (6 bits/register): num_registers * 6 / 8 bytes.
        """
        return self._num_registers

    @property
    def error_rate(self) -> float:
        """
        Expected relative standard error = 1.04 / sqrt(num_registers).

        This is the theoretical standard error of the HyperLogLog estimator,
        derived by Flajolet et al. It means that with ~68% probability the
        estimate is within ±error_rate of the true count, and with ~95%
        probability it is within ±2×error_rate.

        Examples:
          precision=14: 1.04 / sqrt(16384) ≈ 0.0081  (0.81%)
          precision=10: 1.04 / sqrt(1024)  ≈ 0.0325  (3.25%)
        """
        return 1.04 / math.sqrt(self._num_registers)

    # ------------------------------------------------------------------
    # Static utility methods
    # ------------------------------------------------------------------

    @staticmethod
    def error_rate_for_precision(precision: int) -> float:
        """
        Standard error rate for a given precision value.

        Args:
            precision: The b parameter (number of bits for register index).

        Returns:
            Relative error as a fraction (e.g., 0.0081 for 0.81%).
        """
        m: int = 1 << precision
        return 1.04 / math.sqrt(m)

    @staticmethod
    def memory_bytes(precision: int) -> int:
        """
        Memory usage in bytes for a given precision (packed representation).

        Packed: 6 bits per register (values 0–63 fit in 6 bits).
        Total bits = 2^precision × 6.
        Total bytes = 2^precision × 6 / 8 = 2^precision × 3 / 4.

        Examples:
          memory_bytes(14) = 16384 * 6 // 8 = 12288 bytes ≈ 12 KB
          memory_bytes(10) = 1024  * 6 // 8 =   768 bytes

        Args:
            precision: Number of bits for register index.

        Returns:
            Number of bytes needed for the packed register array.
        """
        m: int = 1 << precision
        return (m * 6) // 8

    @staticmethod
    def optimal_precision(desired_error: float) -> int:
        """
        Smallest precision value that achieves the desired relative error rate.

        Solves: 1.04 / sqrt(2^b) ≤ desired_error
                2^b ≥ (1.04 / desired_error)^2
                b ≥ log2((1.04 / desired_error)^2)

        Args:
            desired_error: Maximum acceptable relative error as a fraction
                           (e.g., 0.01 for 1%, 0.0081 for 0.81%).

        Returns:
            Minimum precision (b) that meets the desired error budget.
            Clamped to [4, 16].

        Examples:
            optimal_precision(0.01)  → 14  (0.81% < 1.00%)
            optimal_precision(0.05)  → 10  (3.25% < 5.00%)
        """
        # 1.04 / sqrt(m) <= desired_error  ⟹  m >= (1.04/desired_error)^2
        min_m: float = (1.04 / desired_error) ** 2
        # Smallest b such that 2^b >= min_m
        b: int = math.ceil(math.log2(min_m))
        return max(4, min(16, b))
