"""
Tests for the RESP2 encoder functions.
"""

import pytest

from resp_protocol import (
    RespError,
    encode,
    encode_array,
    encode_bulk_string,
    encode_error,
    encode_integer,
    encode_simple_string,
)


class TestEncodeSimpleString:
    def test_ok(self) -> None:
        assert encode_simple_string("OK") == b"+OK\r\n"

    def test_pong(self) -> None:
        assert encode_simple_string("PONG") == b"+PONG\r\n"

    def test_queued(self) -> None:
        assert encode_simple_string("QUEUED") == b"+QUEUED\r\n"

    def test_empty(self) -> None:
        assert encode_simple_string("") == b"+\r\n"

    def test_rejects_carriage_return(self) -> None:
        with pytest.raises(ValueError):
            encode_simple_string("foo\rbar")

    def test_rejects_newline(self) -> None:
        with pytest.raises(ValueError):
            encode_simple_string("foo\nbar")

    def test_unicode(self) -> None:
        result = encode_simple_string("caf\u00e9")
        assert result == b"+caf\xc3\xa9\r\n"


class TestEncodeError:
    def test_err(self) -> None:
        assert encode_error("ERR unknown command") == b"-ERR unknown command\r\n"

    def test_wrongtype(self) -> None:
        msg = "WRONGTYPE Operation against wrong type"
        assert encode_error(msg) == b"-WRONGTYPE Operation against wrong type\r\n"

    def test_noscript(self) -> None:
        assert encode_error("NOSCRIPT No matching script") == b"-NOSCRIPT No matching script\r\n"

    def test_empty_message(self) -> None:
        assert encode_error("") == b"-\r\n"


class TestEncodeInteger:
    def test_zero(self) -> None:
        assert encode_integer(0) == b":0\r\n"

    def test_positive(self) -> None:
        assert encode_integer(42) == b":42\r\n"

    def test_negative(self) -> None:
        assert encode_integer(-1) == b":-1\r\n"

    def test_large(self) -> None:
        assert encode_integer(1_000_000) == b":1000000\r\n"

    def test_very_large(self) -> None:
        result = encode_integer(2**62)
        assert result == b":" + str(2**62).encode() + b"\r\n"


class TestEncodeBulkString:
    def test_foobar(self) -> None:
        assert encode_bulk_string(b"foobar") == b"$6\r\nfoobar\r\n"

    def test_empty(self) -> None:
        assert encode_bulk_string(b"") == b"$0\r\n\r\n"

    def test_null(self) -> None:
        assert encode_bulk_string(None) == b"$-1\r\n"

    def test_binary_with_crlf(self) -> None:
        # \r\n inside the data is fine because we use length framing
        data = b"foo\r\nbar"
        assert encode_bulk_string(data) == b"$8\r\nfoo\r\nbar\r\n"

    def test_null_byte(self) -> None:
        assert encode_bulk_string(b"\x00") == b"$1\r\n\x00\r\n"

    def test_all_bytes(self) -> None:
        data = bytes(range(256))
        result = encode_bulk_string(data)
        assert result.startswith(b"$256\r\n")
        assert result.endswith(b"\r\n")
        assert len(result) == len("$256\r\n") + 256 + 2


class TestEncodeArray:
    def test_null_array(self) -> None:
        assert encode_array(None) == b"*-1\r\n"

    def test_empty_array(self) -> None:
        assert encode_array([]) == b"*0\r\n"

    def test_two_bulk_strings(self) -> None:
        result = encode_array([b"foo", b"bar"])
        assert result == b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"

    def test_set_foo_bar(self) -> None:
        # A Redis SET command is an array of 3 bulk strings
        result = encode_array([b"SET", b"foo", b"bar"])
        assert result == b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"

    def test_array_with_integers(self) -> None:
        result = encode_array([1, 2, 3])
        assert result == b"*3\r\n:1\r\n:2\r\n:3\r\n"

    def test_nested_array(self) -> None:
        result = encode_array([[b"foo"], [b"bar"]])
        assert result == b"*2\r\n*1\r\n$3\r\nfoo\r\n*1\r\n$3\r\nbar\r\n"

    def test_mixed_types(self) -> None:
        result = encode_array([1, b"hello", None])
        assert result == b"*3\r\n:1\r\n$5\r\nhello\r\n$-1\r\n"


class TestEncode:
    """High-level encode() dispatcher."""

    def test_none_is_null_bulk(self) -> None:
        assert encode(None) == b"$-1\r\n"

    def test_int(self) -> None:
        assert encode(42) == b":42\r\n"

    def test_str(self) -> None:
        assert encode("hello") == b"$5\r\nhello\r\n"

    def test_bytes(self) -> None:
        assert encode(b"hello") == b"$5\r\nhello\r\n"

    def test_list(self) -> None:
        assert encode([b"foo"]) == b"*1\r\n$3\r\nfoo\r\n"

    def test_resp_error(self) -> None:
        err = RespError("ERR something went wrong")
        assert encode(err) == b"-ERR something went wrong\r\n"

    def test_bool_true(self) -> None:
        assert encode(True) == b":1\r\n"

    def test_bool_false(self) -> None:
        assert encode(False) == b":0\r\n"

    def test_negative_int(self) -> None:
        assert encode(-5) == b":-5\r\n"

    def test_unknown_type_raises(self) -> None:
        with pytest.raises(TypeError):
            encode(3.14)

    def test_unknown_type_dict_raises(self) -> None:
        with pytest.raises(TypeError):
            encode({"key": "value"})

    def test_str_unicode(self) -> None:
        result = encode("caf\u00e9")
        assert result == b"$5\r\ncaf\xc3\xa9\r\n"

    def test_empty_list(self) -> None:
        assert encode([]) == b"*0\r\n"
