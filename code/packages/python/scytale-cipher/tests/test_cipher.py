"""Comprehensive tests for the Scytale cipher implementation.

These tests verify that the Scytale transposition cipher correctly
rearranges characters during encryption and decryption, handles padding,
validates keys, and supports brute-force decryption.

Test Categories
---------------

1. Basic encryption: Known plaintext/key -> ciphertext pairs.
2. Basic decryption: Known ciphertext/key -> plaintext pairs.
3. Round-trip: decrypt(encrypt(text, key), key) == text for all inputs.
4. Padding behavior: Verify space padding on incomplete last rows.
5. Key validation: Invalid keys raise ValueError.
6. Brute force: All keys tried, correct one produces original text.
7. Edge cases: Empty strings, minimum-length texts, key == len(text).
8. Character preservation: All character types are transposed, not substituted.
"""

import pytest

from scytale_cipher import brute_force, decrypt, encrypt


class TestBasicEncryption:
    """Test Scytale encryption with known input/output pairs."""

    def test_encrypt_hello_world_key3(self) -> None:
        """HELLO WORLD with key=3 should produce HLWLEOODL R .

        Grid (4 rows x 3 cols):
            H E L
            L O ' '
            W O R
            L D ' '

        Columns: HLWL + EOOD + L R  = HLWLEOODL R
        """
        assert encrypt("HELLO WORLD", 3) == "HLWLEOODL R "

    def test_encrypt_abcdef_key2(self) -> None:
        """ABCDEF with key=2: grid is 3x2, columns ACE + BDF = ACEBDF."""
        assert encrypt("ABCDEF", 2) == "ACEBDF"

    def test_encrypt_abcdef_key3(self) -> None:
        """ABCDEF with key=3: grid is 2x3, columns AD + BE + CF = ADBECF."""
        assert encrypt("ABCDEF", 3) == "ADBECF"

    def test_encrypt_abcdefgh_key4(self) -> None:
        """ABCDEFGH with key=4: grid is 2x4.

        A B C D
        E F G H

        Columns: AE + BF + CG + DH = AEBFCGDH
        """
        assert encrypt("ABCDEFGH", 4) == "AEBFCGDH"

    def test_encrypt_with_punctuation(self) -> None:
        """Punctuation and digits are transposed, not removed."""
        result = encrypt("AB,CD!", 2)
        # Grid: A B    Columns: AC! + B,D = AC!B,D
        #        , C
        #        D !
        # Actually: 6 chars, key=2, rows=3
        # Grid:  A B
        #        , C
        #        D !
        # col0: A,D  col1: BC!
        assert result == "A,DBC!"

    def test_encrypt_all_spaces(self) -> None:
        """A string of all spaces should remain all spaces."""
        assert encrypt("    ", 2) == "    "

    def test_encrypt_key_equals_length(self) -> None:
        """When key == len(text), there's 1 row, so ciphertext == plaintext."""
        assert encrypt("ABCD", 4) == "ABCD"


class TestBasicDecryption:
    """Test Scytale decryption with known ciphertext/key pairs."""

    def test_decrypt_hello_world_key3(self) -> None:
        """Reverse of the HELLO WORLD encryption example."""
        assert decrypt("HLWLEOODL R ", 3) == "HELLO WORLD"

    def test_decrypt_acebdf_key2(self) -> None:
        assert decrypt("ACEBDF", 2) == "ABCDEF"

    def test_decrypt_adbecf_key3(self) -> None:
        assert decrypt("ADBECF", 3) == "ABCDEF"

    def test_decrypt_strips_trailing_padding(self) -> None:
        """Trailing padding spaces added during encryption are stripped."""
        ciphertext = encrypt("HELLO", 3)
        assert decrypt(ciphertext, 3) == "HELLO"

    def test_decrypt_preserves_internal_spaces(self) -> None:
        """Internal spaces in the original text are preserved."""
        ciphertext = encrypt("A B C", 2)
        assert decrypt(ciphertext, 2) == "A B C"


class TestRoundTrip:
    """Verify decrypt(encrypt(text, key), key) == text for various inputs.

    This is the fundamental correctness property: encrypting then decrypting
    with the same key must return the original text.
    """

    @pytest.mark.parametrize(
        "text,key",
        [
            ("HELLO WORLD", 3),
            ("ABCDEF", 2),
            ("ABCDEF", 3),
            ("The quick brown fox", 4),
            ("12345", 2),
            ("AB", 2),
            ("ABCDEFGHIJKLMNOP", 4),
            ("Test with spaces and 123!", 5),
            ("a", 1),  # This will fail validation — tested in edge cases
        ],
    )
    def test_round_trip(self, text: str, key: int) -> None:
        if key < 2 or key > len(text):
            pytest.skip("Invalid key for this text length")
        assert decrypt(encrypt(text, key), key) == text

    def test_round_trip_long_text(self) -> None:
        """Round-trip with a longer text to stress-test correctness."""
        text = "The quick brown fox jumps over the lazy dog! 1234567890."
        for key in range(2, len(text) // 2 + 1):
            assert decrypt(encrypt(text, key), key) == text


class TestPadding:
    """Test that padding is correctly added and removed."""

    def test_no_padding_needed(self) -> None:
        """When text length is divisible by key, no padding is added."""
        # 6 chars, key=2 -> 3 rows, no padding
        ct = encrypt("ABCDEF", 2)
        assert len(ct) == 6

    def test_padding_added(self) -> None:
        """When text length is not divisible by key, spaces are padded."""
        # 5 chars, key=3 -> ceil(5/3)=2 rows -> 6 padded chars
        ct = encrypt("HELLO", 3)
        assert len(ct) == 6

    def test_padding_stripped_on_decrypt(self) -> None:
        """Padding spaces are removed during decryption."""
        ct = encrypt("HELLO", 3)
        assert decrypt(ct, 3) == "HELLO"

    def test_single_char_padding(self) -> None:
        """One character of padding when len(text) % key == key - 1."""
        # "HELLO WORLD" is 11 chars, key=3 -> 12 padded (1 pad char)
        ct = encrypt("HELLO WORLD", 3)
        assert len(ct) == 12


class TestKeyValidation:
    """Test that invalid keys raise ValueError."""

    def test_key_zero(self) -> None:
        with pytest.raises(ValueError, match="Key must be >= 2"):
            encrypt("HELLO", 0)

    def test_key_one(self) -> None:
        with pytest.raises(ValueError, match="Key must be >= 2"):
            encrypt("HELLO", 1)

    def test_key_negative(self) -> None:
        with pytest.raises(ValueError, match="Key must be >= 2"):
            encrypt("HELLO", -1)

    def test_key_too_large_encrypt(self) -> None:
        with pytest.raises(ValueError, match="Key must be <= text length"):
            encrypt("HI", 3)

    def test_key_too_large_decrypt(self) -> None:
        with pytest.raises(ValueError, match="Key must be <= text length"):
            decrypt("HI", 3)

    def test_decrypt_key_zero(self) -> None:
        with pytest.raises(ValueError, match="Key must be >= 2"):
            decrypt("HELLO", 0)


class TestBruteForce:
    """Test the brute-force decryption function."""

    def test_brute_force_finds_original(self) -> None:
        """The correct key should appear in brute_force results."""
        original = "HELLO WORLD"
        key = 3
        ct = encrypt(original, key)
        results = brute_force(ct)
        found = [r for r in results if r["key"] == key]
        assert len(found) == 1
        assert found[0]["text"] == original

    def test_brute_force_returns_all_keys(self) -> None:
        """brute_force should try keys from 2 to len//2."""
        ct = "ABCDEFGHIJ"  # 10 chars
        results = brute_force(ct)
        keys = [r["key"] for r in results]
        assert keys == [2, 3, 4, 5]

    def test_brute_force_short_text(self) -> None:
        """Very short text (< 4 chars) returns empty list."""
        assert brute_force("AB") == []
        assert brute_force("ABC") == []

    def test_brute_force_each_result_has_key_and_text(self) -> None:
        """Each result dict should have 'key' and 'text' fields."""
        results = brute_force("ABCDEFGH")
        for r in results:
            assert "key" in r
            assert "text" in r
            assert isinstance(r["key"], int)
            assert isinstance(r["text"], str)


class TestEdgeCases:
    """Edge cases: empty strings, single characters, etc."""

    def test_empty_string_encrypt(self) -> None:
        assert encrypt("", 2) == ""

    def test_empty_string_decrypt(self) -> None:
        assert decrypt("", 2) == ""

    def test_key_equals_text_length(self) -> None:
        """Key == text length means 1 row, plaintext == ciphertext."""
        text = "ABCD"
        assert encrypt(text, 4) == text
        assert decrypt(text, 4) == text

    def test_key_equals_two(self) -> None:
        """Key=2 is the minimum valid key; it swaps even/odd positions."""
        assert encrypt("ABCDEF", 2) == "ACEBDF"
        assert decrypt("ACEBDF", 2) == "ABCDEF"


class TestCharacterPreservation:
    """Verify that ALL character types are preserved through transposition."""

    def test_digits_preserved(self) -> None:
        ct = encrypt("123456", 2)
        pt = decrypt(ct, 2)
        assert pt == "123456"

    def test_punctuation_preserved(self) -> None:
        text = "Hello, World!"
        ct = encrypt(text, 4)
        pt = decrypt(ct, 4)
        assert pt == text

    def test_newlines_preserved(self) -> None:
        text = "AB\nCD\nEF"
        ct = encrypt(text, 2)
        pt = decrypt(ct, 2)
        assert pt == text

    def test_unicode_preserved(self) -> None:
        """Non-ASCII characters are transposed correctly."""
        text = "cafe\u0301"  # "cafe" + combining accent
        ct = encrypt(text, 2)
        pt = decrypt(ct, 2)
        assert pt == text


class TestVersion:
    """Verify package metadata is accessible."""

    def test_version_exists(self) -> None:
        from scytale_cipher import __version__

        assert __version__ == "0.1.0"

    def test_all_exports(self) -> None:
        from scytale_cipher import __all__

        assert "encrypt" in __all__
        assert "decrypt" in __all__
        assert "brute_force" in __all__
