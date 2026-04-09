"""
Tests for the BloomFilter implementation (DT22).

Test strategy:
  1. Zero false negatives — every added element must always be found.
  2. False positive rate within 2× the configured target.
  3. Properties (bit_count, hash_count, fill_ratio, estimated FPR).
  4. from_params() factory method.
  5. Optimal parameter formulas (optimal_m, optimal_k, capacity_for_memory).
  6. Over-capacity detection.
  7. Determinism — same inputs produce the same bit array.
  8. Edge cases (empty filter, duplicate adds, non-string elements).
  9. __repr__ and __contains__ sugar.

Coverage target: 95%+ (enforced via pytest-cov in pyproject.toml).
"""

from __future__ import annotations

import random

import pytest

from bloom_filter import BloomFilter


# ---------------------------------------------------------------------------
# 1. Basic membership — no false negatives
# ---------------------------------------------------------------------------

class TestBasicMembership:
    """Fundamental add/contains behaviour."""

    def test_contains_added_string(self) -> None:
        """An element added to the filter must always be found."""
        bf = BloomFilter(expected_items=100)
        bf.add("hello")
        assert "hello" in bf

    def test_not_contains_absent_element(self) -> None:
        """
        Elements never added must not cause false negatives for added elements.

        We cannot assert that absent elements return False (false positives
        are allowed), but we can assert that every added element returns True.
        """
        bf = BloomFilter(expected_items=1000, false_positive_rate=0.001)
        for i in range(500):
            bf.add(f"item_{i}")
        for i in range(500):
            assert f"item_{i}" in bf

    def test_no_false_negatives_random(self) -> None:
        """Every added element must ALWAYS be found — no exceptions."""
        rng = random.Random(42)
        bf = BloomFilter(expected_items=1000)
        added = [rng.randint(0, 10000) for _ in range(500)]
        for x in added:
            bf.add(x)
        for x in added:
            assert x in bf, f"False negative for {x!r}"

    def test_no_false_negatives_large(self) -> None:
        """No false negatives over a large add set."""
        bf = BloomFilter(expected_items=10_000, false_positive_rate=0.01)
        added = [f"element_{i}" for i in range(10_000)]
        for elem in added:
            bf.add(elem)
        for elem in added:
            assert elem in bf, f"False negative for {elem!r}"

    def test_empty_filter_returns_false(self) -> None:
        """An empty filter must return False for everything."""
        bf = BloomFilter()
        assert "anything" not in bf
        assert 0 not in bf
        assert "" not in bf

    def test_add_duplicate_does_not_break(self) -> None:
        """Adding the same element twice must not cause any issues."""
        bf = BloomFilter()
        bf.add("dup")
        bf.add("dup")
        assert "dup" in bf

    def test_various_element_types(self) -> None:
        """add() and contains() accept any element (converted to str)."""
        bf = BloomFilter(expected_items=100)
        for elem in [42, 3.14, True, None, (1, 2), [3, 4]]:
            bf.add(elem)
            assert elem in bf


# ---------------------------------------------------------------------------
# 2. False positive rate
# ---------------------------------------------------------------------------

class TestFalsePositiveRate:
    """False positive rate should be near the configured target."""

    def test_fp_rate_within_2x_target(self) -> None:
        """
        FPR over 10,000 queries on elements not in the filter should be
        within 2× the configured target (probabilistic, very rarely fails).
        """
        bf = BloomFilter(expected_items=1000, false_positive_rate=0.01)
        for i in range(1000):
            bf.add(f"known_{i}")

        fp_count = sum(1 for i in range(10_000) if f"unknown_{i}" in bf)
        fp_rate = fp_count / 10_000
        assert fp_rate <= 0.02, f"FPR too high: {fp_rate:.3%}"

    def test_lower_fp_rate_needs_more_bits(self) -> None:
        """A lower FPR target must require a larger bit array."""
        bf_strict = BloomFilter(expected_items=1000, false_positive_rate=0.001)
        bf_loose = BloomFilter(expected_items=1000, false_positive_rate=0.1)
        assert bf_strict.bit_count > bf_loose.bit_count

    def test_more_items_needs_more_bits(self) -> None:
        """A larger expected_items must require a larger bit array."""
        bf_small = BloomFilter(expected_items=100)
        bf_large = BloomFilter(expected_items=10_000)
        assert bf_large.bit_count > bf_small.bit_count


# ---------------------------------------------------------------------------
# 3. Properties
# ---------------------------------------------------------------------------

class TestProperties:
    """bit_count, hash_count, bits_set, fill_ratio, estimated FPR."""

    def test_bit_count_is_positive(self) -> None:
        bf = BloomFilter(expected_items=500)
        assert bf.bit_count > 0

    def test_hash_count_is_at_least_1(self) -> None:
        bf = BloomFilter(expected_items=1000, false_positive_rate=0.01)
        assert bf.hash_count >= 1

    def test_bits_set_starts_at_zero(self) -> None:
        bf = BloomFilter()
        assert bf.bits_set == 0

    def test_bits_set_increases_after_add(self) -> None:
        bf = BloomFilter(expected_items=1000)
        bf.add("alpha")
        assert bf.bits_set > 0

    def test_bits_set_does_not_exceed_bit_count(self) -> None:
        bf = BloomFilter(expected_items=100)
        for i in range(200):
            bf.add(f"item_{i}")
        assert bf.bits_set <= bf.bit_count

    def test_fill_ratio_zero_when_empty(self) -> None:
        bf = BloomFilter()
        assert bf.fill_ratio == 0.0

    def test_fill_ratio_increases_with_adds(self) -> None:
        bf = BloomFilter(expected_items=1000)
        r0 = bf.fill_ratio
        bf.add("x")
        r1 = bf.fill_ratio
        assert r1 > r0

    def test_fill_ratio_between_0_and_1(self) -> None:
        bf = BloomFilter(expected_items=100)
        for i in range(200):
            bf.add(f"x_{i}")
        assert 0.0 <= bf.fill_ratio <= 1.0

    def test_estimated_fp_rate_zero_when_empty(self) -> None:
        bf = BloomFilter()
        assert bf.estimated_false_positive_rate == 0.0

    def test_estimated_fp_rate_increases_with_adds(self) -> None:
        bf = BloomFilter(expected_items=1000)
        r0 = bf.estimated_false_positive_rate
        for i in range(100):
            bf.add(f"item_{i}")
        r1 = bf.estimated_false_positive_rate
        assert r1 > r0

    def test_size_bytes_near_expected(self) -> None:
        """For 1M elements at 1% FPR, bit array should be ~1.14 MB."""
        bf = BloomFilter(expected_items=1_000_000, false_positive_rate=0.01)
        assert 1_100_000 < bf.size_bytes() < 1_200_000


# ---------------------------------------------------------------------------
# 4. from_params() factory
# ---------------------------------------------------------------------------

class TestFromParams:
    """BloomFilter.from_params() creates a filter with explicit m and k."""

    def test_from_params_sets_bit_count(self) -> None:
        bf = BloomFilter.from_params(bit_count=1000, hash_count=5)
        assert bf.bit_count == 1000

    def test_from_params_sets_hash_count(self) -> None:
        bf = BloomFilter.from_params(bit_count=1000, hash_count=5)
        assert bf.hash_count == 5

    def test_from_params_add_and_contains(self) -> None:
        bf = BloomFilter.from_params(bit_count=10_000, hash_count=7)
        bf.add("test")
        assert "test" in bf
        assert bf.bits_set > 0

    def test_from_params_no_capacity(self) -> None:
        """from_params() sets _n_expected=0, so is_over_capacity() → False."""
        bf = BloomFilter.from_params(bit_count=1000, hash_count=3)
        for i in range(1000):
            bf.add(f"x_{i}")
        assert not bf.is_over_capacity()


# ---------------------------------------------------------------------------
# 5. Optimal parameter formulas
# ---------------------------------------------------------------------------

class TestOptimalParams:
    """Static methods optimal_m, optimal_k, capacity_for_memory."""

    def test_optimal_m_1m_elements_1pct(self) -> None:
        """For 1M elements, 1% FPR → ~9,585,059 bits."""
        m = BloomFilter.optimal_m(1_000_000, 0.01)
        assert 9_500_000 < m < 9_700_000

    def test_optimal_m_lower_fpr_larger(self) -> None:
        """Lower FPR → more bits needed."""
        m1 = BloomFilter.optimal_m(1_000_000, 0.01)
        m2 = BloomFilter.optimal_m(1_000_000, 0.001)
        assert m2 > m1

    def test_optimal_k_1m_elements(self) -> None:
        """For m≈9.585M bits and n=1M elements → k=7."""
        m = BloomFilter.optimal_m(1_000_000, 0.01)
        k = BloomFilter.optimal_k(m, 1_000_000)
        assert k == 7

    def test_optimal_k_at_least_1(self) -> None:
        """k must always be at least 1."""
        k = BloomFilter.optimal_k(1, 1_000_000)
        assert k >= 1

    def test_capacity_for_memory_positive(self) -> None:
        """capacity_for_memory should return a positive integer."""
        cap = BloomFilter.capacity_for_memory(1_000_000, 0.01)
        assert cap > 0

    def test_capacity_for_memory_inverse_of_optimal_m(self) -> None:
        """
        capacity_for_memory should be approximately inverse of optimal_m.
        For n elements at FPR p, optimal_m gives m bits. Plugging m bytes back
        should give approximately n.
        """
        n = 500_000
        p = 0.01
        m_bits = BloomFilter.optimal_m(n, p)
        m_bytes = (m_bits + 7) // 8
        recovered_n = BloomFilter.capacity_for_memory(m_bytes, p)
        # Allow ±5% tolerance for rounding
        assert abs(recovered_n - n) / n < 0.05


# ---------------------------------------------------------------------------
# 6. Over-capacity detection
# ---------------------------------------------------------------------------

class TestOverCapacity:
    """Adding more elements than expected_items raises FPR."""

    def test_not_over_capacity_initially(self) -> None:
        bf = BloomFilter(expected_items=100)
        assert not bf.is_over_capacity()

    def test_over_capacity_after_exceeding_n(self) -> None:
        bf = BloomFilter(expected_items=10)
        for i in range(11):
            bf.add(f"elem_{i}")
        assert bf.is_over_capacity()

    def test_over_capacity_increases_estimated_fpr(self) -> None:
        """
        Adding 2× expected elements should raise estimated FPR above target.
        """
        n = 500
        bf = BloomFilter(expected_items=n, false_positive_rate=0.01)
        for i in range(2 * n):
            bf.add(f"element_{i}")
        assert bf.is_over_capacity()
        assert bf.estimated_false_positive_rate > 0.01


# ---------------------------------------------------------------------------
# 7. Determinism
# ---------------------------------------------------------------------------

class TestDeterminism:
    """Same inputs in same order → identical bit arrays."""

    def test_same_elements_same_bits(self) -> None:
        bf1 = BloomFilter(expected_items=1000, false_positive_rate=0.01)
        bf2 = BloomFilter(expected_items=1000, false_positive_rate=0.01)
        for i in range(100):
            bf1.add(f"item_{i}")
            bf2.add(f"item_{i}")
        assert bf1._bits == bf2._bits

    def test_different_elements_different_bits(self) -> None:
        bf1 = BloomFilter(expected_items=1000)
        bf2 = BloomFilter(expected_items=1000)
        bf1.add("alpha")
        bf2.add("beta")
        # Very likely to differ unless there's a collision (extremely unlikely)
        assert bf1._bits != bf2._bits or bf1.bits_set == 0


# ---------------------------------------------------------------------------
# 8. Edge cases
# ---------------------------------------------------------------------------

class TestEdgeCases:
    """Corner cases and boundary conditions."""

    def test_add_empty_string(self) -> None:
        bf = BloomFilter(expected_items=100)
        bf.add("")
        assert "" in bf

    def test_add_integer_zero(self) -> None:
        bf = BloomFilter(expected_items=100)
        bf.add(0)
        assert 0 in bf

    def test_add_none(self) -> None:
        bf = BloomFilter(expected_items=100)
        bf.add(None)
        assert None in bf

    def test_unicode_element(self) -> None:
        bf = BloomFilter(expected_items=100)
        bf.add("こんにちは")
        assert "こんにちは" in bf

    def test_long_string(self) -> None:
        bf = BloomFilter(expected_items=100)
        long_str = "x" * 10_000
        bf.add(long_str)
        assert long_str in bf

    def test_minimal_filter(self) -> None:
        """expected_items=1 should produce a working filter."""
        bf = BloomFilter(expected_items=1, false_positive_rate=0.01)
        bf.add("only")
        assert "only" in bf

    def test_bits_set_stable_on_duplicate(self) -> None:
        """Adding the same element twice must not double-count bits_set."""
        bf = BloomFilter(expected_items=1000)
        bf.add("dup")
        count_after_first = bf.bits_set
        bf.add("dup")
        assert bf.bits_set == count_after_first


# ---------------------------------------------------------------------------
# 9. __repr__ and __contains__
# ---------------------------------------------------------------------------

class TestReprAndContains:
    """String representation and __contains__ sugar."""

    def test_repr_contains_bloomfilter(self) -> None:
        bf = BloomFilter(expected_items=100)
        r = repr(bf)
        assert "BloomFilter" in r

    def test_repr_contains_m_and_k(self) -> None:
        bf = BloomFilter(expected_items=100)
        r = repr(bf)
        assert "m=" in r
        assert "k=" in r

    def test_contains_dunder_delegates_to_contains(self) -> None:
        """The `in` operator should behave identically to .contains()."""
        bf = BloomFilter(expected_items=100)
        bf.add("test")
        assert ("test" in bf) == bf.contains("test")
        assert ("missing" in bf) == bf.contains("missing")

    def test_repr_shows_bits_set(self) -> None:
        bf = BloomFilter(expected_items=100)
        bf.add("item")
        r = repr(bf)
        assert "bits_set" in r
