"""intel_8008_packager — Intel HEX ROM image encoder/decoder for the Intel 8008.

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

Intel 8008 Address Space
-------------------------

The Intel 8008 has a 14-bit address space (16 KB = 0x0000–0x3FFF):

  0x0000–0x1FFF   ROM: program code (8 KB)
  0x2000–0x3FFF   RAM: static variable data (8 KB)

The packager handles the full 16 KB range.  Unused ROM bytes should be
padded to 0xFF (erased flash state) before packaging; this is handled by
the assembler when a forward ORG directive is used.

Quick Start
-----------

::

    from intel_8008_packager import encode_hex, decode_hex

    # Binary → Intel HEX (e.g. for an EPROM programmer or simulator)
    binary = bytes([0x06, 0x00, 0xFF])   # MVI B, 0; HLT
    hex_text = encode_hex(binary)

    # Intel HEX → binary (round-trip verification)
    origin, recovered = decode_hex(hex_text)
    assert recovered == binary

Relationship to Pipeline
-------------------------

This package is the fifth and final stage of the Oct → Intel 8008 pipeline:

    intel-8008-assembler  →  binary bytes
    intel-8008-packager   →  Intel HEX file (.hex)
    intel8008-simulator   reads the .hex file directly

Public API
----------

- ``encode_hex(binary, origin=0) -> str``  — binary → Intel HEX string
- ``decode_hex(hex_text) -> (origin, bytes)``  — Intel HEX string → binary
"""

from __future__ import annotations

from intel_8008_packager.hex_encoder import decode_hex, encode_hex

__all__ = [
    "decode_hex",
    "encode_hex",
]
