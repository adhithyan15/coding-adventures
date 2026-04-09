"""
Tests for the DJB2 hash function by Dan Bernstein.
"""

import pytest

from hash_functions import djb2


class TestDjb2KnownVectors:
    """Verify against hand-computed and published DJB2 values."""

    def test_empty_string(self) -> None:
        # An empty input means the loop never executes; the initial hash
        # value 5381 is returned unchanged.
        assert djb2(b"") == 5381

    def test_single_a(self) -> None:
        # hash = 5381 * 33 + 97 = 177638 + 97 = 177670  (wait: 5381*33=177573+97=177670)
        # Actually: (5381 << 5) + 5381 + 97 = 172192 + 5381 + 97 = 177670
        assert djb2(b"a") == 177670

    def test_abc(self) -> None:
        # Trace:
        #   h = 5381
        #   'a': h = (5381<<5)+5381+97 = 177670
        #   'b': h = (177670<<5)+177670+98 = 5685440+177670+98 = 5863208
        #   'c': h = (5863208<<5)+5863208+99 = 187622656+5863208+99 = 193485963
        assert djb2(b"abc") == 193485963

    def test_str_input(self) -> None:
        assert djb2("abc") == djb2(b"abc")

    def test_deterministic(self) -> None:
        assert djb2(b"hello") == djb2(b"hello")

    def test_different_inputs_differ(self) -> None:
        assert djb2(b"foo") != djb2(b"bar")

    def test_returns_integer(self) -> None:
        result = djb2(b"hello")
        assert isinstance(result, int)
        assert result >= 0

    def test_binary_safe(self) -> None:
        assert isinstance(djb2(bytes(range(256))), int)

    def test_null_byte_changes_hash(self) -> None:
        assert djb2(b"a\x00b") != djb2(b"ab")

    def test_unicode_str(self) -> None:
        s = "caf\u00e9"
        assert djb2(s) == djb2(s.encode("utf-8"))

    def test_long_string(self) -> None:
        # Should handle arbitrary length input without errors
        data = b"x" * 10_000
        result = djb2(data)
        assert isinstance(result, int)
        assert result >= 0

    def test_output_bounded_by_64_bits(self) -> None:
        # We mask to 64 bits to prevent unbounded growth
        result = djb2(b"a" * 1000)
        assert result < 2**64


class TestDjb2Properties:
    """Property-based style tests for DJB2."""

    def test_order_matters(self) -> None:
        # Permutations of the same bytes should (usually) differ
        assert djb2(b"ab") != djb2(b"ba")

    def test_length_matters(self) -> None:
        assert djb2(b"a") != djb2(b"aa")

    def test_case_sensitive(self) -> None:
        assert djb2(b"Hello") != djb2(b"hello")
