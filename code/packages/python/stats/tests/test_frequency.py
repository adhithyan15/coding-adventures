"""Tests for frequency analysis functions."""

from __future__ import annotations

import pytest

from stats.frequency import (
    chi_squared,
    chi_squared_text,
    frequency_count,
    frequency_distribution,
)


# ── Frequency Count ─────────────────────────────────────────────────────


class TestFrequencyCount:
    """frequency_count(text) -> {letter: count}."""

    def test_parity_vector(self) -> None:
        """ST01 parity: frequency_count('Hello') -> {H:1, E:1, L:2, O:1}."""
        result = frequency_count("Hello")
        assert result == {"H": 1, "E": 1, "L": 2, "O": 1}

    def test_case_insensitive(self) -> None:
        """Lowercase and uppercase are treated as the same letter."""
        result = frequency_count("AaA")
        assert result == {"A": 3}

    def test_ignores_non_alpha(self) -> None:
        """Numbers, spaces, and punctuation are ignored."""
        result = frequency_count("A1 B! C?")
        assert result == {"A": 1, "B": 1, "C": 1}

    def test_empty_string(self) -> None:
        assert frequency_count("") == {}

    def test_numbers_only(self) -> None:
        assert frequency_count("12345") == {}

    def test_full_alphabet(self) -> None:
        result = frequency_count("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        assert len(result) == 26
        for letter in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
            assert result[letter] == 1


# ── Frequency Distribution ──────────────────────────────────────────────


class TestFrequencyDistribution:
    """frequency_distribution(text) -> {letter: proportion}."""

    def test_uniform(self) -> None:
        result = frequency_distribution("AABB")
        assert result == pytest.approx({"A": 0.5, "B": 0.5})

    def test_empty_string(self) -> None:
        assert frequency_distribution("") == {}

    def test_sums_to_one(self) -> None:
        result = frequency_distribution("HELLO WORLD")
        total = sum(result.values())
        assert total == pytest.approx(1.0)

    def test_single_letter(self) -> None:
        result = frequency_distribution("AAA")
        assert result == {"A": 1.0}


# ── Chi-Squared ─────────────────────────────────────────────────────────


class TestChiSquared:
    """chi_squared(observed, expected) -> float."""

    def test_parity_vector(self) -> None:
        """ST01 parity: chi_squared([10,20,30], [20,20,20]) -> 10.0."""
        result = chi_squared([10, 20, 30], [20, 20, 20])
        assert result == pytest.approx(10.0)

    def test_perfect_match(self) -> None:
        """Identical distributions have chi-squared = 0."""
        result = chi_squared([10, 20, 30], [10, 20, 30])
        assert result == pytest.approx(0.0)

    def test_length_mismatch_raises(self) -> None:
        with pytest.raises(ValueError, match="same length"):
            chi_squared([1, 2], [1, 2, 3])

    def test_single_element(self) -> None:
        result = chi_squared([5.0], [10.0])
        assert result == pytest.approx(2.5)


# ── Chi-Squared Text ───────────────────────────────────────────────────


class TestChiSquaredText:
    """chi_squared_text(text, expected_freq) -> float."""

    def test_perfect_match(self) -> None:
        """Text matching expected frequencies should have low chi-squared."""
        # 100 A's with expected freq A=1.0 -> perfect match
        result = chi_squared_text("A" * 100, {"A": 1.0})
        assert result == pytest.approx(0.0)

    def test_empty_text(self) -> None:
        result = chi_squared_text("", {"A": 0.5, "B": 0.5})
        assert result == 0.0

    def test_with_english_frequencies(self) -> None:
        """English text should have lower chi-squared than random text."""
        from stats.cryptanalysis import ENGLISH_FREQUENCIES

        english = "THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG"
        random_text = "ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ"
        chi_english = chi_squared_text(english, ENGLISH_FREQUENCIES)
        chi_random = chi_squared_text(random_text, ENGLISH_FREQUENCIES)
        assert chi_english < chi_random

    def test_case_insensitive(self) -> None:
        """Text comparison is case-insensitive."""
        r1 = chi_squared_text("HELLO", {"H": 0.2, "E": 0.2, "L": 0.4, "O": 0.2})
        r2 = chi_squared_text("hello", {"H": 0.2, "E": 0.2, "L": 0.4, "O": 0.2})
        assert r1 == pytest.approx(r2)
