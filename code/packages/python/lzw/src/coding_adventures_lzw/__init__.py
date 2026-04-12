# =============================================================================
# CodingAdventures.LZW
# =============================================================================
#
# LZW (Lempel-Ziv-Welch, 1984) lossless compression algorithm.
# Part of the CMP compression series in the coding-adventures monorepo.
#
# What Is LZW?
# ------------
#
# LZW is LZ78 with one key change: the dictionary is **pre-seeded** with all
# 256 single-byte sequences before encoding begins. This means:
#
#   1. The encoder never needs to emit a raw byte outside the code stream —
#      every byte already has a code (0–255).
#   2. Tokens are just codes (unsigned integers), not (dict_index, next_char)
#      tuples like LZ78.
#   3. With only codes to transmit, the stream can be **bit-packed** at
#      variable width — codes start at 9 bits and grow as the dictionary
#      expands. This is exactly how GIF compression works.
#
# LZW encodes data as a sequence of dictionary codes:
#
#   CLEAR_CODE (256) — reset dictionary and code_size
#   STOP_CODE  (257) — end of stream
#   0–255      — pre-seeded single bytes
#   258+       — dynamically added multi-byte entries
#
# Reserved Codes
# --------------
#
#   0–255:  Pre-seeded. Code c decodes to the single byte c.
#   256:    CLEAR_CODE. Reset to initial 256-entry state.
#   257:    STOP_CODE.  End of stream.
#   258+:   Dynamic entries built during encoding.
#
# Wire Format (CMP03)
# -------------------
#
#   Bytes 0–3:  original_length  (big-endian uint32)
#   Bytes 4+:   bit-packed variable-width codes, LSB-first within each byte
#
#     - Starts at code_size = 9 bits
#     - Grows when next_code crosses the next power-of-2 boundary
#     - Maximum code_size = 16 (up to 65536 dictionary entries)
#     - Stream always begins with CLEAR_CODE and ends with STOP_CODE
#
# The Tricky Token
# ----------------
#
# During decoding there is a classic edge case: the decoder may receive a code
# equal to next_code — a code it has not yet added to its dictionary. This
# happens when the encoded sequence has the form xyx...x (the new entry starts
# with the same byte as the previous entry). In that case:
#
#   entry = dict[prev_code] + bytes([dict[prev_code][0]])
#
# This always produces the correct sequence because the encoder only emits such
# a code when the new entry equals the previous entry extended by its own first
# byte.
#
# The Series: CMP00 -> CMP05
# --------------------------
#
#   CMP00 (LZ77,    1977) — Sliding-window backreferences.
#   CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
#   CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals.
#   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF. (this module)
#   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
#   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
# =============================================================================

from __future__ import annotations

import struct

__all__ = [
    "compress",
    "decompress",
    "CLEAR_CODE",
    "STOP_CODE",
    "INITIAL_NEXT_CODE",
    "INITIAL_CODE_SIZE",
    "MAX_CODE_SIZE",
]

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

CLEAR_CODE: int = 256
"""Reset code — instructs the decoder to clear its dictionary and restart."""

STOP_CODE: int = 257
"""End-of-stream code — the decoder stops reading after this code."""

INITIAL_NEXT_CODE: int = 258
"""First dynamically assigned dictionary code."""

INITIAL_CODE_SIZE: int = 9
"""Starting bit-width for codes (covers 0–511, more than enough for 258)."""

MAX_CODE_SIZE: int = 16
"""Maximum bit-width; dictionary caps at 2^16 = 65536 entries."""


# ---------------------------------------------------------------------------
# Bit I/O helpers
# ---------------------------------------------------------------------------

class _BitWriter:
    """Accumulates variable-width codes into a byte buffer, LSB-first.

    LSB-first packing means the first code written occupies bits 0..N-1 of
    the first byte, spilling into subsequent bytes if necessary. This matches
    the GIF and Unix compress conventions.

    Example — writing code 0b110100101 (9 bits, value 421) then code 0b10 (2 bits):

      buffer = 0b110100101  (after first write)
      byte 0 = 0b10100101  (low 8 bits emitted)
      buffer = 0b1          remaining high bit
      write 0b10 → buffer = 0b101  (shift left by 1 to combine with remainder)
      (flush) byte 1 = 0b101
    """

    def __init__(self) -> None:
        self._buffer: int = 0   # bit accumulator (up to 64 bits)
        self._bit_pos: int = 0  # number of valid bits in _buffer
        self._output: bytearray = bytearray()

    def write(self, code: int, code_size: int) -> None:
        """Write `code` using exactly `code_size` bits."""
        self._buffer |= code << self._bit_pos
        self._bit_pos += code_size
        while self._bit_pos >= 8:
            self._output.append(self._buffer & 0xFF)
            self._buffer >>= 8
            self._bit_pos -= 8

    def flush(self) -> None:
        """Flush any remaining bits as a final partial byte."""
        if self._bit_pos > 0:
            self._output.append(self._buffer & 0xFF)
            self._buffer = 0
            self._bit_pos = 0

    def bytes(self) -> bytes:
        """Return the accumulated output as an immutable bytes object."""
        return bytes(self._output)


class _BitReader:
    """Reads variable-width codes from a byte buffer, LSB-first.

    Mirrors _BitWriter exactly: bits within each byte are consumed from the
    least-significant end first.
    """

    def __init__(self, data: bytes) -> None:
        self._data: bytes = data
        self._pos: int = 0      # next byte index to read from _data
        self._buffer: int = 0   # bit accumulator
        self._bit_pos: int = 0  # number of valid bits in _buffer

    def read(self, code_size: int) -> int:
        """Read and return the next `code_size`-bit code.

        Raises EOFError if the stream is exhausted before enough bits are
        available.
        """
        while self._bit_pos < code_size:
            if self._pos >= len(self._data):
                msg = "unexpected end of bit stream"
                raise EOFError(msg)
            self._buffer |= self._data[self._pos] << self._bit_pos
            self._pos += 1
            self._bit_pos += 8
        code = self._buffer & ((1 << code_size) - 1)
        self._buffer >>= code_size
        self._bit_pos -= code_size
        return code

    def exhausted(self) -> bool:
        """Return True when no more bits can be read."""
        return self._pos >= len(self._data) and self._bit_pos == 0


# ---------------------------------------------------------------------------
# Encoder
# ---------------------------------------------------------------------------

def _encode_codes(data: bytes | bytearray) -> tuple[list[int], int]:
    """Encode *data* into a list of LZW codes (including CLEAR and STOP).

    Returns ``(codes, original_length)`` where ``original_length`` is
    ``len(data)`` — stored in the wire-format header so the decoder can trim
    any bit-padding added by the flush step.

    Algorithm:
      1. Initialise the encode dictionary: byte → code for all 256 bytes.
      2. Emit CLEAR_CODE to mark the start of the stream.
      3. Walk the input byte-by-byte, extending the current prefix *w*:
         - If *w + b* is already in the dictionary, extend *w*.
         - Otherwise, emit code_for(*w*), add *w + b* as a new entry, reset
           *w* to just *b*.
         - When the dictionary is full (next_code == 2^MAX_CODE_SIZE), emit
           CLEAR_CODE and re-initialise.
      4. Flush the remaining prefix and emit STOP_CODE.
    """
    codes: list[int] = []
    original_length = len(data)

    # Encode dictionary: sequence → code.
    # Keys are bytes objects (immutable, hashable).
    encode_dict: dict[bytes, int] = {bytes([b]): b for b in range(256)}
    next_code = INITIAL_NEXT_CODE
    max_entries = 1 << MAX_CODE_SIZE  # 65536

    codes.append(CLEAR_CODE)

    w = b""  # current working prefix (bytes)

    for byte in data:
        wb = w + bytes([byte])
        if wb in encode_dict:
            w = wb  # extend the prefix
        else:
            # Emit code for the current prefix.
            codes.append(encode_dict[w])

            if next_code < max_entries:
                encode_dict[wb] = next_code
                next_code += 1
            elif next_code == max_entries:
                # Dictionary full — emit CLEAR and reset.
                codes.append(CLEAR_CODE)
                encode_dict = {bytes([b]): b for b in range(256)}
                next_code = INITIAL_NEXT_CODE

            w = bytes([byte])  # restart with the unmatched byte

    # Flush remaining prefix.
    if w:
        codes.append(encode_dict[w])

    codes.append(STOP_CODE)
    return codes, original_length


# ---------------------------------------------------------------------------
# Decoder
# ---------------------------------------------------------------------------

def _decode_codes(codes: list[int]) -> bytes:
    """Decode a list of LZW codes back to a byte string.

    Handles:
      - CLEAR_CODE: reset dictionary and code_size.
      - STOP_CODE: stop decoding.
      - Tricky token (code == next_code): construct the entry as
        ``dict[prev_code] + bytes([dict[prev_code][0]])``.

    Invalid codes (code > next_code, or code < 0) are skipped silently to
    provide robustness against minor stream corruption.
    """
    # Decode dictionary: code → byte sequence.
    # Initialise with all 256 single-byte entries plus CLEAR/STOP placeholders.
    decode_dict: list[bytes] = [bytes([b]) for b in range(256)]
    decode_dict.append(b"")  # 256 = CLEAR_CODE placeholder
    decode_dict.append(b"")  # 257 = STOP_CODE  placeholder
    next_code = INITIAL_NEXT_CODE

    output = bytearray()
    prev_code: int | None = None

    for code in codes:
        if code == CLEAR_CODE:
            # Reset dictionary to 256 single-byte entries.
            decode_dict = [bytes([b]) for b in range(256)]
            decode_dict.append(b"")  # 256
            decode_dict.append(b"")  # 257
            next_code = INITIAL_NEXT_CODE
            prev_code = None
            continue

        if code == STOP_CODE:
            break

        # Resolve the entry for this code.
        if code < len(decode_dict):
            entry = decode_dict[code]
        elif code == next_code:
            # Tricky token: the code refers to an entry not yet in the dict.
            # By construction this only happens when the new entry starts with
            # the same byte as the previous entry.
            if prev_code is None:
                # Malformed stream — skip.
                continue
            prev_entry = decode_dict[prev_code]
            entry = prev_entry + bytes([prev_entry[0]])
        else:
            # Truly invalid code — skip.
            continue

        output.extend(entry)

        # Add new entry to the decode dictionary.
        if prev_code is not None and next_code < (1 << MAX_CODE_SIZE):
            prev_entry = decode_dict[prev_code]
            decode_dict.append(prev_entry + bytes([entry[0]]))
            next_code += 1

        prev_code = code

    return bytes(output)


# ---------------------------------------------------------------------------
# Serialisation
# ---------------------------------------------------------------------------

def _pack_codes(codes: list[int], original_length: int) -> bytes:
    """Pack a list of LZW codes into the CMP03 wire format.

    Wire format:
      Bytes 0–3: original_length (big-endian uint32)
      Bytes 4+:  codes as variable-width LSB-first bit-packed integers

    The code size starts at INITIAL_CODE_SIZE (9) and grows each time
    next_code crosses the next power-of-2 boundary.  CLEAR_CODE resets the
    code size back to INITIAL_CODE_SIZE.
    """
    writer = _BitWriter()
    code_size = INITIAL_CODE_SIZE
    next_code = INITIAL_NEXT_CODE

    for code in codes:
        writer.write(code, code_size)

        if code == CLEAR_CODE:
            # After CLEAR, the receiver resets to code_size=9 and next_code=258.
            code_size = INITIAL_CODE_SIZE
            next_code = INITIAL_NEXT_CODE
        elif code != STOP_CODE:
            # Each emitted data code corresponds to a new dictionary entry on
            # both sides (except STOP).  Advance next_code and grow code_size
            # as needed.
            if next_code < (1 << MAX_CODE_SIZE):
                next_code += 1
                if next_code > (1 << code_size) and code_size < MAX_CODE_SIZE:
                    code_size += 1

    writer.flush()
    header = struct.pack(">I", original_length)
    return header + writer.bytes()


def _unpack_codes(data: bytes) -> tuple[list[int], int]:
    """Unpack CMP03 wire format bytes into a list of LZW codes.

    Returns ``(codes, original_length)``.

    Security: the decoder stops on STOP_CODE or stream exhaustion, so a
    crafted stream cannot cause unbounded iteration.
    """
    if len(data) < 4:
        return [CLEAR_CODE, STOP_CODE], 0

    (original_length,) = struct.unpack(">I", data[:4])
    reader = _BitReader(data[4:])

    codes: list[int] = []
    code_size = INITIAL_CODE_SIZE
    next_code = INITIAL_NEXT_CODE

    try:
        while not reader.exhausted():
            code = reader.read(code_size)
            codes.append(code)

            if code == STOP_CODE:
                break
            elif code == CLEAR_CODE:
                code_size = INITIAL_CODE_SIZE
                next_code = INITIAL_NEXT_CODE
            else:
                if next_code < (1 << MAX_CODE_SIZE):
                    next_code += 1
                    if next_code > (1 << code_size) and code_size < MAX_CODE_SIZE:
                        code_size += 1
    except EOFError:
        # Truncated stream — treat as STOP.
        pass

    return codes, original_length


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def compress(
    data: bytes | bytearray,
    max_code_size: int = MAX_CODE_SIZE,  # noqa: ARG001
) -> bytes:
    """Compress *data* using LZW and return CMP03 wire-format bytes.

    Parameters
    ----------
    data:
        The raw bytes to compress.
    max_code_size:
        Maximum code bit-width.  Currently fixed at 16; the parameter exists
        for forward-compatibility with future variants.

    Returns
    -------
    bytes
        Compressed data in CMP03 wire format.
    """
    codes, original_length = _encode_codes(bytes(data))
    return _pack_codes(codes, original_length)


def decompress(data: bytes | bytearray) -> bytes:
    """Decompress CMP03 wire-format *data* and return the original bytes.

    Parameters
    ----------
    data:
        Compressed bytes produced by :func:`compress`.

    Returns
    -------
    bytes
        The original, uncompressed data.
    """
    codes, original_length = _unpack_codes(bytes(data))
    result = _decode_codes(codes)
    # Trim any bit-padding artefacts using the stored original_length.
    return result[:original_length]
