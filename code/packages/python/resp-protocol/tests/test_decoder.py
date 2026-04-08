"""
Tests for the RESP2 decoder: decode(), decode_all(), and RespDecoder.
"""

import pytest

from resp_protocol import RespDecodeError, RespDecoder, RespError, decode, decode_all
from resp_protocol.encoder import encode_array, encode_bulk_string


class TestDecodeSimpleString:
    def test_ok(self) -> None:
        value, consumed = decode(b"+OK\r\n")
        assert value == "OK"
        assert consumed == 5

    def test_pong(self) -> None:
        value, consumed = decode(b"+PONG\r\n")
        assert value == "PONG"
        assert consumed == 7

    def test_empty_simple_string(self) -> None:
        value, consumed = decode(b"+\r\n")
        assert value == ""
        assert consumed == 3

    def test_incomplete_simple_string(self) -> None:
        value, consumed = decode(b"+OK")
        assert value is None
        assert consumed == 0

    def test_extra_bytes_not_consumed(self) -> None:
        value, consumed = decode(b"+OK\r\n+NEXT\r\n")
        assert value == "OK"
        assert consumed == 5


class TestDecodeError:
    def test_err_message(self) -> None:
        raw = b"-ERR unknown command\r\n"
        value, consumed = decode(raw)
        assert isinstance(value, RespError)
        assert value.message == "ERR unknown command"
        assert value.error_type == "ERR"
        assert value.detail == "unknown command"
        assert consumed == len(raw)

    def test_wrongtype(self) -> None:
        raw = b"-WRONGTYPE Operation against wrong type\r\n"
        value, consumed = decode(raw)
        assert isinstance(value, RespError)
        assert value.error_type == "WRONGTYPE"
        assert consumed == len(raw)

    def test_incomplete_error(self) -> None:
        value, consumed = decode(b"-ERR")
        assert value is None
        assert consumed == 0

    def test_error_no_detail(self) -> None:
        raw = b"-ERR\r\n"
        value, consumed = decode(raw)
        assert isinstance(value, RespError)
        assert value.error_type == "ERR"
        assert value.detail == ""


class TestDecodeInteger:
    def test_zero(self) -> None:
        value, consumed = decode(b":0\r\n")
        assert value == 0
        assert consumed == 4

    def test_positive(self) -> None:
        value, consumed = decode(b":42\r\n")
        assert value == 42
        assert consumed == 5

    def test_negative(self) -> None:
        value, consumed = decode(b":-1\r\n")
        assert value == -1
        assert consumed == 5

    def test_large_number(self) -> None:
        value, consumed = decode(b":1000000\r\n")
        assert value == 1_000_000

    def test_incomplete(self) -> None:
        value, consumed = decode(b":42")
        assert value is None
        assert consumed == 0

    def test_invalid_integer(self) -> None:
        with pytest.raises(RespDecodeError):
            decode(b":abc\r\n")


class TestDecodeBulkString:
    def test_foobar(self) -> None:
        value, consumed = decode(b"$6\r\nfoobar\r\n")
        assert value == b"foobar"
        assert consumed == 12

    def test_empty(self) -> None:
        value, consumed = decode(b"$0\r\n\r\n")
        assert value == b""
        assert consumed == 6

    def test_null(self) -> None:
        value, consumed = decode(b"$-1\r\n")
        assert value is None
        assert consumed == 5

    def test_binary_with_crlf_inside(self) -> None:
        # The \r\n inside the data must not confuse the parser
        payload = b"foo\r\nbar"
        encoded = encode_bulk_string(payload)
        value, consumed = decode(encoded)
        assert value == payload
        assert consumed == len(encoded)

    def test_all_256_bytes(self) -> None:
        payload = bytes(range(256))
        encoded = encode_bulk_string(payload)
        value, consumed = decode(encoded)
        assert value == payload

    def test_incomplete_header(self) -> None:
        value, consumed = decode(b"$6")
        assert value is None
        assert consumed == 0

    def test_incomplete_body(self) -> None:
        value, consumed = decode(b"$6\r\nfoo")
        assert value is None
        assert consumed == 0

    def test_invalid_length(self) -> None:
        with pytest.raises(RespDecodeError):
            decode(b"$abc\r\n")

    def test_negative_invalid_length(self) -> None:
        with pytest.raises(RespDecodeError):
            decode(b"$-5\r\n")

    def test_missing_trailing_crlf(self) -> None:
        # Manually craft a corrupt bulk string with wrong trailing bytes
        # to exercise the guard on line 168 of decoder.py
        with pytest.raises(RespDecodeError):
            decode(b"$3\r\nfooXX")


class TestDecodeArray:
    def test_empty_array(self) -> None:
        value, consumed = decode(b"*0\r\n")
        assert value == []
        assert consumed == 4

    def test_null_array(self) -> None:
        value, consumed = decode(b"*-1\r\n")
        assert value is None
        assert consumed == 5

    def test_two_integers(self) -> None:
        value, consumed = decode(b"*2\r\n:1\r\n:2\r\n")
        assert value == [1, 2]
        assert consumed == 12

    def test_set_foo_bar(self) -> None:
        raw = b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"
        value, consumed = decode(raw)
        assert value == [b"SET", b"foo", b"bar"]
        assert consumed == len(raw)

    def test_nested_array(self) -> None:
        # [[1, 2], [3, 4]]
        raw = b"*2\r\n*2\r\n:1\r\n:2\r\n*2\r\n:3\r\n:4\r\n"
        value, consumed = decode(raw)
        assert value == [[1, 2], [3, 4]]
        assert consumed == len(raw)

    def test_array_with_null_element(self) -> None:
        raw = b"*3\r\n:1\r\n$-1\r\n:3\r\n"
        value, consumed = decode(raw)
        assert value == [1, None, 3]

    def test_incomplete_array_header(self) -> None:
        value, consumed = decode(b"*2")
        assert value is None
        assert consumed == 0

    def test_incomplete_array_element(self) -> None:
        # Array header is complete but one element is missing
        value, consumed = decode(b"*2\r\n:1\r\n")
        assert value is None
        assert consumed == 0

    def test_invalid_count(self) -> None:
        with pytest.raises(RespDecodeError):
            decode(b"*abc\r\n")

    def test_negative_invalid_count(self) -> None:
        with pytest.raises(RespDecodeError):
            decode(b"*-5\r\n")


class TestDecodeInlineCommand:
    """RESP inline commands (plain text, e.g. from telnet)."""

    def test_ping_inline(self) -> None:
        value, consumed = decode(b"PING\r\n")
        assert value == [b"PING"]
        assert consumed == 6

    def test_set_inline(self) -> None:
        value, consumed = decode(b"SET foo bar\r\n")
        assert value == [b"SET", b"foo", b"bar"]
        assert consumed == 13

    def test_inline_incomplete(self) -> None:
        value, consumed = decode(b"PING")
        assert value is None
        assert consumed == 0

    def test_inline_empty_line(self) -> None:
        # An empty line (just \r\n) produces an empty array
        value, consumed = decode(b"\r\n")
        assert value == []
        assert consumed == 2


class TestDecodeEmpty:
    def test_empty_buffer(self) -> None:
        value, consumed = decode(b"")
        assert value is None
        assert consumed == 0


class TestDecodeAll:
    def test_single_message(self) -> None:
        messages, consumed = decode_all(b"+OK\r\n")
        assert messages == ["OK"]
        assert consumed == 5

    def test_two_messages(self) -> None:
        messages, consumed = decode_all(b"+OK\r\n:42\r\n")
        assert messages == ["OK", 42]
        assert consumed == 10

    def test_incomplete_tail(self) -> None:
        messages, consumed = decode_all(b"+OK\r\n+PARTIAL")
        assert messages == ["OK"]
        assert consumed == 5

    def test_empty_buffer(self) -> None:
        messages, consumed = decode_all(b"")
        assert messages == []
        assert consumed == 0

    def test_three_commands(self) -> None:
        from resp_protocol import encode
        full = (
            encode([b"SET", b"k", b"v"])
            + encode([b"GET", b"k"])
            + encode([b"DEL", b"k"])
        )
        messages, consumed = decode_all(full)
        assert len(messages) == 3
        assert messages[0] == [b"SET", b"k", b"v"]
        assert messages[1] == [b"GET", b"k"]
        assert messages[2] == [b"DEL", b"k"]
        assert consumed == len(full)


class TestRespDecoder:
    """Tests for the stateful streaming RespDecoder class."""

    def test_single_feed(self) -> None:
        dec = RespDecoder()
        dec.feed(b"+OK\r\n")
        assert dec.has_message()
        assert dec.get_message() == "OK"

    def test_fragmented_feed(self) -> None:
        dec = RespDecoder()
        dec.feed(b"+")
        assert not dec.has_message()
        dec.feed(b"OK")
        assert not dec.has_message()
        dec.feed(b"\r\n")
        assert dec.has_message()
        assert dec.get_message() == "OK"

    def test_bulk_string_fragmented(self) -> None:
        dec = RespDecoder()
        dec.feed(b"$6\r\nfoo")
        assert not dec.has_message()
        dec.feed(b"bar\r\n")
        assert dec.has_message()
        assert dec.get_message() == b"foobar"

    def test_get_message_raises_when_empty(self) -> None:
        dec = RespDecoder()
        with pytest.raises(ValueError):
            dec.get_message()

    def test_multiple_messages_in_one_feed(self) -> None:
        dec = RespDecoder()
        dec.feed(b"+OK\r\n:42\r\n-ERR bad\r\n")
        assert dec.get_message() == "OK"
        assert dec.get_message() == 42
        msg = dec.get_message()
        assert isinstance(msg, RespError)
        assert msg.message == "ERR bad"
        assert not dec.has_message()

    def test_decode_all_convenience(self) -> None:
        dec = RespDecoder()
        messages = dec.decode_all(b"+OK\r\n:1\r\n")
        assert messages == ["OK", 1]

    def test_decode_all_accumulates_partial(self) -> None:
        dec = RespDecoder()
        m1 = dec.decode_all(b"+OK\r\n+PARTIAL")
        assert m1 == ["OK"]
        m2 = dec.decode_all(b"\r\n")
        assert m2 == ["PARTIAL"]

    def test_streaming_byte_by_byte(self) -> None:
        """Simulate worst-case TCP fragmentation: one byte at a time."""
        from resp_protocol import encode
        messages_to_send = [
            encode([b"SET", b"k", b"v"]),
            encode([b"GET", b"k"]),
            encode([b"DEL", b"k"]),
        ]
        full_stream = b"".join(messages_to_send)

        dec = RespDecoder()
        received: list = []
        for byte_val in full_stream:
            dec.feed(bytes([byte_val]))
            while dec.has_message():
                received.append(dec.get_message())

        assert len(received) == 3
        assert received[0] == [b"SET", b"k", b"v"]
        assert received[1] == [b"GET", b"k"]
        assert received[2] == [b"DEL", b"k"]


class TestIncompleteParsing:
    """Every prefix of a full message must return (None, 0)."""

    def test_set_command_all_prefixes(self) -> None:
        full = b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"
        for i in range(1, len(full)):
            partial = full[:i]
            value, consumed = decode(partial)
            assert value is None and consumed == 0, (
                f"Expected incomplete at {i} bytes, got value={value!r}"
            )
        # Full message must parse correctly
        value, consumed = decode(full)
        assert value == [b"SET", b"foo", b"bar"]
        assert consumed == len(full)
