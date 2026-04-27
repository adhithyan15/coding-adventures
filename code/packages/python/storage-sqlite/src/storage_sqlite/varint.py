"""
SQLite's big-endian, variable-length integer encoding.

Varints show up *everywhere* in the SQLite file format — record headers,
serial types, rowids, cell payload lengths, freelist entries. Any byte
that names "how many of something follows" is a varint.

The encoding
------------

A varint is 1 to 9 bytes. For bytes 1 through 8, the high bit is a
**continuation flag**: 1 means "another byte follows", 0 means "this is
the last byte of the value, and the low 7 bits are the tail of the
number". The 9th byte is special — if we get there, it contributes all
8 of its bits to the value (no continuation bit, because there can't
be a 10th byte).

So the value is the concatenation of:

- the low 7 bits of bytes 1..8, in big-endian order, as long as the
  high bit of each is set
- the 8 bits of byte 9 if we reach it

Worked example — encoding the integer ``300``:

::

    300 = 0b100101100   (9 bits)
    ─────────────────
    split into 7-bit chunks, big-endian:
        high 7 bits:  0b0000010 = 0x02
        low  7 bits:  0b0101100 = 0x2C
    byte 1:  0x02 | 0x80 = 0x82   (continuation bit set)
    byte 2:  0x2C                 (final byte)

So ``300`` encodes as ``b"\\x82\\x2c"`` — 2 bytes.

Bytes needed, by value range:

::

    1 byte:   0 .. 2^7-1      (128 values)
    2 bytes:  2^7 .. 2^14-1
    3 bytes:  2^14 .. 2^21-1
    ...
    8 bytes:  2^49 .. 2^56-1
    9 bytes:  2^56 .. 2^64-1   (the full u64 range)

Signed values
-------------

Varints are conceptually unsigned, but SQLite uses them to store
signed 64-bit integers too (as rowids and as serial-type-6 bodies only
indirectly — the serial type itself is always non-negative). We handle
the signed case at the varint layer by reinterpreting the unsigned
64-bit value as a two's-complement signed 64-bit value when the caller
asks for signed decoding. This matches SQLite's internal use, which
treats the 9-byte form as "the full 64 bits, signed or unsigned
depending on context".

References:
`SQLite file format §7 <https://www.sqlite.org/fileformat2.html#varint>`_.
"""

from __future__ import annotations

# Largest value that fits in 8 bytes of varint (8 × 7 = 56 bits). Any
# value this big or bigger needs the 9-byte form where byte 9 carries
# its full 8 bits instead of 7.
_MAX_8BYTE_VARINT: int = (1 << 56) - 1

# u64 ceiling. Varints can represent the full unsigned 64-bit range —
# byte 9's 8 bits plus bytes 1..8's 7-bit payloads gives 64 total.
_U64_MAX: int = (1 << 64) - 1

# For signed reinterpretation: if the top bit of the u64 is set, the
# value is negative in two's complement.
_I64_SIGN_BIT: int = 1 << 63


def encode(value: int) -> bytes:
    """Encode an unsigned integer as a SQLite varint.

    Accepts values in ``[0, 2**64)``. Signed values (rowids etc.) should
    be converted to their unsigned two's-complement representation by
    the caller before calling, or use :func:`encode_signed`.

    The output is 1..9 bytes — always the *shortest* form that can
    represent the value. That's required for byte-compat: real SQLite
    writes minimal varints, so a round-trip that widens a value would
    produce files that don't match.
    """
    if value < 0 or value > _U64_MAX:
        raise ValueError(f"varint out of range [0, 2**64): {value}")

    # 9-byte form — byte 9 takes 8 bits, bytes 1..8 each take 7.
    if value > _MAX_8BYTE_VARINT:
        out = bytearray(9)
        # Low 8 bits go into byte 9 (the last byte, no continuation flag).
        out[8] = value & 0xFF
        v = value >> 8
        # Bytes 1..8 hold the upper 56 bits, 7 per byte, high bit set on
        # all but the last-of-eight (which is byte 8). But because we're
        # in the 9-byte form, *all* of bytes 1..8 have the continuation
        # bit set — the "this is the final byte" signal is "we reached
        # byte 9", not a cleared bit in byte 8.
        for i in range(7, -1, -1):
            out[i] = (v & 0x7F) | 0x80
            v >>= 7
        return bytes(out)

    # 1..8 byte form — every non-final byte has the continuation bit set;
    # the final byte does not. Build the bytes high-to-low in a small
    # buffer, then reverse.
    if value == 0:
        return b"\x00"

    chunks: list[int] = []
    v = value
    # First (least-significant) chunk has no continuation flag — it's
    # the "last byte" signal.
    chunks.append(v & 0x7F)
    v >>= 7
    while v > 0:
        chunks.append((v & 0x7F) | 0x80)
        v >>= 7
    # We built chunks low-to-high; the wire order is high-to-low.
    chunks.reverse()
    return bytes(chunks)


def encode_signed(value: int) -> bytes:
    """Encode a signed 64-bit integer as a varint.

    Negative values are cast to their two's-complement unsigned form
    first — a signed −1 becomes unsigned ``2**64 − 1`` and always fills
    the 9-byte form.
    """
    if value < -(1 << 63) or value >= (1 << 63):
        raise ValueError(f"signed varint out of range [-2**63, 2**63): {value}")
    if value < 0:
        value += 1 << 64
    return encode(value)


def decode(data: bytes, offset: int = 0) -> tuple[int, int]:
    """Decode a varint at ``data[offset:]``.

    Returns ``(value, bytes_consumed)``. ``bytes_consumed`` is always
    between 1 and 9.

    Raises :class:`ValueError` if the buffer is too short to contain a
    complete varint.
    """
    end = len(data)
    if offset >= end:
        raise ValueError("varint decode: buffer empty at offset")

    value = 0
    # Bytes 1..8: 7 bits each, with continuation.
    for i in range(8):
        pos = offset + i
        if pos >= end:
            raise ValueError("varint decode: buffer truncated mid-continuation")
        byte = data[pos]
        if byte & 0x80:
            # Continuation bit set — absorb the low 7 bits and keep going.
            value = (value << 7) | (byte & 0x7F)
        else:
            # No continuation — this is the final byte. Add its 7 bits
            # and we're done.
            value = (value << 7) | byte
            return value, i + 1

    # All 8 high bits were set — we're in the 9-byte form. Byte 9
    # contributes its full 8 bits and terminates.
    pos = offset + 8
    if pos >= end:
        raise ValueError("varint decode: buffer truncated before 9th byte")
    value = (value << 8) | data[pos]
    return value, 9


def decode_signed(data: bytes, offset: int = 0) -> tuple[int, int]:
    """Decode a signed varint. See :func:`decode`.

    The unsigned u64 is reinterpreted as a signed i64 (two's complement).
    """
    value, consumed = decode(data, offset)
    if value & _I64_SIGN_BIT:
        value -= 1 << 64
    return value, consumed


def size(value: int) -> int:
    """Return the number of bytes :func:`encode` would produce.

    Useful for computing record-header lengths without actually encoding.
    """
    if value < 0 or value > _U64_MAX:
        raise ValueError(f"varint out of range [0, 2**64): {value}")
    if value > _MAX_8BYTE_VARINT:
        return 9
    if value == 0:
        return 1
    # One byte per 7 bits of payload, rounded up. bit_length gives the
    # number of significant bits; (bits + 6) // 7 rounds up.
    return (value.bit_length() + 6) // 7
