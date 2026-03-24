"""wasm-leb128 — LEB128 variable-length integer encoding for WASM binary format

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.

-------------------------------------------------------------------------------
WHAT IS LEB128?
-------------------------------------------------------------------------------

LEB128 stands for "Little-Endian Base-128." It is a variable-length encoding
for integers, invented at Hewlett-Packard and used prominently in:

  - WebAssembly binary format (for all integer values)
  - DWARF debug information
  - Android DEX files
  - Protocol Buffers (a variant called "varint")

The key insight: most integers in real programs are small. A 32-bit integer
field that always holds 0–127 wastes 3 bytes in a fixed-width encoding. LEB128
only uses 1 byte for those values, 2 bytes for values up to 16383, and so on.

-------------------------------------------------------------------------------
HOW LEB128 WORKS — THE BIT LAYOUT
-------------------------------------------------------------------------------

Each byte of a LEB128 value is structured as follows:

    Bit 7 (high bit): continuation flag
    Bits 6–0 (low 7 bits): data payload

    ┌─────┬─────────────────────────────────────────────────────────────┐
    │ Bit │  7   6   5   4   3   2   1   0                              │
    │─────┼─────────────────────────────────────────────────────────────│
    │Role │ MSB  d6  d5  d4  d3  d2  d1  d0  (MSB = continuation flag)  │
    └─────┴─────────────────────────────────────────────────────────────┘

    MSB = 1 → more bytes follow
    MSB = 0 → this is the last byte

Example: encode 624485 as unsigned LEB128

    624485 in binary: 0010_0110_0001_0000_1110_0101

    Split into 7-bit groups (right to left):
        0100110    0001110    1100101
        (group 3)  (group 2)  (group 1)

    Each group becomes a byte. All groups except the last get MSB=1:
        group 1: 1_1100101 = 0xE5  (more bytes follow)
        group 2: 1_0001110 = 0x8E  (more bytes follow)
        group 3: 0_0100110 = 0x26  (last byte, MSB=0)

    Result: [0xE5, 0x8E, 0x26]

Decoding reverses this: strip the MSB from each byte, reconstruct the
integer by placing each 7-bit group at increasing bit positions.

-------------------------------------------------------------------------------
SIGNED LEB128
-------------------------------------------------------------------------------

Signed LEB128 (used for i32, i64 in WASM) uses two's complement and sign
extension. The difference from unsigned:

  - After reading the last byte (MSB=0), check whether its bit 6 is set.
  - If bit 6 is set, the value is negative and we sign-extend: set all bits
    above the current bit position to 1.

Example: encode -2 as signed LEB128

    In 32-bit two's complement: -2 = 0xFFFFFFFE = 1111...1110

    We take 7-bit groups:
        ...1111111    1111110   → two groups would be 7F, 7E
        But -2 fits in one byte: 0x7E = 0b01111110
            bits 6–0: 1111110 (the low 7 bits of -2)
            MSB: 0 (last byte)
        Sign bit (bit 6 of last byte) = 1 → this is negative → correct.

    Result: [0x7E]

    Decoding 0x7E:
        value = 0x7E & 0x7F = 0b1111110 = 126 (raw bits)
        bit_position = 7 (we consumed 7 bits)
        last byte, bit 6 = 1 → sign extend:
            value |= -(1 << 7) = value | 0xFFFFFF80
            value = 0b1111110 | 0xFFFFFF80 = 0xFFFFFFFE = -2 ✓

-------------------------------------------------------------------------------
API SUMMARY
-------------------------------------------------------------------------------

    decode_unsigned(data, offset=0) → (value: int, bytes_consumed: int)
    decode_signed(data, offset=0)   → (value: int, bytes_consumed: int)
    encode_unsigned(value: int)     → bytes
    encode_signed(value: int)       → bytes

Errors:
    LEB128Error — raised when the byte stream is unterminated (all bytes have
                  continuation bit set but we run out of data).
"""

from __future__ import annotations

__version__ = "0.1.0"

# ---------------------------------------------------------------------------
# Maximum bytes for 32-bit LEB128 values.
#
# A 32-bit integer needs at most ceil(32/7) = 5 bytes in LEB128.
# The 5th byte can only contribute 4 bits (bits 28–31), so its valid range
# for unsigned u32 is 0x00–0x0F (for unsigned) and 0x00–0x0F or 0x70–0x7F
# (for signed i32).
#
# We use this limit as a safety guard against malformed data.
# ---------------------------------------------------------------------------
_MAX_BYTES = 5


class LEB128Error(Exception):
    """Raised when LEB128 decoding encounters malformed input.

    Attributes:
        message: Human-readable description of what went wrong.
        offset:  Byte offset in the input data where the error occurred.

    Example:
        try:
            decode_unsigned(b'\\x80\\x80')  # unterminated — no final byte
        except LEB128Error as e:
            print(e.message)  # "unterminated LEB128 at offset 0: ..."
            print(e.offset)   # 0
    """

    def __init__(self, message: str, offset: int) -> None:
        super().__init__(message)
        self.message = message
        self.offset = offset


# ---------------------------------------------------------------------------
# DECODE UNSIGNED
#
# Algorithm:
#   result = 0
#   shift  = 0
#   for each byte at (offset, offset+1, offset+2, ...):
#       payload = byte & 0x7F        # strip the continuation bit
#       result |= payload << shift   # place the 7 bits at the right position
#       shift  += 7
#       if byte & 0x80 == 0:         # continuation bit not set → done
#           return (result, bytes_consumed)
#   → if we reach here, we ran out of bytes → error
# ---------------------------------------------------------------------------
def decode_unsigned(data: bytes | bytearray, offset: int = 0) -> tuple[int, int]:
    """Decode an unsigned LEB128 integer from *data* starting at *offset*.

    Args:
        data:   Raw bytes containing the LEB128-encoded integer.
        offset: Starting position within *data*. Defaults to 0.

    Returns:
        A tuple ``(value, bytes_consumed)`` where *value* is the decoded
        non-negative integer and *bytes_consumed* is how many bytes were read.

    Raises:
        LEB128Error: If the sequence is unterminated (all bytes read have
                     continuation bit set but no terminating byte found).

    Example:
        >>> decode_unsigned(bytes([0xE5, 0x8E, 0x26]))
        (624485, 3)
        >>> decode_unsigned(bytes([0x00]))
        (0, 1)
        >>> decode_unsigned(bytes([0xFF, 0x00, 0x03]), offset=1)
        (0, 1)
    """
    result = 0
    shift = 0
    start = offset

    for i in range(_MAX_BYTES):
        # Safety check: make sure we haven't gone past the end of the buffer.
        if offset >= len(data):
            raise LEB128Error(
                f"unterminated LEB128 at offset {start}: ran out of bytes after "
                f"{i} byte(s) (continuation bit still set)",
                start,
            )

        byte = data[offset]
        offset += 1

        # Extract the 7 data bits and place them at the correct bit position.
        #
        # Visual example (i=0, shift=0):
        #   byte    = 0xE5 = 1_1100101
        #                    ^ continuation bit (stripped)
        #                      ^^^^^^^ 7 data bits = 0x65 = 101
        #   payload = 0xE5 & 0x7F = 0x65
        #   result |= 0x65 << 0  → result = 0x65
        #
        # (i=1, shift=7):
        #   byte    = 0x8E = 1_0001110
        #   payload = 0x0E
        #   result |= 0x0E << 7 = 0x700 → result = 0x765
        #
        # (i=2, shift=14):
        #   byte    = 0x26 = 0_0100110 ← MSB=0, last byte
        #   payload = 0x26
        #   result |= 0x26 << 14 = 0x98000 → result = 0x98765 = 624485
        payload = byte & 0x7F
        result |= payload << shift
        shift += 7

        if (byte & 0x80) == 0:
            # Continuation bit is 0 → this was the last byte.
            return result, offset - start

    # If we exit the loop without returning, we consumed _MAX_BYTES bytes but
    # still haven't seen a terminating byte. That's malformed data.
    raise LEB128Error(
        f"unterminated LEB128 at offset {start}: continuation bit still set after "
        f"{_MAX_BYTES} bytes (maximum for 32-bit encoding)",
        start,
    )


# ---------------------------------------------------------------------------
# DECODE SIGNED
#
# Identical to decode_unsigned EXCEPT: after reading the last byte, check
# whether the value's sign bit is set. If so, sign-extend to Python's
# arbitrary-precision integer (which naturally handles all widths).
#
# "Sign extension" means: if the number ends at bit position `shift`, and bit
# (shift - 1) is 1, then all bits from `shift` upward should be 1 too. In
# Python integers (which have infinite precision), we do:
#
#   result |= -(1 << shift)
#
# Because -(1 << shift) in Python has all bits set from position `shift` up
# (an infinite string of 1s in binary). OR-ing that into result propagates 1s
# into every bit above the sign bit.
#
# Example: -2 encoded as [0x7E]
#   byte    = 0x7E = 0_1111110 (MSB=0, last byte)
#   payload = 0x7E & 0x7F = 0x7E = 0b1111110 = 126
#   result  = 126
#   shift   = 7
#   sign bit check: (0x7E & 0x40) != 0 → True (bit 6 of last byte is 1)
#   sign extend: result |= -(1 << 7) = -(128)
#     = 126 | -128 = 0b01111110 | ...11110000000 = ...11111110 = -2 ✓
# ---------------------------------------------------------------------------
def decode_signed(data: bytes | bytearray, offset: int = 0) -> tuple[int, int]:
    """Decode a signed LEB128 integer from *data* starting at *offset*.

    Uses two's complement sign extension: if the high bit of the last data
    group is 1, the value is negative.

    Args:
        data:   Raw bytes containing the LEB128-encoded integer.
        offset: Starting position within *data*. Defaults to 0.

    Returns:
        A tuple ``(value, bytes_consumed)`` where *value* is the decoded
        signed integer and *bytes_consumed* is how many bytes were read.

    Raises:
        LEB128Error: If the sequence is unterminated.

    Example:
        >>> decode_signed(bytes([0x7E]))
        (-2, 1)
        >>> decode_signed(bytes([0x80, 0x80, 0x80, 0x80, 0x78]))
        (-2147483648, 5)
    """
    result = 0
    shift = 0
    start = offset

    for i in range(_MAX_BYTES):
        if offset >= len(data):
            raise LEB128Error(
                f"unterminated LEB128 at offset {start}: ran out of bytes after "
                f"{i} byte(s) (continuation bit still set)",
                start,
            )

        byte = data[offset]
        offset += 1

        payload = byte & 0x7F
        result |= payload << shift
        shift += 7

        if (byte & 0x80) == 0:
            # Last byte. Check sign bit.
            #
            # The sign bit of our decoded value is at position (shift - 1),
            # which is bit (shift-1) in *result*. Equivalently, it is bit 6
            # of the last byte we just read (since each byte contributes 7
            # bits, the top data bit of any byte is bit 6).
            #
            # We test: is bit (shift-1) of result set?
            #   Equivalently: (byte & 0x40) != 0
            #
            # If yes, sign-extend by OR-ing in an infinite sequence of 1-bits
            # starting at position `shift`.
            if byte & 0x40:
                result |= -(1 << shift)
            return result, offset - start

    raise LEB128Error(
        f"unterminated LEB128 at offset {start}: continuation bit still set after "
        f"{_MAX_BYTES} bytes (maximum for 32-bit encoding)",
        start,
    )


# ---------------------------------------------------------------------------
# ENCODE UNSIGNED
#
# Algorithm (inverse of decode):
#   loop:
#       take the low 7 bits of value   → payload
#       shift value right by 7
#       if value != 0:
#           emit (payload | 0x80)      # more bytes follow
#       else:
#           emit payload               # last byte, MSB stays 0
#           break
#
# Example: encode 624485 = 0x98465
#   iteration 1: payload = 0x98465 & 0x7F = 0x65, value >>= 7 → 0x1308 (≠0)
#                emit 0x65 | 0x80 = 0xE5
#   iteration 2: payload = 0x1308 & 0x7F = 0x08, value >>= 7 → 0x26 (≠0)
#                emit 0x08 | 0x80 = 0x88
#   iteration 3: payload = 0x26 & 0x7F = 0x26, value >>= 7 → 0 (=0)
#                emit 0x26
#   result: [0xE5, 0x88, 0x26] ✓
# ---------------------------------------------------------------------------
def encode_unsigned(value: int) -> bytes:
    """Encode a non-negative integer as unsigned LEB128.

    Args:
        value: Non-negative integer to encode. Must be >= 0.

    Returns:
        LEB128-encoded bytes.

    Raises:
        ValueError: If *value* is negative.

    Example:
        >>> encode_unsigned(0)
        b'\\x00'
        >>> encode_unsigned(624485)
        b'\\xe5\\x88&'
        >>> list(encode_unsigned(3))
        [3]
    """
    if value < 0:
        msg = f"encode_unsigned: value must be non-negative, got {value}"
        raise ValueError(msg)

    out: list[int] = []

    # We must iterate at least once, even for value=0, to emit the single
    # byte 0x00 rather than an empty sequence.
    while True:
        payload = value & 0x7F   # low 7 bits
        value >>= 7
        if value != 0:
            # More groups remain → set the continuation bit.
            out.append(payload | 0x80)
        else:
            # This is the final group → continuation bit stays 0.
            out.append(payload)
            break

    return bytes(out)


# ---------------------------------------------------------------------------
# ENCODE SIGNED
#
# Very similar to encode_unsigned, but we keep going until the "remaining"
# value can be represented unambiguously with the sign bit of the last byte.
#
# The loop continues as long as EITHER:
#   (a) more is yet to be encoded, AND
#   (b) the sign of the remaining value does not match bit 6 of the payload.
#
# Condition (b) is checked by: not (value == 0 and (payload & 0x40) == 0)
#                               not (value == -1 and (payload & 0x40) != 0)
#
# Which is equivalently collapsed into the standard done-condition:
#   done = (value == 0 and (payload & 0x40) == 0) or
#          (value == -1 and (payload & 0x40) != 0)
#
# Why do we check this? Because the decoder uses bit 6 of the last byte as
# the sign bit. If we stopped early, the decoder would misinterpret the sign.
#
# Example: encode -2
#   value = -2 = ...1111111111111110 (two's complement, Python arbitrary int)
#   iteration 1:
#       payload = -2 & 0x7F = 0x7E = 0b1111110  (bit 6 = 1 → negative sign)
#       value >>= 7 (arithmetic for negative in Python) = -1
#       done? value==-1 and (payload & 0x40)!=0  →  True
#       emit 0x7E
#   result: [0x7E] ✓
#
# Example: encode 64 (tricky! bit 6 of 0x40 is set, but 64 is positive)
#   iteration 1:
#       payload = 64 & 0x7F = 0x40  (bit 6 = 1 — would look negative!)
#       value >>= 7 = 0
#       done? value==0 and (0x40 & 0x40)==0  →  False (bit 6 IS set)
#       emit 0x40 | 0x80 = 0xC0  (not done yet)
#   iteration 2:
#       payload = 0 & 0x7F = 0x00  (bit 6 = 0)
#       value >>= 7 = 0
#       done? value==0 and (0x00 & 0x40)==0  →  True
#       emit 0x00
#   result: [0xC0, 0x00] — decodes to 64 with correct positive sign ✓
# ---------------------------------------------------------------------------
def encode_signed(value: int) -> bytes:
    """Encode a signed integer as signed LEB128 (two's complement).

    Args:
        value: Any integer (positive, negative, or zero).

    Returns:
        LEB128-encoded bytes.

    Example:
        >>> encode_signed(0)
        b'\\x00'
        >>> encode_signed(-2)
        b'~'
        >>> list(encode_signed(-2147483648))
        [128, 128, 128, 128, 120]
    """
    out: list[int] = []

    while True:
        payload = value & 0x7F   # low 7 bits
        value >>= 7              # arithmetic right shift (sign-preserving)

        # Check if this can be the final byte.
        # It's final when the remaining value would be fully represented by
        # the sign extension of bit 6 in this byte.
        done = (value == 0 and (payload & 0x40) == 0) or (
            value == -1 and (payload & 0x40) != 0
        )

        if done:
            out.append(payload)   # last byte — no continuation bit
            break
        else:
            out.append(payload | 0x80)   # more bytes follow

    return bytes(out)
