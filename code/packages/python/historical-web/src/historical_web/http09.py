"""The 1991 simple GET exchange, as described in the CERN HTTP archive."""


def encode_http09_request(address: str) -> bytes:
    """Format an ASCII document address with the historical CRLF terminator."""
    try:
        encoded = address.encode("ascii")
    except UnicodeEncodeError as error:
        raise ValueError("invalid HTTP/0.9 document address") from error
    if not encoded or any(byte < 0x21 or byte > 0x7E for byte in encoded):
        raise ValueError("invalid HTTP/0.9 document address")
    return b"GET " + encoded + b"\r\n"


def parse_http09_request(wire: bytes) -> str:
    """Read exactly one simple request line. A bare LF is also accepted."""
    prefix = b"GET "
    if not wire.startswith(prefix) or len(wire) < len(prefix) + 2:
        raise ValueError("invalid HTTP/0.9 request line")
    address_end = len(wire) - 1
    if wire[-1:] != b"\n":
        raise ValueError("invalid HTTP/0.9 request line")
    if address_end and wire[address_end - 1 : address_end] == b"\r":
        address_end -= 1
    address = wire[len(prefix) : address_end]
    if not address or any(byte < 0x21 or byte > 0x7E for byte in address):
        raise ValueError("invalid HTTP/0.9 request line")
    return address.decode("ascii")


def decode_http09_response(body: bytes) -> str:
    """Decode the completed response after the server closes the connection."""
    # Every byte belongs to the body. EOF, rather than a length field,
    # tells the caller when to hand over the response.
    return body.decode("ascii")
