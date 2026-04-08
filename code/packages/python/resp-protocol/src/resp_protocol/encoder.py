"""
RESP2 encoder: convert Python values to RESP wire-format bytes.

Every encoder function returns bytes that are ready to be written to a
TCP socket.  The functions are pure — they have no side effects.

RESP wire format recap:
  Simple String:  +<text>\r\n            (no \r or \n allowed in text)
  Error:          -<message>\r\n
  Integer:        :<number>\r\n
  Bulk String:    $<length>\r\n<bytes>\r\n  or  $-1\r\n  (null)
  Array:          *<count>\r\n<element>...  or  *-1\r\n  (null array)
"""

from __future__ import annotations

from typing import Any

from resp_protocol.types import RespError

# The CRLF terminator.  Every RESP line ends with these two bytes.
# CRLF = carriage return (0x0D) + line feed (0x0A).
_CRLF: bytes = b"\r\n"


def encode_simple_string(s: str) -> bytes:
    """
    Encode a Simple String.

    Simple Strings are used for short, successful replies like "OK", "PONG",
    or "QUEUED".  They cannot contain \\r or \\n — use encode_bulk_string
    for arbitrary text or binary data.

    Wire format: +<text>\\r\\n

    Examples:
        encode_simple_string("OK")    → b"+OK\\r\\n"
        encode_simple_string("PONG")  → b"+PONG\\r\\n"

    Args:
        s: The reply text.  Must not contain \\r or \\n.

    Returns:
        RESP-encoded bytes.

    Raises:
        ValueError: If s contains \\r or \\n.
    """
    if "\r" in s or "\n" in s:
        raise ValueError(
            f"Simple string must not contain \\r or \\n; use encode_bulk_string instead. "
            f"Got: {s!r}"
        )
    return b"+" + s.encode("utf-8") + _CRLF


def encode_error(msg: str) -> bytes:
    """
    Encode a RESP Error reply.

    Errors carry both an error type (e.g. "ERR", "WRONGTYPE") and a
    human-readable message in a single string.  Clients should parse
    the first word as the error class.

    Wire format: -<message>\\r\\n

    Examples:
        encode_error("ERR unknown command")
            → b"-ERR unknown command\\r\\n"
        encode_error("WRONGTYPE value is not a list")
            → b"-WRONGTYPE value is not a list\\r\\n"

    Args:
        msg: The full error message string.

    Returns:
        RESP-encoded bytes.
    """
    return b"-" + msg.encode("utf-8") + _CRLF


def encode_integer(n: int) -> bytes:
    """
    Encode a RESP Integer.

    Integers are used for commands that return counts, indices, or status
    flags: INCR, LLEN, SADD, EXPIRE, etc.  They are signed 64-bit values.

    Wire format: :<number>\\r\\n

    Examples:
        encode_integer(0)   → b":0\\r\\n"
        encode_integer(42)  → b":42\\r\\n"
        encode_integer(-1)  → b":-1\\r\\n"

    Args:
        n: Any Python integer (Redis uses signed 64-bit range).

    Returns:
        RESP-encoded bytes.
    """
    return b":" + str(n).encode("ascii") + _CRLF


def encode_bulk_string(s: bytes | None) -> bytes:
    """
    Encode a RESP Bulk String.

    Bulk Strings are length-prefixed binary-safe strings.  They can hold
    any bytes — including \\r\\n — because the parser reads exactly
    `length` bytes after the header line, not until a newline.

    This is the correct type for SET values, GET replies, field names,
    command arguments, etc.

    Wire format:
        $<length>\\r\\n<bytes>\\r\\n    (non-null)
        $-1\\r\\n                       (null — key does not exist)

    Examples:
        encode_bulk_string(b"foobar")     → b"$6\\r\\nfoobar\\r\\n"
        encode_bulk_string(b"")           → b"$0\\r\\n\\r\\n"
        encode_bulk_string(None)          → b"$-1\\r\\n"
        encode_bulk_string(b"foo\\r\\nbar") → b"$7\\r\\nfoo\\r\\nbar\\r\\n"

    Args:
        s: The bytes to encode, or None for a null bulk string.

    Returns:
        RESP-encoded bytes.
    """
    if s is None:
        # Null bulk string signals "key not found" (e.g. GET of missing key).
        return b"$-1" + _CRLF
    header = b"$" + str(len(s)).encode("ascii") + _CRLF
    return header + s + _CRLF


def encode_array(items: list[Any] | None) -> bytes:
    """
    Encode a RESP Array.

    Arrays are used for multi-bulk replies (LRANGE, SMEMBERS, HGETALL)
    and for sending commands from the client (every command is an array
    of bulk strings).

    Each element is encoded recursively by calling `encode()`, so arrays
    can contain integers, bulk strings, nested arrays, errors, etc.

    Wire format:
        *<count>\\r\\n<element>...   (non-null)
        *-1\\r\\n                    (null array)
        *0\\r\\n                     (empty array)

    Examples:
        encode_array(None)         → b"*-1\\r\\n"
        encode_array([])           → b"*0\\r\\n"
        encode_array([b"foo", b"bar"]) → b"*2\\r\\n$3\\r\\nfoo\\r\\n$3\\r\\nbar\\r\\n"

    Args:
        items: A list of Python values to encode, or None for null array.

    Returns:
        RESP-encoded bytes.
    """
    if items is None:
        return b"*-1" + _CRLF
    header = b"*" + str(len(items)).encode("ascii") + _CRLF
    # Encode each element recursively and concatenate all the parts.
    body = b"".join(encode(item) for item in items)
    return header + body


def encode(value: Any) -> bytes:
    """
    High-level encoder: dispatch to the appropriate RESP type based on
    the Python type of the value.

    Type mapping:
        None       → Null Bulk String  ($-1\\r\\n)
        bool       → Integer           (:1\\r\\n or :0\\r\\n)
        int        → Integer           (:<n>\\r\\n)
        str        → Bulk String       ($<len>\\r\\n<utf-8>\\r\\n)
        bytes      → Bulk String       ($<len>\\r\\n<bytes>\\r\\n)
        list       → Array             (*<n>\\r\\n<elements>)
        RespError  → Error             (-<msg>\\r\\n)

    Note: bool is checked before int because bool is a subclass of int
    in Python — isinstance(True, int) is True.

    Args:
        value: Any Python value supported by RESP2.

    Returns:
        RESP-encoded bytes.

    Raises:
        TypeError: If the value's type has no RESP mapping.
    """
    if value is None:
        return encode_bulk_string(None)
    if isinstance(value, bool):
        # bool subclasses int, so we must check it first.
        # True  → :1\r\n
        # False → :0\r\n
        return encode_integer(1 if value else 0)
    if isinstance(value, int):
        return encode_integer(value)
    if isinstance(value, str):
        # Strings are encoded as bulk strings (UTF-8) so they are binary-safe.
        return encode_bulk_string(value.encode("utf-8"))
    if isinstance(value, bytes):
        return encode_bulk_string(value)
    if isinstance(value, list):
        return encode_array(value)
    if isinstance(value, RespError):
        return encode_error(value.message)
    raise TypeError(
        f"Cannot encode value of type {type(value).__name__!r}: {value!r}"
    )
