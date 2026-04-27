"""Tests for the HTTP/1 head parser."""

from http_core import BodyKind, HttpVersion
from http1 import (
    __version__,
    Http1ParseError,
    parse_request_head,
    parse_response_head,
)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_parse_simple_request() -> None:
    parsed = parse_request_head(b"GET / HTTP/1.0\r\nHost: example.com\r\n\r\n")
    assert parsed.head.method == "GET"
    assert parsed.head.target == "/"
    assert parsed.head.version == HttpVersion(1, 0)
    assert parsed.body_kind == BodyKind.none()


def test_parse_post_request_with_content_length() -> None:
    parsed = parse_request_head(
        b"POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello"
    )
    assert parsed.body_kind == BodyKind.content_length(5)


def test_parse_response_head() -> None:
    parsed = parse_response_head(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody")
    assert parsed.head.status == 200
    assert parsed.head.reason == "OK"
    assert parsed.body_kind == BodyKind.content_length(4)


def test_response_without_length_uses_until_eof() -> None:
    parsed = parse_response_head(b"HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n")
    assert parsed.body_kind == BodyKind.until_eof()


def test_bodyless_status_codes_ignore_body_headers() -> None:
    parsed = parse_response_head(
        b"HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n"
    )
    assert parsed.body_kind == BodyKind.none()


def test_accepts_lf_only_and_preserves_duplicate_headers() -> None:
    parsed = parse_response_head(
        b"\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload"
    )
    assert [header.value for header in parsed.head.headers] == ["a=1", "b=2"]


def test_invalid_header_raises() -> None:
    try:
        parse_request_head(b"GET / HTTP/1.1\r\nHost example.com\r\n\r\n")
    except Http1ParseError as error:
        assert "invalid HTTP/1 header" in str(error)
    else:
        raise AssertionError("expected Http1ParseError")


def test_invalid_content_length_raises() -> None:
    try:
        parse_response_head(b"HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n")
    except Http1ParseError as error:
        assert "invalid Content-Length" in str(error)
    else:
        raise AssertionError("expected Http1ParseError")
