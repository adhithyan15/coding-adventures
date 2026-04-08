"""
Round-trip tests: encode(x) → decode → re-encode, verifying encode(x) == re-encode(decode(encode(x))).

This also serves as integration tests proving the encoder and decoder
are consistent with each other.
"""

import pytest

from resp_protocol import RespError, decode, encode
from resp_protocol.encoder import encode_array, encode_bulk_string, encode_integer, encode_simple_string


class TestRoundTrip:
    """encode then decode should reproduce the original wire bytes."""

    def _round_trip(self, value: object) -> None:
        encoded = encode(value)
        decoded, consumed = decode(encoded)
        assert consumed == len(encoded), f"Consumed {consumed} of {len(encoded)} bytes"
        re_encoded = encode(decoded)
        assert re_encoded == encoded, (
            f"Round trip failed for {value!r}: "
            f"original={encoded!r} re-encoded={re_encoded!r}"
        )

    def test_none(self) -> None:
        self._round_trip(None)

    def test_empty_str(self) -> None:
        self._round_trip("")

    def test_hello_str(self) -> None:
        self._round_trip("hello")

    def test_hello_world_str(self) -> None:
        self._round_trip("hello world")

    def test_empty_bytes(self) -> None:
        self._round_trip(b"")

    def test_binary_bytes(self) -> None:
        self._round_trip(b"binary\x00data")

    def test_crlf_in_bytes(self) -> None:
        self._round_trip(b"has\r\nnewlines")

    def test_all_bytes(self) -> None:
        self._round_trip(bytes(range(256)))

    def test_zero(self) -> None:
        self._round_trip(0)

    def test_forty_two(self) -> None:
        self._round_trip(42)

    def test_negative_one(self) -> None:
        self._round_trip(-1)

    def test_empty_list(self) -> None:
        self._round_trip([])

    def test_string_list(self) -> None:
        # Strings in lists get encoded as bulk strings and decoded as bytes
        encoded = encode([b"SET", b"foo", b"bar"])
        decoded, consumed = decode(encoded)
        assert decoded == [b"SET", b"foo", b"bar"]
        assert consumed == len(encoded)

    def test_mixed_list(self) -> None:
        encoded = encode([1, None, b"ok", b"bytes"])
        decoded, consumed = decode(encoded)
        assert decoded == [1, None, b"ok", b"bytes"]
        assert consumed == len(encoded)

    def test_nested_list(self) -> None:
        encoded = encode([[b"foo"], [b"bar"]])
        decoded, consumed = decode(encoded)
        assert decoded == [[b"foo"], [b"bar"]]
        assert consumed == len(encoded)

    def test_resp_error(self) -> None:
        err = RespError("ERR something broke")
        encoded = encode(err)
        decoded, consumed = decode(encoded)
        assert isinstance(decoded, RespError)
        assert decoded.message == "ERR something broke"
        assert consumed == len(encoded)


class TestSimpleStringRoundTrip:
    """Simple strings need encode_simple_string since encode(str) uses bulk strings."""

    def test_ok(self) -> None:
        encoded = encode_simple_string("OK")
        decoded, consumed = decode(encoded)
        assert decoded == "OK"
        assert consumed == len(encoded)

    def test_pong(self) -> None:
        encoded = encode_simple_string("PONG")
        decoded, consumed = decode(encoded)
        assert decoded == "PONG"
        assert consumed == len(encoded)


class TestIntegerRoundTrip:
    def test_many_integers(self) -> None:
        for n in [-1000, -1, 0, 1, 42, 1_000_000]:
            encoded = encode_integer(n)
            decoded, consumed = decode(encoded)
            assert decoded == n
            assert consumed == len(encoded)


class TestBulkStringRoundTrip:
    def test_null(self) -> None:
        encoded = encode_bulk_string(None)
        decoded, consumed = decode(encoded)
        assert decoded is None
        assert consumed == len(encoded)

    def test_non_null(self) -> None:
        for data in [b"", b"hello", b"foo\r\nbar", bytes(range(256))]:
            encoded = encode_bulk_string(data)
            decoded, consumed = decode(encoded)
            assert decoded == data
            assert consumed == len(encoded)


class TestArrayRoundTrip:
    def test_null_array(self) -> None:
        encoded = encode_array(None)
        decoded, consumed = decode(encoded)
        assert decoded is None
        assert consumed == len(encoded)

    def test_empty_array(self) -> None:
        encoded = encode_array([])
        decoded, consumed = decode(encoded)
        assert decoded == []
        assert consumed == len(encoded)
