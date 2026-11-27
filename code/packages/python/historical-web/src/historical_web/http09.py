"""The 1991 simple GET exchange, as described in the CERN HTTP archive."""

import re

_ADDRESS = re.compile(rb"[!-~]+")
_REQUEST = re.compile(rb"GET ([!-~]+)\r?\n")


def encode_http09_request(address: str) -> bytes:
    """Format an ASCII document address with the historical CRLF terminator."""
    try:
        encoded = address.encode("ascii")
    except UnicodeEncodeError as error:
        raise ValueError("invalid HTTP/0.9 document address") from error
    if not _ADDRESS.fullmatch(encoded):
        raise ValueError("invalid HTTP/0.9 document address")
    return b"GET " + encoded + b"\r\n"


def parse_http09_request(wire: bytes) -> str:
    """Read exactly one simple request line. A bare LF is also accepted."""
    match = _REQUEST.fullmatch(wire)
    if match is None:
        raise ValueError("invalid HTTP/0.9 request line")
    return match[1].decode("ascii")


def decode_http09_response(body: bytes) -> str:
    """Decode the completed response after the server closes the connection."""
    # Every byte belongs to the body. EOF, rather than a length field,
    # tells the caller when to hand over the response.
    return body.decode("ascii")
