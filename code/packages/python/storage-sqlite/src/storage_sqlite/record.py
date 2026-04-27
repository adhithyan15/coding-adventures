"""
The record codec: SQL row values ↔ bytes.

A *record* is what SQLite writes into each B-tree cell (modulo the
rowid and payload-length prefix that the B-tree layer adds). It
encodes one row's columns as a tight byte string with a tiny internal
TOC — the "record header" — so that a reader can skip columns it
doesn't want without decoding everything before them.

Record layout
-------------

::

    [header-length varint]
    [serial type 1 varint] [serial type 2 varint] … [serial type N varint]
    [payload 1 bytes]      [payload 2 bytes]      … [payload N bytes]

- ``header-length`` covers *itself* plus every serial-type varint. So
  if a record has no columns, the header is the single byte ``0x01``
  (a 1-byte varint whose value is 1 — "the header is 1 byte long and
  contains nothing").
- Each serial type tells the decoder the type and byte width of the
  corresponding payload. See :data:`SERIAL_TYPES` below and the table
  in the module docstring of :mod:`storage_sqlite`.

Serial types
------------

::

    type  meaning                        bytes
    ----  -----------------------------  -----
      0   NULL                             0
      1   8-bit signed int                 1
      2   16-bit signed int BE             2
      3   24-bit signed int BE             3
      4   32-bit signed int BE             4
      5   48-bit signed int BE             6
      6   64-bit signed int BE             8
      7   64-bit IEEE 754 float BE         8
      8   the integer 0                    0
      9   the integer 1                    0
    10,11 reserved                          —
     N≥12 even  BLOB of (N-12)/2 bytes    variable
     N≥13 odd   TEXT (UTF-8) of (N-13)/2  variable

"Smallest fitting" matters
--------------------------

For byte-compat with the real sqlite3 binary, the integer ``3`` must
encode as serial type 1 (a single byte ``0x03``), not type 6 (eight
bytes). Real SQLite always picks the smallest serial type that fits
each value, and so do we. Integers 0 and 1 are special-cased further —
they collapse to serial types 8 and 9 with *zero* payload bytes.

Booleans
--------

Python ``bool`` is an ``int`` subclass, so ``isinstance(True, int)``
is true. We follow SQLite and store booleans as the integers 1 and 0.

References: `SQLite record format §2 <https://www.sqlite.org/fileformat2.html#record_format>`_.
"""

from __future__ import annotations

import struct

from storage_sqlite.errors import CorruptDatabaseError
from storage_sqlite.varint import decode as varint_decode
from storage_sqlite.varint import encode as varint_encode
from storage_sqlite.varint import size as varint_size

# A cell value in a record. No DECIMAL, no DATE/TIME — those map onto
# these five at the SQL type-affinity layer above us.
Value = None | int | float | str | bytes

# Serial type constants — using named values beats magic numbers when
# reading the encode/decode dispatchers below.
_ST_NULL: int = 0
_ST_INT8: int = 1
_ST_INT16: int = 2
_ST_INT24: int = 3
_ST_INT32: int = 4
_ST_INT48: int = 5
_ST_INT64: int = 6
_ST_FLOAT: int = 7
_ST_ZERO: int = 8
_ST_ONE: int = 9
# Values 10 and 11 are reserved ("internal use"). A well-formed file
# never contains them; seeing one on decode means corruption.

# Integer serial types paired with their byte widths. Order matters —
# the encoder walks this list from smallest to largest and stops at
# the first one whose range contains the value.
_INT_SERIAL_TYPES: tuple[tuple[int, int], ...] = (
    (_ST_INT8, 1),
    (_ST_INT16, 2),
    (_ST_INT24, 3),
    (_ST_INT32, 4),
    (_ST_INT48, 6),
    (_ST_INT64, 8),
)

_INT8_MIN: int = -(1 << 7)
_INT8_MAX: int = (1 << 7) - 1
_INT16_MIN: int = -(1 << 15)
_INT16_MAX: int = (1 << 15) - 1
_INT24_MIN: int = -(1 << 23)
_INT24_MAX: int = (1 << 23) - 1
_INT32_MIN: int = -(1 << 31)
_INT32_MAX: int = (1 << 31) - 1
_INT48_MIN: int = -(1 << 47)
_INT48_MAX: int = (1 << 47) - 1
_INT64_MIN: int = -(1 << 63)
_INT64_MAX: int = (1 << 63) - 1

# Parallel range table used by the encoder to pick the smallest type.
_INT_RANGES: tuple[tuple[int, int, int, int], ...] = (
    (_INT8_MIN, _INT8_MAX, _ST_INT8, 1),
    (_INT16_MIN, _INT16_MAX, _ST_INT16, 2),
    (_INT24_MIN, _INT24_MAX, _ST_INT24, 3),
    (_INT32_MIN, _INT32_MAX, _ST_INT32, 4),
    (_INT48_MIN, _INT48_MAX, _ST_INT48, 6),
    (_INT64_MIN, _INT64_MAX, _ST_INT64, 8),
)

# Decoder lookup: serial type → fixed payload width (for the types
# where the width is constant). Variable-width types (BLOB/TEXT) are
# computed from the serial type number itself.
_FIXED_WIDTHS: dict[int, int] = {
    _ST_NULL: 0,
    _ST_INT8: 1,
    _ST_INT16: 2,
    _ST_INT24: 3,
    _ST_INT32: 4,
    _ST_INT48: 6,
    _ST_INT64: 8,
    _ST_FLOAT: 8,
    _ST_ZERO: 0,
    _ST_ONE: 0,
}


# ----------------------------------------------------------------------
# Per-value encoding.
# ----------------------------------------------------------------------


def _encode_int(value: int) -> tuple[int, bytes]:
    """Pick the smallest serial type that fits ``value`` and pack it.

    Returns ``(serial_type, payload_bytes)``.
    """
    # Types 8 and 9 are zero-byte encodings for the constants 0 and 1.
    # They buy us space on the wire for what are the most common
    # integer column values (bit flags, NULL-ish columns).
    if value == 0:
        return _ST_ZERO, b""
    if value == 1:
        return _ST_ONE, b""

    for lo, hi, stype, width in _INT_RANGES:
        if lo <= value <= hi:
            return stype, _pack_int(value, width)

    raise ValueError(f"integer {value} out of SQLite 64-bit range")


def _pack_int(value: int, width: int) -> bytes:
    """Pack a signed integer big-endian into exactly ``width`` bytes.

    Python's ``int.to_bytes`` does exactly this as long as we pass
    ``signed=True``. SQLite uses two's complement for every signed
    width, including the odd 3-byte and 6-byte forms.
    """
    return value.to_bytes(width, byteorder="big", signed=True)


def _serial_type_and_payload(value: Value) -> tuple[int, bytes]:
    """Dispatch from a Python value to ``(serial_type, payload_bytes)``."""
    if value is None:
        return _ST_NULL, b""

    # ``bool`` comes before ``int`` in isinstance checks only
    # accidentally — both match ``int``. We handle bool as int here
    # because Python's ``True + 1 == 2`` semantics mean the int
    # handler is correct for bools too.
    if isinstance(value, int):
        return _encode_int(int(value))  # coerce bool and int subclasses

    if isinstance(value, float):
        # IEEE 754 double, big-endian. Python's struct handles NaN/inf
        # transparently; SQLite doesn't reject them at the record
        # layer either (the SQL layer above might, depending on typing).
        return _ST_FLOAT, struct.pack(">d", float(value))

    if isinstance(value, str):
        body = value.encode("utf-8")
        # TEXT serial type is 13 + 2*length, an odd number ≥ 13.
        return 13 + 2 * len(body), body

    if isinstance(value, bytes | bytearray | memoryview):
        body = bytes(value)
        # BLOB serial type is 12 + 2*length, an even number ≥ 12.
        return 12 + 2 * len(body), body

    raise TypeError(f"cannot encode value of type {type(value).__name__}: {value!r}")


# ----------------------------------------------------------------------
# Per-value decoding.
# ----------------------------------------------------------------------


def _payload_width(serial_type: int) -> int:
    """Return the number of payload bytes for ``serial_type``.

    For fixed-width types this is a table lookup; for BLOB/TEXT it's
    derived from the serial type number itself.
    """
    if serial_type in _FIXED_WIDTHS:
        return _FIXED_WIDTHS[serial_type]
    if serial_type == 10 or serial_type == 11:
        raise CorruptDatabaseError(f"serial type {serial_type} is reserved")
    if serial_type < 0:
        raise CorruptDatabaseError(f"negative serial type {serial_type}")
    # BLOB (even ≥ 12) or TEXT (odd ≥ 13).
    if serial_type % 2 == 0:
        return (serial_type - 12) // 2
    return (serial_type - 13) // 2


def _decode_value(serial_type: int, payload: bytes) -> Value:
    """Decode a payload slice into a Python value given its serial type."""
    if serial_type == _ST_NULL:
        return None
    if serial_type == _ST_ZERO:
        return 0
    if serial_type == _ST_ONE:
        return 1
    if serial_type in {_ST_INT8, _ST_INT16, _ST_INT24, _ST_INT32, _ST_INT48, _ST_INT64}:
        return int.from_bytes(payload, byteorder="big", signed=True)
    if serial_type == _ST_FLOAT:
        return struct.unpack(">d", payload)[0]
    if serial_type == 10 or serial_type == 11:
        raise CorruptDatabaseError(f"serial type {serial_type} is reserved")
    if serial_type % 2 == 0:  # BLOB
        return bytes(payload)
    # TEXT — malformed UTF-8 is a file-level corruption from the caller's
    # perspective, so translate the decode error to our corruption type.
    try:
        return payload.decode("utf-8")
    except UnicodeDecodeError as e:
        raise CorruptDatabaseError(f"TEXT column is not valid UTF-8: {e}") from e


# ----------------------------------------------------------------------
# Record encode / decode.
# ----------------------------------------------------------------------


def encode(values: list[Value] | tuple[Value, ...]) -> bytes:
    """Encode a list of column values into a record byte string.

    The header is a varint of the total header length, followed by one
    varint per column giving its serial type. Then the payloads
    follow in the same order.
    """
    # First pass: dispatch every value to (serial_type, payload_bytes).
    # We need these before we can size the header (which depends on the
    # serial-type varints) or concatenate payloads.
    per_value: list[tuple[int, bytes]] = [_serial_type_and_payload(v) for v in values]

    # The header-length varint covers itself plus every serial-type
    # varint. That self-reference means we have to iterate: guess a
    # size, check if the guess causes the size to change, adjust.
    # In practice the guess is right on the first or second try because
    # the only way it changes is if adding a byte to the length varint
    # bumps it across a 7-bit boundary — rare and bounded at 9 bytes.
    serial_type_bytes: list[bytes] = [varint_encode(st) for st, _ in per_value]
    types_total = sum(len(b) for b in serial_type_bytes)

    header_len = 1 + types_total  # 1 = assume 1-byte header-length varint
    while varint_size(header_len) != header_len - types_total:
        header_len = varint_size(header_len) + types_total

    out = bytearray()
    out += varint_encode(header_len)
    for b in serial_type_bytes:
        out += b
    for _, payload in per_value:
        out += payload
    return bytes(out)


def decode(data: bytes, offset: int = 0) -> tuple[list[Value], int]:
    """Decode a record at ``data[offset:]`` back to a list of values.

    Returns ``(values, bytes_consumed)``. Raises
    :class:`CorruptDatabaseError` if the header is malformed or the
    record is truncated.
    """
    try:
        header_len, header_len_width = varint_decode(data, offset)
    except ValueError as e:
        raise CorruptDatabaseError(f"record header length varint: {e}") from e

    if header_len < header_len_width:
        raise CorruptDatabaseError(
            f"record header_len={header_len} shorter than its own length varint "
            f"({header_len_width} bytes)"
        )

    # Walk the serial-type varints inside the declared header region.
    types: list[int] = []
    cursor = offset + header_len_width
    header_end = offset + header_len
    if header_end > len(data):
        raise CorruptDatabaseError(
            f"record header runs past buffer: header_end={header_end}, len={len(data)}"
        )
    while cursor < header_end:
        try:
            stype, consumed = varint_decode(data, cursor)
        except ValueError as e:
            raise CorruptDatabaseError(f"record serial type varint: {e}") from e
        types.append(stype)
        cursor += consumed
    if cursor != header_end:
        # Shouldn't happen — varint_decode consumes byte-aligned amounts
        # and we track them exactly — but guard against malformed data.
        raise CorruptDatabaseError("record header inconsistent with its length")

    values: list[Value] = []
    payload_cursor = header_end
    for stype in types:
        width = _payload_width(stype)
        end = payload_cursor + width
        if end > len(data):
            raise CorruptDatabaseError(
                f"record payload truncated at serial type {stype} (need {width} bytes)"
            )
        payload = bytes(data[payload_cursor:end])
        values.append(_decode_value(stype, payload))
        payload_cursor = end

    return values, payload_cursor - offset
