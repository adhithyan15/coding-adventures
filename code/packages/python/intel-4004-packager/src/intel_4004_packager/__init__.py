"""intel_4004_packager — Intel HEX ROM image encoder/decoder.

Converts raw binary machine code into the Intel HEX format used by EPROM
programmers, and parses Intel HEX back to binary for round-trip verification.

Intel HEX is the industry-standard format for programming ROM chips.  Each
record encodes a chunk of binary data at a specific memory address:

    :LLAAAATTDD...CC
    LL   = byte count (1 byte)
    AAAA = start address (2 bytes, big-endian)
    TT   = record type (0x00 = data, 0x01 = EOF)
    DD   = data bytes
    CC   = checksum: two's complement of sum of all preceding bytes

Quick Start
-----------

::

    from intel_4004_packager import encode_hex, decode_hex

    # Binary → Intel HEX (e.g. for an EPROM programmer)
    binary = bytes([0xD7, 0x01, 0xC0])   # LDM 7; HLT; BBL 0
    hex_text = encode_hex(binary)

    # Intel HEX → binary (round-trip verification)
    origin, recovered = decode_hex(hex_text)
    assert recovered == binary

Public API
----------

- ``encode_hex(binary, origin=0) -> str``  — binary → Intel HEX string
- ``decode_hex(hex_text) -> (origin, bytes)``  — Intel HEX string → binary
"""

from __future__ import annotations

from intel_4004_packager.hex_encoder import decode_hex, encode_hex

__all__ = [
    "decode_hex",
    "encode_hex",
]
