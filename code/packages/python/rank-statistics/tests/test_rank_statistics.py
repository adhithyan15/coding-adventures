import itertools
import json
import random
from math import nextafter
from pathlib import Path

import pytest

from rank_statistics import median, median_interval

FIXTURE_PATH = Path(__file__).with_name("microbenchmark-pairs-v1.json")
if not FIXTURE_PATH.is_file():
    FIXTURE_PATH = (
        Path(__file__).resolve().parents[4]
        / "specs/fixtures/microbenchmark-pairs-v1.json"
    )
FIXTURE = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))


def oracle_rank(n, confidence):
    """Enumerate all fair-coin outcomes rather than using binomial coefficients."""
    valid = []
    for rank in range(1, n // 2 + 2):
        tail = sum(
            sum(outcome) <= 2 * (rank - 1) - n
            for outcome in itertools.product((-1, 1), repeat=n)
        )
        if 200 * tail <= (100 - confidence) * (1 << n):
            valid.append(rank)
    return max(valid, default=0)


def verdict(interval, factor):
    if interval is None:
        return "inconclusive"
    low, high = interval
    if high < 1 / factor:
        return "faster"
    if low > factor:
        return "slower"
    if low >= 1 / factor and high <= factor:
        return "equivalent"
    return "inconclusive"


def test_median_even_odd_and_nonmutation():
    values = [4.0, 1.0, 3.0, 2.0]
    assert median(values) == 2.5
    assert values == [4.0, 1.0, 3.0, 2.0]
    assert median([9, 2, 5]) == 5.0


def test_midpoint_handles_float_extremes_and_subnormals():
    smallest = nextafter(0.0, 1.0)
    largest = 1.7976931348623157e308
    assert median([-largest, largest]) == 0.0
    assert median([largest, largest]) == largest
    assert median([smallest, smallest]) == smallest
    assert median([smallest, 2 * smallest]) == 2 * smallest


@pytest.mark.parametrize(
    "values",
    [[], [float("nan")], [float("inf")], [-float("inf")], [10**10000], [True]],
)
def test_median_rejects_empty_nonfinite_or_non_numeric_values(values):
    with pytest.raises(ValueError):
        median(values)


@pytest.mark.parametrize("confidence", [0, 100, -1, 75.0, True])
def test_interval_rejects_invalid_confidence(confidence):
    with pytest.raises(ValueError, match="confidence"):
        median_interval([1, 2], confidence)


def test_interval_rejects_empty_and_nonfinite_inputs():
    with pytest.raises(ValueError, match="empty"):
        median_interval([], 95)
    with pytest.raises(ValueError, match="finite"):
        median_interval([1, float("nan")], 95)


@pytest.mark.parametrize("confidence", [50, 75, 90, 95, 99])
def test_small_sample_cutoffs_match_independent_coin_oracle(confidence):
    for n in range(1, 13):
        rank = oracle_rank(n, confidence)
        got = median_interval(list(range(n)), confidence)
        assert got == (rank - 1, n - rank) if rank else got is None


def test_three_observation_75_76_percent_boundary():
    assert median_interval([1, 2, 3], 75) == (1.0, 3.0)
    assert median_interval([1, 2, 3], 76) is None


def test_ties_permutations_and_endpoints_are_observations():
    values = [1, 1, 2, 2, 2, 9, 9, 10, 10]
    expected = median_interval(values, 75)
    assert expected == (2.0, 9.0)
    assert median_interval(list(reversed(values)), 75) == expected
    assert expected[0] in values and expected[1] in values


def test_seeded_property_fuzz():
    rng = random.Random(75123)
    for _ in range(500):
        values = [rng.randint(-8, 8) for _ in range(rng.randint(1, 18))]
        before = list(values)
        confidence = rng.randint(1, 99)
        interval = median_interval(values, confidence)
        assert values == before
        if len(values) <= 12:
            rank = oracle_rank(len(values), confidence)
            if rank:
                expected = (sorted(values)[rank - 1], sorted(values)[-rank])
                assert interval == expected
            else:
                assert interval is None
        elif interval is not None:
            ordered = sorted(values)
            assert interval[0] in values and interval[1] in values
            assert interval[0] <= interval[1]
            assert ordered[0] <= interval[0] <= interval[1] <= ordered[-1]


@pytest.mark.parametrize("case", FIXTURE["cases"], ids=lambda case: case["name"])
def test_portable_fixture_replay_and_consumer_verdict(case):
    ratios = [candidate / case["baseline_ns"] for candidate in case["candidate_ns"]]
    estimate = median(ratios)
    interval = median_interval(ratios, FIXTURE["confidence_percent"])
    assert estimate == pytest.approx(case["ratio"])
    if case["interval"] is None:
        assert interval is None
    else:
        assert interval == pytest.approx(case["interval"])
    assert verdict(interval, FIXTURE["minimum_factor"]) == case["verdict"]
