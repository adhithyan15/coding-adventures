"""
Tests for the HyperLogLog cardinality estimator.

Test strategy mirrors the DT21 spec:
  1. Basic accuracy at multiple cardinalities
  2. Duplicate suppression
  3. Merge (union) correctness for disjoint and overlapping sets
  4. Property accessors (num_registers, error_rate, precision, len)
  5. Input validation (precision out of range, merge precision mismatch)
  6. Static utility methods (error_rate_for_precision, memory_bytes, optimal_precision)
  7. Internal helper: _count_leading_zeros
  8. Edge cases: empty sketch, single element, very small cardinality

Coverage target: ≥95% of all src/hyperloglog lines.
"""

from __future__ import annotations

import math

import pytest

from hyperloglog import HyperLogLog
from hyperloglog.hyperloglog import _count_leading_zeros, _alpha


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------

def _within(estimate: int, true_count: int, fraction: float) -> bool:
    """Return True if |estimate - true_count| / true_count <= fraction."""
    return abs(estimate - true_count) / true_count <= fraction


# ---------------------------------------------------------------------------
# 1. Basic accuracy
# ---------------------------------------------------------------------------

class TestBasicAccuracy:
    """Accuracy tests at various cardinalities using precision=14 (Redis default)."""

    def test_empty_count_zero(self) -> None:
        """An empty sketch must report 0."""
        hll = HyperLogLog()
        assert hll.count() == 0

    def test_single_element(self) -> None:
        """A sketch with one distinct element must report approximately 1."""
        hll = HyperLogLog()
        hll.add("hello")
        # LinearCounting handles very small cardinalities; result should be 1
        assert hll.count() == 1

    def test_two_distinct_elements(self) -> None:
        """Two distinct elements should give an estimate close to 2."""
        hll = HyperLogLog(precision=14)
        hll.add("foo")
        hll.add("bar")
        est = hll.count()
        assert 1 <= est <= 5

    def test_estimate_accuracy_small(self) -> None:
        """1,000 distinct elements: estimate within 10% of true count."""
        hll = HyperLogLog(precision=14)
        for i in range(1000):
            hll.add(f"element_{i}")
        est = hll.count()
        assert 900 <= est <= 1100

    def test_estimate_accuracy_medium(self) -> None:
        """10,000 distinct elements: estimate within 5% of true count."""
        hll = HyperLogLog(precision=14)
        for i in range(10_000):
            hll.add(f"user_{i}")
        est = hll.count()
        assert 9_500 <= est <= 10_500

    def test_estimate_accuracy_large(self) -> None:
        """100,000 distinct elements: estimate within 5% of true count."""
        hll = HyperLogLog(precision=14)
        for i in range(100_000):
            hll.add(i)
        est = hll.count()
        assert 95_000 <= est <= 105_000

    def test_various_element_types(self) -> None:
        """add() should accept integers, floats, strings, and tuples."""
        hll = HyperLogLog(precision=10)
        hll.add(42)
        hll.add(3.14)
        hll.add("hello")
        hll.add((1, 2, 3))
        est = hll.count()
        # 4 distinct elements; with precision=10 we expect a rough estimate
        assert 1 <= est <= 20


# ---------------------------------------------------------------------------
# 2. Duplicate suppression
# ---------------------------------------------------------------------------

class TestDuplicateSuppression:
    """Identical elements added multiple times should not inflate the estimate."""

    def test_duplicate_elements_not_counted(self) -> None:
        """Adding the same string 1,000 times yields count ≈ 1."""
        hll = HyperLogLog()
        for _ in range(1_000):
            hll.add("same")
        # Should be close to 1; definitely not 1000
        assert hll.count() < 10

    def test_same_integer_repeated(self) -> None:
        """Adding the same integer 10,000 times yields count ≈ 1."""
        hll = HyperLogLog(precision=14)
        for _ in range(10_000):
            hll.add("same_element")
        assert hll.count() == 1

    def test_mixed_duplicates_and_uniques(self) -> None:
        """500 unique + 500 duplicates of one element ≈ 501 distinct."""
        hll = HyperLogLog(precision=14)
        for i in range(500):
            hll.add(f"unique_{i}")
        for _ in range(500):
            hll.add("repeated")
        est = hll.count()
        # True count is 501; allow 10% error
        assert 450 <= est <= 560


# ---------------------------------------------------------------------------
# 3. Merge (union)
# ---------------------------------------------------------------------------

class TestMerge:
    """Tests for merge(), which computes the set union of two HLL sketches."""

    def test_merge_disjoint_sets(self) -> None:
        """Union of two disjoint 1,000-element sets ≈ 2,000."""
        hll1 = HyperLogLog(precision=14)
        hll2 = HyperLogLog(precision=14)
        for i in range(1_000):
            hll1.add(f"a_{i}")
        for i in range(1_000):
            hll2.add(f"b_{i}")
        merged = hll1.merge(hll2)
        est = merged.count()
        assert 1_800 <= est <= 2_200

    def test_merge_overlapping_sets(self) -> None:
        """Union of two identical 1,000-element sets ≈ 1,000 (not 2,000)."""
        hll1 = HyperLogLog(precision=14)
        hll2 = HyperLogLog(precision=14)
        for i in range(1_000):
            hll1.add(i)
            hll2.add(i)  # same elements
        merged = hll1.merge(hll2)
        est = merged.count()
        # Should be ~1,000, not ~2,000
        assert 800 <= est <= 1_200

    def test_merge_with_empty(self) -> None:
        """Merging a populated HLL with an empty one yields the same estimate."""
        hll1 = HyperLogLog(precision=14)
        for i in range(500):
            hll1.add(f"x_{i}")
        hll_empty = HyperLogLog(precision=14)
        merged = hll1.merge(hll_empty)
        assert merged.count() == hll1.count()

    def test_merge_both_empty(self) -> None:
        """Merging two empty sketches yields count 0."""
        hll1 = HyperLogLog(precision=14)
        hll2 = HyperLogLog(precision=14)
        merged = hll1.merge(hll2)
        assert merged.count() == 0

    def test_merge_different_precision_raises(self) -> None:
        """Merging sketches with different precisions must raise ValueError."""
        hll1 = HyperLogLog(precision=10)
        hll2 = HyperLogLog(precision=14)
        with pytest.raises(ValueError, match="precision"):
            hll1.merge(hll2)

    def test_merge_does_not_mutate_originals(self) -> None:
        """merge() must return a NEW sketch; originals must be unchanged."""
        hll1 = HyperLogLog(precision=10)
        hll2 = HyperLogLog(precision=10)
        hll1.add("alpha")
        hll2.add("beta")
        orig_regs1 = list(hll1._registers)
        orig_regs2 = list(hll2._registers)
        _merged = hll1.merge(hll2)
        assert hll1._registers == orig_regs1
        assert hll2._registers == orig_regs2

    def test_merge_precision_preserved(self) -> None:
        """The merged sketch must carry the same precision as the originals."""
        hll1 = HyperLogLog(precision=10)
        hll2 = HyperLogLog(precision=10)
        merged = hll1.merge(hll2)
        assert merged.precision == 10
        assert merged.num_registers == 1024


# ---------------------------------------------------------------------------
# 4. Properties and dunder methods
# ---------------------------------------------------------------------------

class TestProperties:
    """Tests for properties: precision, num_registers, error_rate, len."""

    def test_num_registers_precision_10(self) -> None:
        hll = HyperLogLog(precision=10)
        assert hll.num_registers == 1024

    def test_num_registers_precision_14(self) -> None:
        hll = HyperLogLog(precision=14)
        assert hll.num_registers == 16_384

    def test_num_registers_precision_4(self) -> None:
        hll = HyperLogLog(precision=4)
        assert hll.num_registers == 16

    def test_precision_property(self) -> None:
        for b in [4, 8, 10, 12, 14, 16]:
            hll = HyperLogLog(precision=b)
            assert hll.precision == b

    def test_error_rate_precision_14(self) -> None:
        hll = HyperLogLog(precision=14)
        # 1.04 / sqrt(16384) ≈ 0.00812
        assert abs(hll.error_rate - 0.00812) < 0.001

    def test_error_rate_precision_10(self) -> None:
        hll = HyperLogLog(precision=10)
        # 1.04 / sqrt(1024) ≈ 0.0325
        assert abs(hll.error_rate - 0.0325) < 0.002

    def test_len_equals_count(self) -> None:
        """__len__ must return the same value as count()."""
        hll = HyperLogLog()
        for i in range(100):
            hll.add(i)
        assert len(hll) == hll.count()

    def test_len_empty(self) -> None:
        hll = HyperLogLog()
        assert len(hll) == 0

    def test_repr_contains_class_name_and_precision(self) -> None:
        hll = HyperLogLog(precision=14)
        r = repr(hll)
        assert "HyperLogLog" in r
        assert "14" in r

    def test_repr_contains_error_rate(self) -> None:
        hll = HyperLogLog(precision=14)
        r = repr(hll)
        # Error rate should appear in the repr
        assert "%" in r


# ---------------------------------------------------------------------------
# 5. Input validation
# ---------------------------------------------------------------------------

class TestValidation:
    """Tests for constructor validation and merge precision check."""

    def test_precision_too_low_raises(self) -> None:
        with pytest.raises(ValueError):
            HyperLogLog(precision=3)

    def test_precision_too_high_raises(self) -> None:
        with pytest.raises(ValueError):
            HyperLogLog(precision=17)

    def test_precision_at_minimum(self) -> None:
        """precision=4 is the lowest valid value; must not raise."""
        hll = HyperLogLog(precision=4)
        assert hll.num_registers == 16

    def test_precision_at_maximum(self) -> None:
        """precision=16 is the highest valid value; must not raise."""
        hll = HyperLogLog(precision=16)
        assert hll.num_registers == 65_536

    def test_default_precision_is_14(self) -> None:
        hll = HyperLogLog()
        assert hll.precision == 14


# ---------------------------------------------------------------------------
# 6. Static utility methods
# ---------------------------------------------------------------------------

class TestStaticMethods:
    """Tests for error_rate_for_precision, memory_bytes, optimal_precision."""

    def test_error_rate_for_precision_14(self) -> None:
        er = HyperLogLog.error_rate_for_precision(14)
        assert abs(er - 0.00812) < 0.0001

    def test_error_rate_for_precision_10(self) -> None:
        er = HyperLogLog.error_rate_for_precision(10)
        assert abs(er - 0.0325) < 0.001

    def test_memory_bytes_14(self) -> None:
        # 16384 * 6 / 8 = 12288
        assert HyperLogLog.memory_bytes(14) == 12_288

    def test_memory_bytes_10(self) -> None:
        # 1024 * 6 / 8 = 768
        assert HyperLogLog.memory_bytes(10) == 768

    def test_memory_bytes_4(self) -> None:
        # 16 * 6 / 8 = 12
        assert HyperLogLog.memory_bytes(4) == 12

    def test_optimal_precision_1pct_error(self) -> None:
        """For 1% desired error, precision=14 achieves 0.81% < 1%."""
        p = HyperLogLog.optimal_precision(0.01)
        assert p == 14

    def test_optimal_precision_5pct_error(self) -> None:
        """For 5% desired error, precision=9 achieves 4.6% < 5%.

        Calculation: 1.04 / sqrt(2^9) = 1.04 / sqrt(512) ≈ 0.046 = 4.6% < 5%.
        Precision=9 (not 10) is the minimum that satisfies the 5% budget.
        """
        p = HyperLogLog.optimal_precision(0.05)
        assert p == 9

    def test_optimal_precision_result_satisfies_requirement(self) -> None:
        """optimal_precision must always return a precision that actually meets the goal."""
        for desired_error in [0.30, 0.10, 0.05, 0.03, 0.01]:
            p = HyperLogLog.optimal_precision(desired_error)
            achieved = HyperLogLog.error_rate_for_precision(p)
            assert achieved <= desired_error + 1e-9, (
                f"desired={desired_error}, got precision={p}, achieved={achieved}"
            )

    def test_optimal_precision_clamp_low(self) -> None:
        """Very large desired error is clamped to minimum precision 4."""
        p = HyperLogLog.optimal_precision(0.99)
        assert p == 4

    def test_optimal_precision_clamp_high(self) -> None:
        """Very small desired error is clamped to maximum precision 16."""
        p = HyperLogLog.optimal_precision(0.001)
        assert p == 16


# ---------------------------------------------------------------------------
# 7. Internal helper: _count_leading_zeros
# ---------------------------------------------------------------------------

class TestCountLeadingZeros:
    """Unit tests for the internal _count_leading_zeros function."""

    def test_all_zeros(self) -> None:
        """value=0 means all bits are zero."""
        assert _count_leading_zeros(0, 8) == 8
        assert _count_leading_zeros(0, 50) == 50
        assert _count_leading_zeros(0, 1) == 1

    def test_leading_one(self) -> None:
        """value with MSB set → 0 leading zeros."""
        assert _count_leading_zeros(0b10000000, 8) == 0
        assert _count_leading_zeros(1 << 49, 50) == 0

    def test_one_leading_zero(self) -> None:
        assert _count_leading_zeros(0b01000000, 8) == 1

    def test_two_leading_zeros(self) -> None:
        assert _count_leading_zeros(0b00100000, 8) == 2

    def test_seven_leading_zeros(self) -> None:
        assert _count_leading_zeros(0b00000001, 8) == 7

    def test_single_bit_width(self) -> None:
        assert _count_leading_zeros(0, 1) == 1
        assert _count_leading_zeros(1, 1) == 0

    def test_large_value(self) -> None:
        # 50-bit space, value = 2^48 (bit 48 is set) → 1 leading zero
        val = 1 << 48
        assert _count_leading_zeros(val, 50) == 1


# ---------------------------------------------------------------------------
# 8. Internal helper: _alpha
# ---------------------------------------------------------------------------

class TestAlpha:
    """Unit tests for the bias-correction constant function."""

    def test_alpha_16(self) -> None:
        assert _alpha(16) == 0.673

    def test_alpha_32(self) -> None:
        assert _alpha(32) == 0.697

    def test_alpha_64(self) -> None:
        assert _alpha(64) == 0.709

    def test_alpha_128(self) -> None:
        expected = 0.7213 / (1.0 + 1.079 / 128)
        assert abs(_alpha(128) - expected) < 1e-12

    def test_alpha_large(self) -> None:
        # For large m, alpha converges to ~0.7213
        alpha_large = _alpha(16_384)
        assert 0.720 < alpha_large < 0.722


# ---------------------------------------------------------------------------
# 9. Small and large range corrections
# ---------------------------------------------------------------------------

class TestRangeCorrections:
    """Verify that the small-range LinearCounting correction kicks in properly."""

    def test_small_range_10_elements(self) -> None:
        """With very few elements, LinearCounting should give a reasonable answer."""
        hll = HyperLogLog(precision=14)
        for i in range(10):
            hll.add(str(i))
        est = hll.count()
        # Very small cardinality; rough bounds
        assert 5 <= est <= 20

    def test_small_range_100_elements(self) -> None:
        """100 elements with precision=14 → many empty registers → LinearCounting."""
        hll = HyperLogLog(precision=14)
        for i in range(100):
            hll.add(f"item_{i}")
        est = hll.count()
        assert 70 <= est <= 130

    def test_count_consistency_after_many_adds(self) -> None:
        """count() must be idempotent — calling it multiple times returns same value."""
        hll = HyperLogLog(precision=14)
        for i in range(5_000):
            hll.add(i)
        first = hll.count()
        second = hll.count()
        assert first == second


# ---------------------------------------------------------------------------
# 10. Memory invariant
# ---------------------------------------------------------------------------

class TestMemoryInvariant:
    """Registers list length must equal num_registers for all valid precisions."""

    def test_register_count_matches_precision(self) -> None:
        for b in [4, 6, 8, 10, 12, 14, 16]:
            hll = HyperLogLog(precision=b)
            assert len(hll._registers) == 2 ** b
            assert hll.num_registers == 2 ** b

    def test_registers_initially_zero(self) -> None:
        hll = HyperLogLog(precision=10)
        assert all(r == 0 for r in hll._registers)

    def test_registers_non_negative_after_adds(self) -> None:
        hll = HyperLogLog(precision=10)
        for i in range(200):
            hll.add(i)
        assert all(r >= 0 for r in hll._registers)

    def test_memory_bytes_formula(self) -> None:
        for b in [4, 8, 10, 12, 14, 16]:
            expected = (2 ** b * 6) // 8
            assert HyperLogLog.memory_bytes(b) == expected
