"""A closed HTTP/1.0 request-line subset based on RFC 1945 section 5.1."""

from __future__ import annotations

from dataclasses import dataclass

_METHOD_BYTES = frozenset({b"GET", b"HEAD", b"POST"})
_TARGET_PUNCTUATION = frozenset(b"/._-")
_REQUEST_LINE_SUFFIX = b" HTTP/1.0\r\n"


def _valid_target_bytes(target: bytes) -> bool:
    return bool(target) and target[0] == 0x2F and all(
        0x41 <= byte <= 0x5A
        or 0x61 <= byte <= 0x7A
        or 0x30 <= byte <= 0x39
        or byte in _TARGET_PUNCTUATION
        for byte in target
    )


def _encode_argument(value: str) -> bytes:
    try:
        return str.encode(value, "ascii")
    except UnicodeEncodeError as error:
        raise ValueError("HTTP/1.0 request-line values must be ASCII") from error


@dataclass(frozen=True, slots=True)
class Http10RequestLine:
    """Request-line fields; no headers or body are represented."""

    method: str
    target: str


def parse_http10_request_line(wire: bytes) -> Http10RequestLine:
    """Parse exactly one HTTP/1.0 line using the `http-1.0-request-line-1996` subset."""
    if type(wire) is not bytes:
        raise TypeError("wire must be an exact built-in bytes value")
    if not wire.endswith(_REQUEST_LINE_SUFFIX):
        raise ValueError("request line must end with SP HTTP/1.0 CRLF")

    suffix_start = len(wire) - len(_REQUEST_LINE_SUFFIX)
    method_separator = wire.find(b" ", 0, 5)
    if method_separator < 0:
        raise ValueError("HTTP/1.0 request line needs a method SP target")
    method_bytes = wire[:method_separator]
    target_bytes = wire[method_separator + 1 : suffix_start]
    if method_bytes not in _METHOD_BYTES:
        raise ValueError("unsupported HTTP/1.0 request method")
    if not _valid_target_bytes(target_bytes):
        raise ValueError("invalid HTTP/1.0 request target")

    return Http10RequestLine(
        method_bytes.decode("ascii"), target_bytes.decode("ascii")
    )


def encode_http10_request_line(method: str, target: str) -> bytes:
    """Encode validated fields with exact HTTP/1.0 spacing and terminal CRLF."""
    if type(method) is not str or type(target) is not str:
        raise TypeError("method and target must be exact built-in str values")
    method_bytes = _encode_argument(method)
    if method_bytes not in _METHOD_BYTES:
        raise ValueError("unsupported HTTP/1.0 request method")
    target_bytes = _encode_argument(target)
    if not _valid_target_bytes(target_bytes):
        raise ValueError("invalid HTTP/1.0 request target")
    return method_bytes + b" " + target_bytes + _REQUEST_LINE_SUFFIX
