"""
resp_protocol — RESP2 (Redis Serialization Protocol) encoder and decoder.

RESP is the wire format used by Redis for every byte that flows between a
client and a server.  It was designed to be:
  - Simple: a parser fits in about 100 lines of code
  - Fast: binary-safe, length-prefixed, no escaping required
  - Human-readable: you can telnet into Redis and type commands

This module implements the full RESP2 specification:
  + Simple Strings    (+OK\\r\\n)
  - Errors            (-ERR message\\r\\n)
  : Integers          (:42\\r\\n)
  $ Bulk Strings      ($6\\r\\nfoobar\\r\\n), null ($-1\\r\\n)
  * Arrays            (*2\\r\\n...), null (*-1\\r\\n)

Quick start:

    >>> from resp_protocol import encode, decode, RespError, RespDecoder
    >>> encode(["SET", "key", "value"])
    b'*3\\r\\n$3\\r\\nSET\\r\\n$3\\r\\nkey\\r\\n$5\\r\\nvalue\\r\\n'
    >>> decode(b'+OK\\r\\n')
    ('OK', 5)
    >>> decode(b':42\\r\\n')
    (42, 5)
    >>> decode(b'$-1\\r\\n')
    (None, 5)

Streaming use (TCP framing):

    >>> dec = RespDecoder()
    >>> dec.feed(b'*2\\r\\n$3\\r\\nfoo\\r\\n')
    >>> dec.has_message()
    False
    >>> dec.feed(b'$3\\r\\nbar\\r\\n')
    >>> dec.has_message()
    True
    >>> dec.get_message()
    [b'foo', b'bar']
"""

from resp_protocol.decoder import RespDecodeError, RespDecoder, decode, decode_all
from resp_protocol.encoder import (
    encode,
    encode_array,
    encode_bulk_string,
    encode_error,
    encode_integer,
    encode_simple_string,
)
from resp_protocol.types import RespError, RespValue

__all__ = [
    # Types
    "RespError",
    "RespValue",
    # Encoders
    "encode_simple_string",
    "encode_error",
    "encode_integer",
    "encode_bulk_string",
    "encode_array",
    "encode",
    # Decoders
    "decode",
    "decode_all",
    "RespDecoder",
    "RespDecodeError",
]
