"""Tests for cryptanalysis helper functions."""

from __future__ import annotations

import math

import pytest

from stats.cryptanalysis import (
    ENGLISH_FREQUENCIES,
    entropy,
    index_of_coincidence,
)


# ── Index of Coincidence ────────────────────────────────────────────────


class TestIndexOfCoincidence:
    """index_of_coincidence(text) -> float."""

    def test_parity_vector(self) -> None:
        """ST01 parity: IC of 'AABB' -> 0.333...

        counts: A=2, B=2, N=4
        numerator = 2*1 + 2*1 = 4
        denominator = 4*3 = 12
        IC = 4/12 = 0.3333...
        """
        result = index_of_coincidence("AABB")
        assert result == pytest.approx(1 / 3)

    def test_all_same_letter(self) -> None:
        """All same letters -> IC = 1.0."""
        result = index_of_coincidence("AAAA")
        assert result == pytest.approx(1.0)

    def test_all_different(self) -> None:
        """26 unique letters -> IC = 0.0 (each n_i=1, so n_i*(n_i-1)=0)."""
        result = index_of_coincidence("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        assert result == pytest.approx(0.0)

    def test_english_like(self) -> None:
        """Longer English text should have IC closer to 0.0667."""
        # A pangram has very uniform distribution (all 26 letters), so its IC
        # is low. We use a longer repeated text to get a more English-like IC.
        text = "TOBEORNOTTOBETHATISTHEQUESTION"
        result = index_of_coincidence(text)
        # Should be positive and in a reasonable range for English text.
        assert result > 0.0

    def test_empty_text(self) -> None:
        assert index_of_coincidence("") == 0.0

    def test_single_letter(self) -> None:
        assert index_of_coincidence("A") == 0.0

    def test_case_insensitive(self) -> None:
        """IC should be case-insensitive."""
        assert index_of_coincidence("aabb") == pytest.approx(
            index_of_coincidence("AABB")
        )

    def test_ignores_non_alpha(self) -> None:
        """Non-alphabetic characters should be ignored."""
        assert index_of_coincidence("A A B B") == pytest.approx(
            index_of_coincidence("AABB")
        )


# ── Entropy ─────────────────────────────────────────────────────────────


class TestEntropy:
    """entropy(text) -> Shannon entropy in bits."""

    def test_single_letter_repeated(self) -> None:
        """Single repeated letter has zero entropy (no surprise)."""
        assert entropy("AAAA") == pytest.approx(0.0)

    def test_two_equal_letters(self) -> None:
        """Two equally frequent letters -> entropy = 1.0 bit."""
        result = entropy("ABABABAB")
        assert result == pytest.approx(1.0)

    def test_uniform_26_letters(self) -> None:
        """ST01 parity: uniform 26 letters -> log2(26) ~ 4.700."""
        text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        result = entropy(text)
        assert result == pytest.approx(math.log2(26))

    def test_empty_text(self) -> None:
        assert entropy("") == 0.0

    def test_entropy_increases_with_diversity(self) -> None:
        """More diverse text should have higher entropy."""
        low_entropy = entropy("AAAB")
        high_entropy = entropy("ABCD")
        assert high_entropy > low_entropy

    def test_case_insensitive(self) -> None:
        assert entropy("aabb") == pytest.approx(entropy("AABB"))


# ── English Frequencies ─────────────────────────────────────────────────


class TestEnglishFrequencies:
    """ENGLISH_FREQUENCIES constant validation."""

    def test_has_26_entries(self) -> None:
        assert len(ENGLISH_FREQUENCIES) == 26

    def test_all_uppercase_keys(self) -> None:
        for key in ENGLISH_FREQUENCIES:
            assert "A" <= key <= "Z"

    def test_sums_to_approximately_one(self) -> None:
        total = sum(ENGLISH_FREQUENCIES.values())
        assert total == pytest.approx(1.0, abs=0.001)

    def test_e_is_most_frequent(self) -> None:
        """E should be the most frequent letter in English."""
        max_letter = max(ENGLISH_FREQUENCIES, key=ENGLISH_FREQUENCIES.get)  # type: ignore[arg-type]
        assert max_letter == "E"

    def test_z_is_least_frequent(self) -> None:
        """Z should be the least frequent letter in English."""
        min_letter = min(ENGLISH_FREQUENCIES, key=ENGLISH_FREQUENCIES.get)  # type: ignore[arg-type]
        assert min_letter == "Z"

    def test_specific_values(self) -> None:
        """Spot-check a few known frequency values."""
        assert ENGLISH_FREQUENCIES["A"] == pytest.approx(0.08167)
        assert ENGLISH_FREQUENCIES["E"] == pytest.approx(0.12702)
        assert ENGLISH_FREQUENCIES["Z"] == pytest.approx(0.00074)
