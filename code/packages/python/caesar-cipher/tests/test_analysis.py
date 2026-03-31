"""tests/test_analysis.py -- Comprehensive tests for Caesar cipher analysis tools.

These tests verify the brute-force and frequency-analysis attack methods:
- Brute force returns exactly 25 results (shifts 1-25)
- Brute force contains the correct plaintext for known ciphertext
- Frequency analysis identifies the correct shift for long English text
- Frequency analysis handles edge cases (all same letter, empty input)
- English frequency table is well-formed
"""

from __future__ import annotations

from caesar_cipher.analysis import (
    ENGLISH_FREQUENCIES,
    brute_force,
    frequency_analysis,
)
from caesar_cipher.cipher import encrypt

# ---------------------------------------------------------------------------
# English Frequencies table sanity checks
# ---------------------------------------------------------------------------


class TestEnglishFrequencies:
    """Verify the frequency table is complete and well-formed."""

    def test_has_26_entries(self) -> None:
        assert len(ENGLISH_FREQUENCIES) == 26

    def test_all_lowercase_letters_present(self) -> None:
        for letter in "abcdefghijklmnopqrstuvwxyz":
            assert letter in ENGLISH_FREQUENCIES

    def test_all_values_positive(self) -> None:
        for letter, freq in ENGLISH_FREQUENCIES.items():
            assert freq > 0, f"Frequency for '{letter}' should be positive"

    def test_frequencies_sum_to_approximately_one(self) -> None:
        total = sum(ENGLISH_FREQUENCIES.values())
        assert abs(total - 1.0) < 0.01

    def test_e_is_most_common(self) -> None:
        most_common = max(ENGLISH_FREQUENCIES, key=ENGLISH_FREQUENCIES.get)  # type: ignore[arg-type]
        assert most_common == "e"

    def test_z_is_least_common(self) -> None:
        least_common = min(ENGLISH_FREQUENCIES, key=ENGLISH_FREQUENCIES.get)  # type: ignore[arg-type]
        assert least_common == "z"


# ---------------------------------------------------------------------------
# Brute Force
# ---------------------------------------------------------------------------


class TestBruteForce:
    """Verify brute_force tries all 25 non-trivial shifts."""

    def test_returns_25_results(self) -> None:
        results = brute_force("KHOOR")
        assert len(results) == 25

    def test_shifts_are_1_through_25(self) -> None:
        results = brute_force("test")
        shifts = [shift for shift, _text in results]
        assert shifts == list(range(1, 26))

    def test_contains_correct_plaintext(self) -> None:
        """Encrypting 'HELLO' with shift=3 gives 'KHOOR'.
        Brute force should find 'HELLO' at shift=3."""
        results = brute_force("KHOOR")
        found = [(s, t) for s, t in results if t == "HELLO"]
        assert len(found) == 1
        assert found[0][0] == 3

    def test_brute_force_with_lowercase(self) -> None:
        ciphertext = encrypt("secret message", 17)
        results = brute_force(ciphertext)
        found = [(s, t) for s, t in results if t == "secret message"]
        assert len(found) == 1
        assert found[0][0] == 17

    def test_brute_force_empty_string(self) -> None:
        results = brute_force("")
        assert len(results) == 25
        assert all(text == "" for _shift, text in results)

    def test_brute_force_non_alpha(self) -> None:
        """Non-alpha characters are unchanged regardless of shift."""
        results = brute_force("123!@#")
        assert all(text == "123!@#" for _shift, text in results)

    def test_brute_force_preserves_case(self) -> None:
        results = brute_force("Khoor")
        found = [(s, t) for s, t in results if s == 3]
        assert found[0][1] == "Hello"


# ---------------------------------------------------------------------------
# Frequency Analysis
# ---------------------------------------------------------------------------


class TestFrequencyAnalysis:
    """Verify frequency_analysis correctly identifies the shift."""

    def test_long_english_text(self) -> None:
        """A long English sentence should be cracked reliably."""
        plaintext = (
            "the quick brown fox jumps over the lazy dog "
            "and the five boxing wizards jump quickly "
            "pack my box with five dozen liquor jugs "
            "how vexingly quick daft zebras jump"
        )
        for shift in [3, 7, 13, 19]:
            ciphertext = encrypt(plaintext, shift)
            detected_shift, decrypted = frequency_analysis(ciphertext)
            assert detected_shift == shift, (
                f"Expected shift={shift}, got shift={detected_shift}"
            )
            assert decrypted == plaintext

    def test_moderate_text(self) -> None:
        """A moderately long text should still work."""
        plaintext = "to be or not to be that is the question"
        ciphertext = encrypt(plaintext, 10)
        shift, decrypted = frequency_analysis(ciphertext)
        assert shift == 10
        assert decrypted == plaintext

    def test_empty_ciphertext(self) -> None:
        """Empty ciphertext should return shift 0 and empty string."""
        shift, plaintext = frequency_analysis("")
        assert shift == 0
        assert plaintext == ""

    def test_all_same_letter(self) -> None:
        """A string of all one letter is an edge case.
        The analysis should still return *some* result without crashing."""
        ciphertext = "AAAAAAAAAA"
        shift, plaintext = frequency_analysis(ciphertext)
        # We can't guarantee correctness for degenerate input,
        # but we can verify it returns valid types and doesn't crash.
        assert isinstance(shift, int)
        assert 0 <= shift <= 25
        assert isinstance(plaintext, str)
        assert len(plaintext) == 10

    def test_shift_zero_detected(self) -> None:
        """If the text is already English (shift=0), analysis should return 0."""
        plaintext = (
            "the quick brown fox jumps over the lazy dog "
            "and the five boxing wizards jump quickly"
        )
        shift, decrypted = frequency_analysis(plaintext)
        assert shift == 0
        assert decrypted == plaintext

    def test_case_insensitive_analysis(self) -> None:
        """Analysis should work regardless of case in the ciphertext."""
        plaintext = "The Quick Brown Fox Jumps Over The Lazy Dog"
        ciphertext = encrypt(plaintext, 5)
        shift, decrypted = frequency_analysis(ciphertext)
        assert shift == 5
        assert decrypted == plaintext

    def test_non_alpha_ignored(self) -> None:
        """Non-alphabetic characters should not affect frequency analysis."""
        plaintext = "hello world, this is a test! 123"
        ciphertext = encrypt(plaintext, 8)
        shift, decrypted = frequency_analysis(ciphertext)
        assert shift == 8
        assert decrypted == plaintext

    def test_only_non_alpha(self) -> None:
        """If the text has no letters, shift 0 should be returned."""
        shift, plaintext = frequency_analysis("12345!@#$%")
        # With no letters, all shifts produce chi^2=0 equally,
        # so shift=0 wins (first candidate).
        assert shift == 0
        assert plaintext == "12345!@#$%"
