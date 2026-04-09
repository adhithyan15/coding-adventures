"""
Tests for the avalanche_score and distribution_test analysis utilities.
"""

import pytest

from hash_functions import avalanche_score, distribution_test, fnv1a_32, murmur3_32


class TestAvalancheScore:
    """
    Verify that good hash functions achieve near-0.5 avalanche scores.
    """

    def test_fnv1a32_avalanche_in_range(self) -> None:
        score = avalanche_score(fnv1a_32, output_bits=32, sample_size=200)
        # A score between 0.35 and 0.65 is acceptable; 0.40–0.60 is good.
        assert 0.35 <= score <= 0.65, f"Avalanche score {score} out of range"

    def test_murmur3_avalanche_in_range(self) -> None:
        score = avalanche_score(murmur3_32, output_bits=32, sample_size=200)
        # MurmurHash3 has better avalanche than FNV-1a; expect tighter range.
        assert 0.40 <= score <= 0.60, f"Avalanche score {score} out of range"

    def test_returns_float(self) -> None:
        score = avalanche_score(fnv1a_32, output_bits=32, sample_size=10)
        assert isinstance(score, float)

    def test_score_bounded_0_to_1(self) -> None:
        score = avalanche_score(fnv1a_32, output_bits=32, sample_size=50)
        assert 0.0 <= score <= 1.0

    def test_custom_sample_size(self) -> None:
        # Should work with small sample sizes without error
        score = avalanche_score(fnv1a_32, output_bits=32, sample_size=5)
        assert 0.0 <= score <= 1.0


class TestDistributionTest:
    """
    Verify that good hash functions produce near-uniform bucket distribution.
    """

    def test_fnv1a32_chi_squared_reasonable(self) -> None:
        import random
        random.seed(99)
        inputs = [random.randbytes(8) for _ in range(10_000)]
        chi2 = distribution_test(fnv1a_32, inputs, num_buckets=100)
        # For 100 buckets and 10,000 inputs the expected chi-squared is 99.
        # A value below 200 indicates acceptable uniformity.
        assert chi2 < 200, f"Chi-squared {chi2} too high (poor distribution)"

    def test_murmur3_chi_squared_reasonable(self) -> None:
        import random
        random.seed(42)
        inputs = [random.randbytes(8) for _ in range(10_000)]
        chi2 = distribution_test(murmur3_32, inputs, num_buckets=100)
        assert chi2 < 150, f"Chi-squared {chi2} too high"

    def test_returns_float(self) -> None:
        result = distribution_test(fnv1a_32, [b"a", b"b"], num_buckets=10)
        assert isinstance(result, float)

    def test_single_input(self) -> None:
        # Should not divide by zero or crash with one input
        result = distribution_test(fnv1a_32, [b"hello"], num_buckets=10)
        assert isinstance(result, float)

    def test_chi_squared_low_for_uniform_hash(self) -> None:
        # If inputs are uniformly distributed across buckets manually,
        # chi-squared should be close to num_buckets - 1.
        # We test this indirectly: random inputs + good hash → low chi2.
        import random
        random.seed(7)
        inputs = [random.randbytes(16) for _ in range(5_000)]
        chi2 = distribution_test(murmur3_32, inputs, num_buckets=50)
        # Expected ≈ 49; allow for some statistical variance.
        assert chi2 < 150
