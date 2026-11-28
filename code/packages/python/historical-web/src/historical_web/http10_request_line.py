"""A closed HTTP/1.0 request-line subset based on RFC 1945 section 5.1."""

from __future__ import annotations

from dataclasses import dataclass

_METHODS = frozenset({"GET", "HEAD", "POST"})
_TARGET_PUNCTUATION = frozenset(b"/._-")


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
    """Validated request-line fields; no headers or body are represented."""

    method: str
    target: str

    def __post_init__(self) -> None:
        if type(self.method) is not str or type(self.target) is not str:
            raise TypeError("method and target must be exact built-in str values")
        if self.method not in _METHODS:
            raise ValueError("unsupported HTTP/1.0 request method")
        if not _valid_target_bytes(_encode_argument(self.target)):
            raise ValueError("invalid HTTP/1.0 request target")


def parse_http10_request_line(wire: bytes) -> Http10RequestLine:
    """Parse exactly one HTTP/1.0 line using the `http-1.0-request-line-1996` subset."""
    if type(wire) is not bytes:
        raise TypeError("wire must be an exact built-in bytes value")
    if not wire.endswith(b"\r\n"):
        raise ValueError("HTTP/1.0 request line must end with CRLF")

    line = wire[:-2]
    first_space = line.find(b" ")
    second_space = line.find(b" ", first_space + 1)
    if first_space < 0 or second_space < 0:
        raise ValueError("HTTP/1.0 request line needs two SP separators")
    if line.find(b" ", second_space + 1) >= 0:
        raise ValueError("HTTP/1.0 request line uses exactly two SP separators")

    method_bytes = line[:first_space]
    target_bytes = line[first_space + 1 : second_space]
    version = line[second_space + 1 :]
    if method_bytes not in (b"GET", b"HEAD", b"POST"):
        raise ValueError("unsupported HTTP/1.0 request method")
    if not _valid_target_bytes(target_bytes):
        raise ValueError("invalid HTTP/1.0 request target")
    if version != b"HTTP/1.0":
        raise ValueError("request line must use HTTP/1.0")

    return Http10RequestLine(
        method_bytes.decode("ascii"), target_bytes.decode("ascii")
    )


def encode_http10_request_line(method: str, target: str) -> bytes:
    """Encode validated fields with exact HTTP/1.0 spacing and terminal CRLF."""
    if type(method) is not str or type(target) is not str:
        raise TypeError("method and target must be exact built-in str values")
    method_bytes = _encode_argument(method)
    target_bytes = _encode_argument(target)
    if method not in _METHODS:
        raise ValueError("unsupported HTTP/1.0 request method")
    if not _valid_target_bytes(target_bytes):
        raise ValueError("invalid HTTP/1.0 request target")
    return method_bytes + b" " + target_bytes + b" HTTP/1.0\r\n"
