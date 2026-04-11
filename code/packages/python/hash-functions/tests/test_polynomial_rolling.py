"""
Tests for the polynomial rolling hash.
"""

import pytest

from hash_functions import polynomial_rolling


class TestPolynomialRollingKnownValues:
    """Verify the polynomial rolling hash produces expected outputs."""

    def test_empty_returns_zero(self) -> None:
        # With an empty input, the loop never runs and h stays at 0.
        assert polynomial_rolling(b"") == 0

    def test_single_byte(self) -> None:
        # For a single byte b: hash = (0 * 31 + b) % mod = b
        assert polynomial_rolling(b"\x61") == 0x61  # 'a' = 97

    def test_two_bytes(self) -> None:
        # For "ab" (97, 98): h = (0*31 + 97) % mod = 97
        #                        (97*31 + 98) % mod = 3007 + 98 = 3105
        assert polynomial_rolling(b"ab") == 3105

    def test_str_input(self) -> None:
        assert polynomial_rolling("abc") == polynomial_rolling(b"abc")

    def test_deterministic(self) -> None:
        assert polynomial_rolling(b"hello") == polynomial_rolling(b"hello")

    def test_different_inputs_differ(self) -> None:
        assert polynomial_rolling(b"foo") != polynomial_rolling(b"bar")

    def test_output_in_range(self) -> None:
        mod = 2**61 - 1
        for data in [b"", b"a", b"hello", b"abc"]:
            result = polynomial_rolling(data)
            assert 0 <= result < mod

    def test_custom_base(self) -> None:
        r1 = polynomial_rolling(b"hello", base=31)
        r2 = polynomial_rolling(b"hello", base=37)
        assert r1 != r2

    def test_custom_mod(self) -> None:
        # Use a longer input to avoid the unlikely case where two different
        # moduli yield the same residue for a short string.
        data = b"hello world this is a longer test string"
        r1 = polynomial_rolling(data, mod=2**61 - 1)
        r2 = polynomial_rolling(data, mod=10**9 + 7)
        # With a Mersenne-prime mod and a billion+7 mod, results must differ
        # for non-trivial inputs (they live in different residue fields).
        assert r1 != r2

    def test_mod_is_respected(self) -> None:
        # With a tiny mod, results must still be in range
        result = polynomial_rolling(b"hello world", mod=100)
        assert 0 <= result < 100

    def test_unicode_str(self) -> None:
        s = "caf\u00e9"
        assert polynomial_rolling(s) == polynomial_rolling(s.encode("utf-8"))

    def test_order_matters(self) -> None:
        # Different byte orderings produce different hashes
        assert polynomial_rolling(b"abc") != polynomial_rolling(b"cba")

    def test_binary_safe(self) -> None:
        assert isinstance(polynomial_rolling(bytes(range(256))), int)


class TestPolynomialRollingRollingProperty:
    """
    Verify the 'rolling' property: a window can be slid in O(1).

    The rolling property means: if you remove the first character and
    add a new character, you can update the hash without recomputing
    from scratch.  We do not test the incremental API here (there is
    none), but we verify that the math is consistent.
    """

    def test_consistent_with_manual_computation(self) -> None:
        # hash("ab") manually: 97*31 + 98 = 3105
        base = 31
        mod = 2**61 - 1
        manual = (97 * base + 98) % mod
        assert polynomial_rolling(b"ab", base=base, mod=mod) == manual

    def test_consistent_longer_string(self) -> None:
        base = 31
        mod = 2**61 - 1
        # "abc": h=0 → 97 → 97*31+98=3105 → 3105*31+99=96354
        expected = ((97 * base + 98) * base + 99) % mod
        assert polynomial_rolling(b"abc", base=base, mod=mod) == expected
