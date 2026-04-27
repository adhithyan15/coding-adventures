"""Tests for descriptive statistics functions.

Each test verifies the parity test vectors from the ST01 spec, ensuring
identical results across all language implementations.
"""

from __future__ import annotations

import math

import pytest

from stats.descriptive import (
    max,
    mean,
    median,
    min,
    mode,
    range,
    standard_deviation,
    variance,
)


# ── Mean ────────────────────────────────────────────────────────────────


class TestMean:
    """mean(values) -> arithmetic average."""

    def test_parity_vector(self) -> None:
        """ST01 parity: mean([1,2,3,4,5]) -> 3.0."""
        assert mean([1, 2, 3, 4, 5]) == 3.0

    def test_single_value(self) -> None:
        assert mean([42.0]) == 42.0

    def test_negative_values(self) -> None:
        assert mean([-3, -1, 0, 1, 3]) == 0.0

    def test_large_dataset(self) -> None:
        """Mean of 1..100 is 50.5."""
        assert mean(list(builtins_range(1, 101))) == 50.5

    def test_empty_raises(self) -> None:
        with pytest.raises(ValueError, match="at least one value"):
            mean([])

    def test_floating_point(self) -> None:
        assert mean([0.1, 0.2, 0.3]) == pytest.approx(0.2)


# ── Median ──────────────────────────────────────────────────────────────


class TestMedian:
    """median(values) -> middle value (average of two if even length)."""

    def test_parity_odd(self) -> None:
        """ST01 parity: median([1,2,3,4,5]) -> 3.0."""
        assert median([1, 2, 3, 4, 5]) == 3.0

    def test_parity_even(self) -> None:
        """ST01 parity: median([1,2,3,4]) -> 2.5."""
        assert median([1, 2, 3, 4]) == 2.5

    def test_single_value(self) -> None:
        assert median([7.0]) == 7.0

    def test_two_values(self) -> None:
        assert median([1.0, 3.0]) == 2.0

    def test_unsorted_input(self) -> None:
        """Median sorts internally; input order does not matter."""
        assert median([5, 1, 3, 2, 4]) == 3.0

    def test_empty_raises(self) -> None:
        with pytest.raises(ValueError, match="at least one value"):
            median([])


# ── Mode ────────────────────────────────────────────────────────────────


class TestMode:
    """mode(values) -> most frequent value, first occurrence wins ties."""

    def test_parity_vector(self) -> None:
        """ST01 parity: mode([1,2,2,3]) -> 2.0."""
        assert mode([1, 2, 2, 3]) == 2.0

    def test_single_value(self) -> None:
        assert mode([5.0]) == 5.0

    def test_tie_first_wins(self) -> None:
        """When multiple values tie, the first one in the list wins."""
        # 1 and 3 both appear twice; 1 appears first.
        assert mode([1, 3, 1, 3]) == 1.0

    def test_all_same(self) -> None:
        assert mode([7, 7, 7]) == 7.0

    def test_empty_raises(self) -> None:
        with pytest.raises(ValueError, match="at least one value"):
            mode([])


# ── Variance ────────────────────────────────────────────────────────────


class TestVariance:
    """variance(values, population=False) -> spread measure."""

    def test_parity_sample(self) -> None:
        """ST01 parity: sample variance of [2,4,4,4,5,5,7,9]."""
        result = variance([2, 4, 4, 4, 5, 5, 7, 9])
        assert result == pytest.approx(4.571428571428571)

    def test_parity_population(self) -> None:
        """ST01 parity: population variance of [2,4,4,4,5,5,7,9]."""
        result = variance([2, 4, 4, 4, 5, 5, 7, 9], population=True)
        assert result == pytest.approx(4.0)

    def test_zero_variance(self) -> None:
        """All identical values have zero variance."""
        assert variance([5, 5, 5, 5], population=True) == 0.0

    def test_single_value_population(self) -> None:
        assert variance([42.0], population=True) == 0.0

    def test_single_value_sample_raises(self) -> None:
        """Sample variance needs n >= 2 (division by n-1=0 is undefined)."""
        with pytest.raises(ValueError, match="at least two values"):
            variance([42.0])

    def test_empty_raises(self) -> None:
        with pytest.raises(ValueError, match="at least one value"):
            variance([])


# ── Standard Deviation ──────────────────────────────────────────────────


class TestStandardDeviation:
    """standard_deviation -> sqrt(variance)."""

    def test_sample(self) -> None:
        result = standard_deviation([2, 4, 4, 4, 5, 5, 7, 9])
        assert result == pytest.approx(math.sqrt(4.571428571428571))

    def test_population(self) -> None:
        result = standard_deviation([2, 4, 4, 4, 5, 5, 7, 9], population=True)
        assert result == pytest.approx(2.0)


# ── Min / Max / Range ──────────────────────────────────────────────────


class TestMinMaxRange:
    """min, max, range -> boundary and spread measures."""

    def test_min(self) -> None:
        assert min([3, 1, 4, 1, 5]) == 1.0

    def test_max(self) -> None:
        assert max([3, 1, 4, 1, 5]) == 5.0

    def test_range(self) -> None:
        assert range([2, 4, 4, 4, 5, 5, 7, 9]) == 7.0

    def test_range_single(self) -> None:
        assert range([5.0]) == 0.0

    def test_negative_values(self) -> None:
        assert min([-5, -1, 0, 3]) == -5.0
        assert max([-5, -1, 0, 3]) == 3.0
        assert range([-5, -1, 0, 3]) == 8.0

    def test_min_empty_raises(self) -> None:
        with pytest.raises(ValueError, match="at least one value"):
            min([])

    def test_max_empty_raises(self) -> None:
        with pytest.raises(ValueError, match="at least one value"):
            max([])


# ── Helpers ─────────────────────────────────────────────────────────────
# We import builtins.range since our module shadows it.
import builtins

builtins_range = builtins.range
