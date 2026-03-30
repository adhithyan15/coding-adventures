"""tests/test_cipher.py -- Comprehensive tests for Caesar cipher encrypt/decrypt/rot13.

These tests verify correctness of the core cipher operations including:
- Encrypt / decrypt round-trip for all shifts 0-25
- Case preservation (uppercase stays uppercase, lowercase stays lowercase)
- Non-alphabetic passthrough (digits, punctuation, spaces)
- Empty string handling
- Negative shifts
- Shift wrapping (26, 52, -26 behave as identity)
- ROT13 self-inverse property
- Classic worked example: "HELLO" -> "KHOOR" with shift 3
"""

from __future__ import annotations

import pytest

from caesar_cipher.cipher import decrypt, encrypt, rot13

# ---------------------------------------------------------------------------
# Classic worked example
# ---------------------------------------------------------------------------


class TestClassicExample:
    """The canonical Caesar cipher example: HELLO with shift 3."""

    def test_hello_encrypt(self) -> None:
        assert encrypt("HELLO", 3) == "KHOOR"

    def test_hello_decrypt(self) -> None:
        assert decrypt("KHOOR", 3) == "HELLO"


# ---------------------------------------------------------------------------
# Round-trip: encrypt then decrypt should return the original
# ---------------------------------------------------------------------------


class TestRoundTrip:
    """Encrypting and then decrypting with the same shift must be lossless."""

    @pytest.mark.parametrize("shift", list(range(26)))
    def test_round_trip_all_shifts(self, shift: int) -> None:
        plaintext = "The Quick Brown Fox Jumps Over The Lazy Dog!"
        assert decrypt(encrypt(plaintext, shift), shift) == plaintext

    def test_round_trip_lowercase(self) -> None:
        plaintext = "abcdefghijklmnopqrstuvwxyz"
        assert decrypt(encrypt(plaintext, 17), 17) == plaintext

    def test_round_trip_uppercase(self) -> None:
        plaintext = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        assert decrypt(encrypt(plaintext, 10), 10) == plaintext

    def test_round_trip_mixed(self) -> None:
        plaintext = "Hello, World! 123"
        assert decrypt(encrypt(plaintext, 5), 5) == plaintext


# ---------------------------------------------------------------------------
# Case preservation
# ---------------------------------------------------------------------------


class TestCasePreservation:
    """The cipher must preserve the case of each letter."""

    def test_uppercase_stays_uppercase(self) -> None:
        result = encrypt("ABC", 1)
        assert result == "BCD"
        assert result.isupper()

    def test_lowercase_stays_lowercase(self) -> None:
        result = encrypt("abc", 1)
        assert result == "bcd"
        assert result.islower()

    def test_mixed_case_preserved(self) -> None:
        result = encrypt("AbCdEf", 2)
        assert result == "CdEfGh"
        # Check case of each character individually
        assert result[0].isupper()  # A -> C
        assert result[1].islower()  # b -> d
        assert result[2].isupper()  # C -> E
        assert result[3].islower()  # d -> f
        assert result[4].isupper()  # E -> G
        assert result[5].islower()  # f -> h


# ---------------------------------------------------------------------------
# Non-alphabetic passthrough
# ---------------------------------------------------------------------------


class TestNonAlphaPassthrough:
    """Digits, punctuation, spaces, and other non-alpha chars pass through."""

    def test_digits_unchanged(self) -> None:
        assert encrypt("123456", 5) == "123456"

    def test_punctuation_unchanged(self) -> None:
        assert encrypt("!@#$%^&*()", 10) == "!@#$%^&*()"

    def test_spaces_unchanged(self) -> None:
        assert encrypt("   ", 7) == "   "

    def test_mixed_content(self) -> None:
        result = encrypt("Hello, World! 123.", 3)
        assert result == "Khoor, Zruog! 123."

    def test_newlines_and_tabs(self) -> None:
        assert encrypt("a\nb\tc", 1) == "b\nc\td"

    def test_unicode_passthrough(self) -> None:
        # Non-ASCII characters should pass through unchanged
        assert encrypt("cafe\u0301", 1) == "dbgf\u0301"


# ---------------------------------------------------------------------------
# Empty string
# ---------------------------------------------------------------------------


class TestEmptyString:
    """Empty input should produce empty output."""

    def test_encrypt_empty(self) -> None:
        assert encrypt("", 5) == ""

    def test_decrypt_empty(self) -> None:
        assert decrypt("", 5) == ""

    def test_rot13_empty(self) -> None:
        assert rot13("") == ""


# ---------------------------------------------------------------------------
# Negative shifts
# ---------------------------------------------------------------------------


class TestNegativeShifts:
    """Negative shifts should shift left (same as encrypting with 26-shift)."""

    def test_negative_one(self) -> None:
        assert encrypt("B", -1) == "A"

    def test_negative_three(self) -> None:
        assert encrypt("KHOOR", -3) == "HELLO"

    def test_negative_shift_equals_positive_complement(self) -> None:
        text = "Testing negative shifts"
        assert encrypt(text, -5) == encrypt(text, 21)

    def test_negative_wraps_around(self) -> None:
        assert encrypt("A", -1) == "Z"
        assert encrypt("a", -1) == "z"


# ---------------------------------------------------------------------------
# Shift wrapping
# ---------------------------------------------------------------------------


class TestShiftWrapping:
    """Shifts that are multiples of 26 should be identity; large shifts wrap."""

    def test_shift_zero_is_identity(self) -> None:
        text = "No change expected"
        assert encrypt(text, 0) == text

    def test_shift_26_is_identity(self) -> None:
        text = "Full rotation"
        assert encrypt(text, 26) == text

    def test_shift_52_is_identity(self) -> None:
        text = "Double rotation"
        assert encrypt(text, 52) == text

    def test_shift_negative_26_is_identity(self) -> None:
        text = "Negative full rotation"
        assert encrypt(text, -26) == text

    def test_shift_27_equals_shift_1(self) -> None:
        text = "Wrapping test"
        assert encrypt(text, 27) == encrypt(text, 1)

    def test_large_shift(self) -> None:
        text = "Very large shift"
        assert encrypt(text, 1000) == encrypt(text, 1000 % 26)


# ---------------------------------------------------------------------------
# ROT13
# ---------------------------------------------------------------------------


class TestRot13:
    """ROT13 is shift=13, and applying it twice returns the original."""

    def test_rot13_basic(self) -> None:
        assert rot13("Hello") == "Uryyb"

    def test_rot13_self_inverse(self) -> None:
        text = "The quick brown fox jumps over the lazy dog!"
        assert rot13(rot13(text)) == text

    def test_rot13_is_encrypt_13(self) -> None:
        text = "ROT13 test"
        assert rot13(text) == encrypt(text, 13)

    def test_rot13_all_letters(self) -> None:
        # A-M map to N-Z and vice versa
        assert rot13("ABCDEFGHIJKLM") == "NOPQRSTUVWXYZ"
        assert rot13("NOPQRSTUVWXYZ") == "ABCDEFGHIJKLM"

    def test_rot13_preserves_nonalpha(self) -> None:
        assert rot13("Hello, World! 123") == "Uryyb, Jbeyq! 123"


# ---------------------------------------------------------------------------
# Full alphabet shift
# ---------------------------------------------------------------------------


class TestFullAlphabet:
    """Verify the entire alphabet shifts correctly for a few key shifts."""

    def test_shift_1(self) -> None:
        assert encrypt("abcdefghijklmnopqrstuvwxyz", 1) == "bcdefghijklmnopqrstuvwxyza"

    def test_shift_13(self) -> None:
        assert encrypt("abcdefghijklmnopqrstuvwxyz", 13) == "nopqrstuvwxyzabcdefghijklm"

    def test_shift_25(self) -> None:
        assert encrypt("abcdefghijklmnopqrstuvwxyz", 25) == "zabcdefghijklmnopqrstuvwxy"
