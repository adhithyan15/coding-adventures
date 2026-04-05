"""Tests for the Vigenere cipher implementation.

These tests verify encryption, decryption, round-trip correctness, and
the cryptanalysis tools (find_key_length, find_key, break_cipher).

The parity test vectors are shared across all 9 language implementations
to ensure identical behavior.
"""

import pytest

from vigenere_cipher import break_cipher, decrypt, encrypt, find_key, find_key_length

# ---------------------------------------------------------------------------
# Long English text for cryptanalysis testing
# ---------------------------------------------------------------------------
# This paragraph provides enough statistical signal (~300 chars) for the
# IC-based key length estimation and chi-squared key recovery to work
# reliably. The text must be pure English prose -- the more "normal" the
# letter distribution, the better the cryptanalysis performs.
LONG_ENGLISH_TEXT = (
    "The quick brown fox jumps over the lazy dog near the riverbank where "
    "the tall grass sways gently in the warm summer breeze and the birds "
    "sing their melodious songs while the sun sets behind the distant "
    "mountains casting long shadows across the peaceful valley below and "
    "the farmers return from the golden fields carrying baskets of fresh "
    "wheat and corn while their children play happily in the meadows "
    "chasing butterflies and picking wildflowers that grow abundantly "
    "along the winding country roads that lead through the ancient forest "
    "where owls hoot softly in the towering oak trees above the mossy "
    "ground covered with fallen leaves and acorns from the previous autumn"
)


# ===========================================================================
# Encryption Tests
# ===========================================================================


class TestEncrypt:
    """Test suite for Vigenere encryption."""

    def test_parity_vector_attackatdawn(self) -> None:
        """Parity test: encrypt('ATTACKATDAWN', 'LEMON') == 'LXFOPVEFRNHR'."""
        assert encrypt("ATTACKATDAWN", "LEMON") == "LXFOPVEFRNHR"

    def test_parity_vector_mixed_case(self) -> None:
        """Parity test: encrypt('Hello, World!', 'key') == 'Rijvs, Uyvjn!'."""
        assert encrypt("Hello, World!", "key") == "Rijvs, Uyvjn!"

    def test_empty_plaintext(self) -> None:
        """Encrypting empty string returns empty string."""
        assert encrypt("", "KEY") == ""

    def test_single_character(self) -> None:
        """Encrypting a single letter applies the first key shift."""
        # A + B(=1) = B
        assert encrypt("A", "B") == "B"
        # Z + A(=0) = Z
        assert encrypt("Z", "A") == "Z"
        # Z + B(=1) = A (wraps around)
        assert encrypt("Z", "B") == "A"

    def test_preserves_non_alpha(self) -> None:
        """Non-alphabetic characters pass through unchanged."""
        result = encrypt("123!@#", "key")
        assert result == "123!@#"

    def test_key_does_not_advance_on_non_alpha(self) -> None:
        """Key position should not advance on spaces/punctuation.

        With key "AB" (shifts 0, 1):
          'A' shifted by A(0) = 'A', key advances to position 1
          ' ' passes through, key stays at position 1
          'A' shifted by B(1) = 'B', key advances to position 0
        """
        assert encrypt("A A", "AB") == "A B"

    def test_case_insensitive_key(self) -> None:
        """Key should work the same regardless of case."""
        text = "HELLO"
        assert encrypt(text, "KEY") == encrypt(text, "key")
        assert encrypt(text, "Key") == encrypt(text, "key")

    def test_all_uppercase(self) -> None:
        """All uppercase input produces all uppercase output."""
        result = encrypt("ABCDEF", "ABC")
        assert result.isalpha()
        assert result.isupper()

    def test_all_lowercase(self) -> None:
        """All lowercase input produces all lowercase output."""
        result = encrypt("abcdef", "abc")
        assert result.isalpha()
        assert result.islower()

    def test_invalid_key_empty(self) -> None:
        """Empty key raises ValueError."""
        with pytest.raises(ValueError, match="must not be empty"):
            encrypt("hello", "")

    def test_invalid_key_non_alpha(self) -> None:
        """Key with non-alphabetic characters raises ValueError."""
        with pytest.raises(ValueError, match="only letters"):
            encrypt("hello", "key1")

    def test_long_key(self) -> None:
        """Key longer than plaintext still works (extra key chars unused)."""
        result = encrypt("Hi", "ABCDEFGHIJ")
        # H + A(0) = H, i + B(1) = j
        assert result == "Hj"


# ===========================================================================
# Decryption Tests
# ===========================================================================


class TestDecrypt:
    """Test suite for Vigenere decryption."""

    def test_parity_vector_attackatdawn(self) -> None:
        """Parity test: decrypt('LXFOPVEFRNHR', 'LEMON') == 'ATTACKATDAWN'."""
        assert decrypt("LXFOPVEFRNHR", "LEMON") == "ATTACKATDAWN"

    def test_parity_vector_mixed_case(self) -> None:
        """Parity test: decrypt('Rijvs, Uyvjn!', 'key') == 'Hello, World!'."""
        assert decrypt("Rijvs, Uyvjn!", "key") == "Hello, World!"

    def test_empty_ciphertext(self) -> None:
        """Decrypting empty string returns empty string."""
        assert decrypt("", "KEY") == ""

    def test_single_character(self) -> None:
        """Decrypting a single letter reverses the first key shift."""
        # B - B(=1) = A
        assert decrypt("B", "B") == "A"

    def test_preserves_non_alpha(self) -> None:
        """Non-alphabetic characters pass through unchanged."""
        assert decrypt("123!@#", "key") == "123!@#"

    def test_invalid_key_empty(self) -> None:
        """Empty key raises ValueError."""
        with pytest.raises(ValueError):
            decrypt("hello", "")

    def test_invalid_key_non_alpha(self) -> None:
        """Key with digits raises ValueError."""
        with pytest.raises(ValueError):
            decrypt("hello", "k3y")


# ===========================================================================
# Round-Trip Tests
# ===========================================================================


class TestRoundTrip:
    """Verify that decrypt(encrypt(text, key), key) == text for all inputs."""

    @pytest.mark.parametrize(
        "text,key",
        [
            ("ATTACKATDAWN", "LEMON"),
            ("Hello, World!", "key"),
            ("The quick brown fox!", "SECRET"),
            ("abcdefghijklmnopqrstuvwxyz", "Z"),
            ("AAAAAA", "ABCDEF"),
            ("12345 numbers 67890", "test"),
            ("MiXeD CaSe TeXt!!!", "MiXeD"),
            ("", "anykey"),
        ],
    )
    def test_round_trip(self, text: str, key: str) -> None:
        """decrypt(encrypt(text, key), key) must equal text."""
        assert decrypt(encrypt(text, key), key) == text


# ===========================================================================
# Cryptanalysis Tests
# ===========================================================================


class TestFindKeyLength:
    """Test the IC-based key length estimation."""

    def test_finds_correct_key_length(self) -> None:
        """find_key_length should recover the correct key length."""
        key = "SECRET"
        ciphertext = encrypt(LONG_ENGLISH_TEXT, key)
        estimated = find_key_length(ciphertext)
        assert estimated == len(key)

    def test_short_key(self) -> None:
        """Key length 4 should be recoverable."""
        key = "DAWN"
        ciphertext = encrypt(LONG_ENGLISH_TEXT, key)
        estimated = find_key_length(ciphertext)
        assert estimated == len(key)

    def test_medium_key(self) -> None:
        """Key length 5 should be recoverable."""
        key = "LEMON"
        ciphertext = encrypt(LONG_ENGLISH_TEXT, key)
        estimated = find_key_length(ciphertext)
        assert estimated == len(key)


class TestFindKey:
    """Test the chi-squared key recovery."""

    def test_finds_correct_key(self) -> None:
        """find_key should recover the exact key when given correct length."""
        key = "SECRET"
        ciphertext = encrypt(LONG_ENGLISH_TEXT, key)
        recovered = find_key(ciphertext, len(key))
        assert recovered == key

    def test_finds_lemon_key(self) -> None:
        """find_key should recover 'LEMON' from long text."""
        key = "LEMON"
        ciphertext = encrypt(LONG_ENGLISH_TEXT, key)
        recovered = find_key(ciphertext, len(key))
        assert recovered == key


class TestBreakCipher:
    """Test the full automatic cipher breaking."""

    def test_break_cipher_recovers_key_and_plaintext(self) -> None:
        """break_cipher should recover both key and plaintext."""
        key = "SECRET"
        ciphertext = encrypt(LONG_ENGLISH_TEXT, key)
        recovered_key, recovered_plaintext = break_cipher(ciphertext)
        assert recovered_key == key
        assert recovered_plaintext == LONG_ENGLISH_TEXT

    def test_break_cipher_with_mixed_case(self) -> None:
        """break_cipher works on mixed-case text with punctuation."""
        key = "LEMON"
        ciphertext = encrypt(LONG_ENGLISH_TEXT, key)
        recovered_key, recovered_plaintext = break_cipher(ciphertext)
        assert recovered_key == key
        assert recovered_plaintext == LONG_ENGLISH_TEXT


# ===========================================================================
# Edge Case Tests
# ===========================================================================


class TestEdgeCases:
    """Edge cases and boundary conditions."""

    def test_key_a_is_identity(self) -> None:
        """Key 'A' (shift 0) should leave plaintext unchanged."""
        text = "Hello, World!"
        assert encrypt(text, "A") == text

    def test_key_z_wraps_correctly(self) -> None:
        """Key 'Z' (shift 25) should wrap A->Z, B->A, etc."""
        assert encrypt("A", "Z") == "Z"
        assert encrypt("B", "Z") == "A"

    def test_only_non_alpha_characters(self) -> None:
        """Text with no letters returns the same text."""
        assert encrypt("123 !@# $%^", "key") == "123 !@# $%^"
        assert decrypt("123 !@# $%^", "key") == "123 !@# $%^"
