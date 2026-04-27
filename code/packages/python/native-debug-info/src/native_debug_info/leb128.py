"""LEB128 variable-length integer encoding used throughout DWARF.

ULEB128 encodes unsigned integers; SLEB128 encodes signed integers.
Both use 7 bits per byte, with the high bit set on all bytes except the last.

Example — ULEB128(624485):
  624485 = 0x98765
  Bytes: 0xe5, 0x8e, 0x26  (3 bytes)

Example — SLEB128(-123456):
  Bytes: 0xc0, 0xbb, 0x78  (3 bytes)
"""

from __future__ import annotations


def encode_uleb128(value: int) -> bytes:
    """Encode a non-negative integer as ULEB128."""
    if value < 0:
        raise ValueError(f"encode_uleb128 requires a non-negative value, got {value}")
    result = []
    while True:
        byte = value & 0x7F
        value >>= 7
        if value != 0:
            byte |= 0x80  # more bytes follow
        result.append(byte)
        if value == 0:
            break
    return bytes(result)


def encode_sleb128(value: int) -> bytes:
    """Encode a signed integer as SLEB128."""
    result = []
    more = True
    while more:
        byte = value & 0x7F
        value >>= 7
        # Sign-extend: if the sign bit of byte is set but value is now 0 (for positive)
        # or -1 (for negative, two's complement), we're done.
        if (value == 0 and (byte & 0x40) == 0) or (value == -1 and (byte & 0x40) != 0):
            more = False
        else:
            byte |= 0x80
        result.append(byte)
    return bytes(result)


def decode_uleb128(data: bytes, offset: int = 0) -> tuple[int, int]:
    """Decode ULEB128 from data at offset. Returns (value, bytes_consumed)."""
    result = 0
    shift = 0
    consumed = 0
    while True:
        byte = data[offset + consumed]
        consumed += 1
        result |= (byte & 0x7F) << shift
        shift += 7
        if (byte & 0x80) == 0:
            break
    return result, consumed


def decode_sleb128(data: bytes, offset: int = 0) -> tuple[int, int]:
    """Decode SLEB128 from data at offset. Returns (value, bytes_consumed)."""
    result = 0
    shift = 0
    consumed = 0
    while True:
        byte = data[offset + consumed]
        consumed += 1
        result |= (byte & 0x7F) << shift
        shift += 7
        if (byte & 0x80) == 0:
            # Sign-extend if the sign bit of the last byte is set
            if byte & 0x40:
                result |= -(1 << shift)
            break
    return result, consumed
