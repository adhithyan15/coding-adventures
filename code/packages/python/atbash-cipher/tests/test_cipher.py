"""Comprehensive tests for the Atbash cipher implementation.

These tests verify that the Atbash cipher correctly reverses the alphabet
for both uppercase and lowercase letters, preserves non-alphabetic
characters, and satisfies the self-inverse property.

Test Categories
---------------

1. Basic encryption: Known plaintext -> ciphertext pairs.
2. Case preservation: Uppercase stays uppercase, lowercase stays lowercase.
3. Non-alpha passthrough: Digits, punctuation, spaces are unchanged.
4. Self-inverse property: encrypt(encrypt(text)) == text for all inputs.
5. Full alphabet mapping: Every letter maps to its correct reverse.
6. Edge cases: Empty strings, single characters, all-non-alpha strings.
7. Decrypt function: Verifies decrypt is the inverse of encrypt.
"""

from atbash_cipher import decrypt, encrypt


class TestBasicEncryption:
    """Test basic Atbash encryption with known input/output pairs."""

    def test_encrypt_hello_uppercase(self) -> None:
        """HELLO should encrypt to SVOOL.

        H(7)->S(18), E(4)->V(21), L(11)->O(14), L(11)->O(14), O(14)->L(11)
        """
        assert encrypt("HELLO") == "SVOOL"

    def test_encrypt_hello_lowercase(self) -> None:
        """hello should encrypt to svool, preserving lowercase."""
        assert encrypt("hello") == "svool"

    def test_encrypt_mixed_case_with_punctuation(self) -> None:
        """Mixed case with punctuation: non-alpha chars pass through."""
        assert encrypt("Hello, World! 123") == "Svool, Dliow! 123"

    def test_encrypt_full_uppercase_alphabet(self) -> None:
        """The full uppercase alphabet should reverse completely."""
        assert encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ") == "ZYXWVUTSRQPONMLKJIHGFEDCBA"

    def test_encrypt_full_lowercase_alphabet(self) -> None:
        """The full lowercase alphabet should also reverse completely."""
        assert encrypt("abcdefghijklmnopqrstuvwxyz") == "zyxwvutsrqponmlkjihgfedcba"


class TestCasePreservation:
    """Verify that character case is always preserved."""

    def test_uppercase_stays_uppercase(self) -> None:
        """An uppercase input letter must produce an uppercase output."""
        result = encrypt("ABC")
        assert result == "ZYX"
        assert result.isupper()

    def test_lowercase_stays_lowercase(self) -> None:
        """A lowercase input letter must produce a lowercase output."""
        result = encrypt("abc")
        assert result == "zyx"
        assert result.islower()

    def test_mixed_case_preserved(self) -> None:
        """Each character retains its original case."""
        assert encrypt("AbCdEf") == "ZyXwVu"


class TestNonAlphaPassthrough:
    """Non-alphabetic characters must pass through unchanged."""

    def test_digits_unchanged(self) -> None:
        assert encrypt("12345") == "12345"

    def test_punctuation_unchanged(self) -> None:
        assert encrypt("!@#$%^&*()") == "!@#$%^&*()"

    def test_spaces_unchanged(self) -> None:
        assert encrypt("   ") == "   "

    def test_mixed_alpha_and_non_alpha(self) -> None:
        """Letters are encrypted; everything else passes through."""
        assert encrypt("A1B2C3") == "Z1Y2X3"

    def test_newlines_and_tabs(self) -> None:
        """Whitespace characters like newlines and tabs pass through."""
        assert encrypt("A\nB\tC") == "Z\nY\tX"


class TestSelfInverse:
    """The Atbash cipher is self-inverse: encrypt(encrypt(x)) == x.

    This is the most important mathematical property of the cipher.
    Because 25 - (25 - p) = p, applying the transformation twice
    returns to the original.
    """

    def test_self_inverse_hello(self) -> None:
        assert encrypt(encrypt("HELLO")) == "HELLO"

    def test_self_inverse_lowercase(self) -> None:
        assert encrypt(encrypt("hello")) == "hello"

    def test_self_inverse_mixed(self) -> None:
        assert encrypt(encrypt("Hello, World! 123")) == "Hello, World! 123"

    def test_self_inverse_full_alphabet(self) -> None:
        alpha = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        assert encrypt(encrypt(alpha)) == alpha

    def test_self_inverse_empty(self) -> None:
        assert encrypt(encrypt("")) == ""

    def test_self_inverse_single_char(self) -> None:
        for c in "AZaz09!":
            assert encrypt(encrypt(c)) == c

    def test_self_inverse_long_text(self) -> None:
        text = "The quick brown fox jumps over the lazy dog! 42"
        assert encrypt(encrypt(text)) == text


class TestEdgeCases:
    """Edge cases: empty strings, single characters, etc."""

    def test_empty_string(self) -> None:
        assert encrypt("") == ""

    def test_single_uppercase(self) -> None:
        assert encrypt("A") == "Z"
        assert encrypt("Z") == "A"
        assert encrypt("M") == "N"
        assert encrypt("N") == "M"

    def test_single_lowercase(self) -> None:
        assert encrypt("a") == "z"
        assert encrypt("z") == "a"

    def test_single_digit(self) -> None:
        assert encrypt("5") == "5"

    def test_single_space(self) -> None:
        assert encrypt(" ") == " "

    def test_no_letter_maps_to_itself(self) -> None:
        """No letter in the alphabet should map to itself under Atbash.

        This is because 25 - p == p only when p == 12.5, which is not
        an integer, so no letter position satisfies this equation.
        """
        for i in range(26):
            upper = chr(ord("A") + i)
            assert encrypt(upper) != upper, f"{upper} maps to itself!"

            lower = chr(ord("a") + i)
            assert encrypt(lower) != lower, f"{lower} maps to itself!"


class TestDecrypt:
    """Verify decrypt works correctly (it should be identical to encrypt)."""

    def test_decrypt_svool(self) -> None:
        assert decrypt("SVOOL") == "HELLO"

    def test_decrypt_lowercase(self) -> None:
        assert decrypt("svool") == "hello"

    def test_decrypt_is_encrypt_inverse(self) -> None:
        """decrypt(encrypt(text)) should always return the original text."""
        texts = [
            "HELLO",
            "hello",
            "Hello, World! 123",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "",
            "42",
            "Test 123 !@#",
        ]
        for text in texts:
            assert decrypt(encrypt(text)) == text

    def test_encrypt_decrypt_equivalence(self) -> None:
        """encrypt and decrypt should produce identical output for same input."""
        texts = ["HELLO", "svool", "Test!", ""]
        for text in texts:
            assert encrypt(text) == decrypt(text)


class TestVersion:
    """Verify package metadata is accessible."""

    def test_version_exists(self) -> None:
        from atbash_cipher import __version__

        assert __version__ == "0.1.0"

    def test_all_exports(self) -> None:
        from atbash_cipher import __all__

        assert "encrypt" in __all__
        assert "decrypt" in __all__
