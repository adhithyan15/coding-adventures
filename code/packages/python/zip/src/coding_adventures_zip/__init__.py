"""zip — CMP09: ZIP archive format (PKZIP, 1989).

ZIP bundles one or more files into a single ``.zip`` archive, compressing each
entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
The same format underlies Java JARs, Office Open XML (.docx), Android APKs,
Python wheels (.whl), and many more.

Architecture
============

.. code-block:: text

    ┌─────────────────────────────────────────────────────┐
    │  [Local File Header + File Data]  ← entry 1         │
    │  [Local File Header + File Data]  ← entry 2         │
    │  ...                                                │
    │  ══════════ Central Directory ══════════            │
    │  [Central Dir Header]  ← entry 1 (has local offset)│
    │  [Central Dir Header]  ← entry 2                   │
    │  [End of Central Directory Record]                  │
    └─────────────────────────────────────────────────────┘

The dual-header design enables two workflows:

- **Sequential write**: append Local Headers one-by-one, write CD at the end.
- **Random-access read**: seek to EOCD at the end, read CD, jump to any entry.

Wire Format (all integers little-endian)
=========================================

Local File Header (30 + n + e bytes)::

    0x04034B50  signature
    version_needed u16   20=DEFLATE, 10=Stored
    flags u16            bit 11 = UTF-8 filename
    method u16           0=Stored, 8=DEFLATE
    mod_time u16         MS-DOS packed time
    mod_date u16         MS-DOS packed date
    crc32 u32
    compressed_size u32
    uncompressed_size u32
    name_len u16
    extra_len u16
    name bytes...
    extra bytes...
    file data...

Central Directory Header (46 + n + e + c bytes)::

    0x02014B50  signature
    version_made_by u16
    version_needed u16
    flags u16
    method u16
    mod_time u16
    mod_date u16
    crc32 u32
    compressed_size u32
    uncompressed_size u32
    name_len u16
    extra_len u16
    comment_len u16
    disk_start u16
    int_attrs u16
    ext_attrs u32
    local_offset u32
    name bytes...

End of Central Directory Record (22 bytes)::

    0x06054B50  signature
    disk_num u16
    cd_disk u16
    entries_this_disk u16
    entries_total u16
    cd_size u32
    cd_offset u32
    comment_len u16

DEFLATE Inside ZIP
==================

ZIP method 8 stores **raw RFC 1951 DEFLATE** — no zlib wrapper (no CMF/FLG
header, no Adler-32 checksum). This implementation produces RFC 1951
fixed-Huffman compressed blocks (BTYPE=01) using the ``lzss`` package for LZ77
match-finding, giving real compression without transmitting dynamic Huffman
tables.

Series
======

.. code-block:: text

    CMP00 (LZ77,    1977) — Sliding-window backreferences.
    CMP01 (LZ78,    1978) — Explicit dictionary (trie).
    CMP02 (LZSS,    1982) — LZ77 + flag bits.
    CMP03 (LZW,     1984) — LZ78 + pre-initialized alphabet; GIF.
    CMP04 (Huffman, 1952) — Entropy coding.
    CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
    CMP09 (ZIP,     1989) — DEFLATE container; universal archive. (this package)
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from typing import cast

from coding_adventures_lzss import Literal as LzssLiteral
from coding_adventures_lzss import Match as LzssMatch
from coding_adventures_lzss import encode as lzss_encode

__all__ = [
    "RAW_INFLATE_ERROR_CODES",
    "RAW_INFLATE_MAX_OUTPUT",
    "RawInflateError",
    "RawInflateResult",
    "raw_deflate",
    "raw_inflate",
    "raw_inflate_counted",
    "ZipWriter",
    "ZipReader",
    "ZipEntry",
    "zip_bytes",
    "unzip",
    "crc32",
    "dos_datetime",
    "DOS_EPOCH",
]

RAW_INFLATE_MAX_OUTPUT = 256 * 1024 * 1024
"""Absolute output ceiling for one in-memory raw RFC 1951 decode."""

RAW_INFLATE_ERROR_CODES: tuple[str, ...] = (
    "invalid-output-limit",
    "unexpected-eof",
    "reserved-block-type",
    "stored-length-mismatch",
    "huffman-oversubscribed",
    "incomplete-code-length-tree",
    "incomplete-literal-length-tree",
    "incomplete-distance-tree",
    "repeat-without-previous",
    "repeat-overrun",
    "invalid-literal-length-symbol",
    "reserved-distance-symbol",
    "invalid-back-reference",
    "output-limit-exceeded",
)


class RawInflateError(ValueError):
    """Stable, payload-blind raw inflate failure."""

    def __init__(self, code: str) -> None:
        if code not in RAW_INFLATE_ERROR_CODES:
            raise ValueError("unknown raw inflate error code")
        self.code = code
        super().__init__(code)


@dataclass(frozen=True)
class RawInflateResult:
    """Complete decoded bytes plus exact input bytes reached through BFINAL."""

    output: bytes
    bytes_consumed: int


# =============================================================================
# CRC-32
# =============================================================================
#
# CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
# It detects accidental corruption of decompressed content. It is NOT a
# cryptographic hash — for tamper-detection use AES-GCM or a signed manifest.


def _make_crc_table() -> list[int]:
    """Precomputed CRC-32 lookup table (polynomial 0xEDB88320)."""
    table: list[int] = []
    for i in range(256):
        c = i
        for _ in range(8):
            if c & 1:
                c = 0xEDB8_8320 ^ (c >> 1)
            else:
                c >>= 1
        table.append(c & 0xFFFF_FFFF)
    return table


_CRC_TABLE: list[int] = _make_crc_table()


def crc32(data: bytes, initial: int = 0) -> int:
    """Compute CRC-32 over ``data``, starting from ``initial`` (0 for a fresh hash).

    The internal pre/post XOR with 0xFFFFFFFF is handled inside this function.
    For an incremental update pass the previous result as ``initial``.

    >>> crc32(b"hello world")
    222957957
    >>> crc32(b"123456789")
    3421780262
    """
    crc = initial ^ 0xFFFF_FFFF
    for byte in data:
        crc = _CRC_TABLE[(crc ^ byte) & 0xFF] ^ (crc >> 8)
    return (crc ^ 0xFFFF_FFFF) & 0xFFFF_FFFF


# =============================================================================
# RFC 1951 DEFLATE — Bit I/O
# =============================================================================
#
# RFC 1951 packs bits LSB-first within bytes. Huffman codes are sent with the
# most-significant bit first — so before writing a Huffman code we reverse its
# bits and then write the reversed value LSB-first. Extra bits (length/distance
# extras, stored block headers) are written directly LSB-first without reversal.


def _reverse_bits(value: int, nbits: int) -> int:
    """Reverse the low ``nbits`` bits of ``value``."""
    result = 0
    for _ in range(nbits):
        result = (result << 1) | (value & 1)
        value >>= 1
    return result


class _BitWriter:
    """Writes bits into a byte stream, LSB-first."""

    def __init__(self) -> None:
        self._buf: int = 0
        self._bits: int = 0
        self._out: bytearray = bytearray()

    def write_lsb(self, value: int, nbits: int) -> None:
        """Write ``nbits`` low bits of ``value``, LSB-first."""
        self._buf |= (value & ((1 << nbits) - 1)) << self._bits
        self._bits += nbits
        while self._bits >= 8:
            self._out.append(self._buf & 0xFF)
            self._buf >>= 8
            self._bits -= 8

    def write_huffman(self, code: int, nbits: int) -> None:
        """Write a Huffman code: MSB-first logically → bit-reverse, then LSB-first."""
        self.write_lsb(_reverse_bits(code, nbits), nbits)

    def align(self) -> None:
        """Align to the next byte boundary (used before stored blocks)."""
        if self._bits > 0:
            self._out.append(self._buf & 0xFF)
            self._buf = 0
            self._bits = 0

    def write_bytes(self, data: bytes | bytearray | memoryview) -> None:
        """Append bytes after the caller has aligned the stream."""
        if self._bits != 0:
            raise ValueError("byte write requires aligned bit stream")
        self._out.extend(data)

    def finish(self) -> bytes:
        """Flush and return the accumulated bytes."""
        self.align()
        return bytes(self._out)


class _BitReader:
    """Reads bits from a bytes object, LSB-first."""

    def __init__(self, data: bytes) -> None:
        self._data = data
        self._pos = 0
        self._buf = 0
        self._bits = 0

    def _fill(self, need: int) -> bool:
        while self._bits < need:
            if self._pos >= len(self._data):
                return False
            self._buf |= self._data[self._pos] << self._bits
            self._pos += 1
            self._bits += 8
        return True

    def read_lsb(self, nbits: int) -> int | None:
        """Read ``nbits`` bits, LSB-first. Returns ``None`` on EOF."""
        if nbits == 0:
            return 0
        if not self._fill(nbits):
            return None
        val = self._buf & ((1 << nbits) - 1)
        self._buf >>= nbits
        self._bits -= nbits
        return val

    def align(self) -> None:
        """Discard partial byte, aligning to the next byte boundary."""
        discard = self._bits % 8
        if discard > 0:
            self._buf >>= discard
            self._bits -= discard


# =============================================================================
# RFC 1951 DEFLATE — Fixed Huffman Tables
# =============================================================================
#
# RFC 1951 §3.2.6 specifies fixed (pre-defined) Huffman code lengths.
# Using fixed Huffman blocks (BTYPE=01) means we never transmit code tables —
# both encoder and decoder know the tables in advance.
#
# Literal/Length code lengths:
#   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
#   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
#   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
#   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
#
# Distance codes:
#   Symbols 0–29: 5-bit codes equal to the symbol number.


def _fixed_ll_encode(sym: int) -> tuple[int, int]:
    """Return ``(code, nbits)`` for LL symbol 0–287 (RFC 1951 fixed Huffman)."""
    if 0 <= sym <= 143:
        return (0b0011_0000 + sym, 8)
    if 144 <= sym <= 255:
        return (0b1_1001_0000 + (sym - 144), 9)
    if 256 <= sym <= 279:
        return (sym - 256, 7)
    if 280 <= sym <= 287:
        return (0b1100_0000 + (sym - 280), 8)
    raise ValueError(f"fixed_ll_encode: invalid LL symbol {sym}")


# =============================================================================
# RFC 1951 DEFLATE — Length / Distance Tables
# =============================================================================
#
# Match lengths (3-258) map to LL symbols 257-285 + extra bits. The encoder's
# LZSS tokenizer currently emits at most 255 bytes; the decoder accepts 258.
# Match distances (1-32768) map to distance codes 0-29 + extra bits.

# (base_length, extra_bits) for LL symbols 257..285
_LENGTH_TABLE: list[tuple[int, int]] = [
    (3, 0),
    (4, 0),
    (5, 0),
    (6, 0),
    (7, 0),
    (8, 0),
    (9, 0),
    (10, 0),  # 257-264
    (11, 1),
    (13, 1),
    (15, 1),
    (17, 1),  # 265-268
    (19, 2),
    (23, 2),
    (27, 2),
    (31, 2),  # 269-272
    (35, 3),
    (43, 3),
    (51, 3),
    (59, 3),  # 273-276
    (67, 4),
    (83, 4),
    (99, 4),
    (115, 4),  # 277-280
    (131, 5),
    (163, 5),
    (195, 5),
    (227, 5),  # 281-284
    (258, 0),  # 285
]

# (base_offset, extra_bits) for distance codes 0..29
_DIST_TABLE: list[tuple[int, int]] = [
    (1, 0),
    (2, 0),
    (3, 0),
    (4, 0),
    (5, 1),
    (7, 1),
    (9, 2),
    (13, 2),
    (17, 3),
    (25, 3),
    (33, 4),
    (49, 4),
    (65, 5),
    (97, 5),
    (129, 6),
    (193, 6),
    (257, 7),
    (385, 7),
    (513, 8),
    (769, 8),
    (1025, 9),
    (1537, 9),
    (2049, 10),
    (3073, 10),
    (4097, 11),
    (6145, 11),
    (8193, 12),
    (12289, 12),
    (16385, 13),
    (24577, 13),
]


def _encode_length(length: int) -> tuple[int, int, int]:
    """Map match length (3–255) → ``(LL symbol, base, extra_bits)``."""
    for i in range(len(_LENGTH_TABLE) - 1, -1, -1):
        base, extra = _LENGTH_TABLE[i]
        if length >= base:
            return (257 + i, base, extra)
    raise ValueError(f"_encode_length: unreachable for length={length}")


def _encode_dist(offset: int) -> tuple[int, int, int]:
    """Map match offset (1–32768) → ``(dist code, base, extra_bits)``."""
    for code in range(len(_DIST_TABLE) - 1, -1, -1):
        base, extra = _DIST_TABLE[code]
        if offset >= base:
            return (code, base, extra)
    raise ValueError(f"_encode_dist: unreachable for offset={offset}")


# =============================================================================
# RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
# =============================================================================
#
# Strategy:
#   1. Run LZ77/LZSS match-finding (window=32768, max match=255, min=3).
#   2. Emit a single BTYPE=01 (fixed Huffman) block.
#   3. Literal bytes → fixed LL Huffman code.
#   4. Match (offset, length) → length LL code + extra bits + distance code.
#   5. End-of-block symbol (256) → fixed LL Huffman code.


_BOUNDED_DEFLATE_THRESHOLD = 4096
_DEFLATE_STORED_BLOCK_MAX = 65_535
_DEFLATE_HASH_SIZE = 1 << 16


def _triplet_key(data: memoryview, position: int) -> int:
    return (data[position] << 16) | (data[position + 1] << 8) | data[position + 2]


def _likely_repetitive(data: memoryview) -> bool:
    """Use a bounded prefix probe to choose fixed Huffman or stored framing."""
    end = min(max(0, len(data) - 2), _BOUNDED_DEFLATE_THRESHOLD)
    seen: set[int] = set()
    repeats = 0
    for position in range(end):
        key = _triplet_key(data, position)
        if key in seen:
            repeats += 1
            if repeats >= 64:
                return True
        else:
            seen.add(key)
    return False


def _write_fixed_match(bw: _BitWriter, offset: int, length: int) -> None:
    symbol, base_length, extra_length_bits = _encode_length(length)
    code, code_bits = _fixed_ll_encode(symbol)
    bw.write_huffman(code, code_bits)
    if extra_length_bits > 0:
        bw.write_lsb(length - base_length, extra_length_bits)
    distance_code, base_distance, extra_distance_bits = _encode_dist(offset)
    bw.write_huffman(distance_code, 5)
    if extra_distance_bits > 0:
        bw.write_lsb(offset - base_distance, extra_distance_bits)


def _write_fixed_streaming_block(bw: _BitWriter, data: memoryview, final: bool) -> None:
    """Emit one fixed-Huffman block with fixed-memory, linear match lookup."""
    bw.write_lsb(1 if final else 0, 1)
    bw.write_lsb(1, 1)
    bw.write_lsb(0, 1)
    positions = [-1] * _DEFLATE_HASH_SIZE
    cursor = 0
    while cursor < len(data):
        match_length = 0
        distance = 0
        if cursor + 2 < len(data):
            key = _triplet_key(data, cursor)
            slot = ((key * 2_654_435_761) & 0xFFFF_FFFF) & (_DEFLATE_HASH_SIZE - 1)
            candidate = positions[slot]
            positions[slot] = cursor
            distance = cursor - candidate
            if (
                candidate >= 0
                and distance <= 32_768
                and data[candidate] == data[cursor]
                and data[candidate + 1] == data[cursor + 1]
                and data[candidate + 2] == data[cursor + 2]
            ):
                match_length = 3
                limit = min(258, len(data) - cursor)
                while (
                    match_length < limit
                    and data[candidate + match_length] == data[cursor + match_length]
                ):
                    match_length += 1
        if match_length >= 3:
            _write_fixed_match(bw, distance, match_length)
            match_end = cursor + match_length
            for position in range(cursor + 1, match_end):
                if position + 2 < len(data):
                    key = _triplet_key(data, position)
                    slot = ((key * 2_654_435_761) & 0xFFFF_FFFF) & (
                        _DEFLATE_HASH_SIZE - 1
                    )
                    positions[slot] = position
            cursor = match_end
        else:
            code, code_bits = _fixed_ll_encode(data[cursor])
            bw.write_huffman(code, code_bits)
            cursor += 1
    end_code, end_bits = _fixed_ll_encode(256)
    bw.write_huffman(end_code, end_bits)


def _write_stored_block(bw: _BitWriter, data: memoryview, final: bool) -> None:
    bw.write_lsb(1 if final else 0, 1)
    bw.write_lsb(0, 2)
    bw.align()
    bw.write_lsb(len(data), 16)
    bw.write_lsb(len(data) ^ 0xFFFF, 16)
    bw.write_bytes(data)


def _deflate_compress_bounded(data: memoryview) -> bytes:
    """Encode large input without boxed LZSS tokens or unbounded match scans."""
    bw = _BitWriter()
    for start in range(0, len(data), _DEFLATE_STORED_BLOCK_MAX):
        block = data[start : start + _DEFLATE_STORED_BLOCK_MAX]
        final = start + len(block) == len(data)
        if _likely_repetitive(block):
            _write_fixed_streaming_block(bw, block, final)
        else:
            _write_stored_block(bw, block, final)
    return bw.finish()


def _deflate_compress(data: bytes | bytearray | memoryview) -> bytes:
    """Compress ``data`` to a raw RFC 1951 DEFLATE bit-stream (fixed Huffman).

    The output starts directly with the 3-bit block header — no zlib wrapper.
    """
    view = memoryview(data).cast("B")
    bw = _BitWriter()

    if not view:
        # Empty stored block: BFINAL=1 BTYPE=00 + LEN=0 + NLEN=0xFFFF.
        bw.write_lsb(1, 1)  # BFINAL=1
        bw.write_lsb(0, 2)  # BTYPE=00 (stored)
        bw.align()
        bw.write_lsb(0x0000, 16)  # LEN=0
        bw.write_lsb(0xFFFF, 16)  # NLEN=~0
        return bw.finish()

    if len(view) > _BOUNDED_DEFLATE_THRESHOLD:
        return _deflate_compress_bounded(view)

    # Run LZ77/LZSS tokenizer. Window=32768 covers the full RFC 1951 distance range.
    tokens = lzss_encode(bytes(view), window_size=32768, max_match=255, min_match=3)

    # Block header: BFINAL=1 (last block), BTYPE=01 (fixed Huffman).
    bw.write_lsb(1, 1)  # BFINAL
    bw.write_lsb(1, 1)  # BTYPE bit 0 = 1
    bw.write_lsb(0, 1)  # BTYPE bit 1 = 0  →  BTYPE = 01

    for tok in tokens:
        if isinstance(tok, LzssLiteral):
            code, nbits = _fixed_ll_encode(tok.byte)
            bw.write_huffman(code, nbits)
        elif isinstance(tok, LzssMatch):
            _write_fixed_match(bw, tok.offset, tok.length)

    # End-of-block symbol (256).
    eob_code, eob_bits = _fixed_ll_encode(256)
    bw.write_huffman(eob_code, eob_bits)

    return bw.finish()


# =============================================================================
# RFC 1951 DEFLATE — Decompress
# =============================================================================
#
# The decoder accepts stored, fixed-Huffman, dynamic-Huffman, and multi-block
# streams. It reads one Huffman bit at a time so exact byte consumption never
# includes an untouched suffix byte.


@dataclass(frozen=True)
class _HuffmanDecoder:
    table: dict[tuple[int, int], int]
    complete: bool
    symbol_count: int
    one_bit_count: int


def _inflate_error(code: str) -> RawInflateError:
    return RawInflateError(code)


def _read_bits(br: _BitReader, nbits: int) -> int:
    value = br.read_lsb(nbits)
    if value is None:
        raise _inflate_error("unexpected-eof")
    return value


def _build_huffman_decoder(lengths: list[int]) -> _HuffmanDecoder:
    counts = [0] * 16
    for length in lengths:
        if length < 0 or length > 15:
            raise _inflate_error("invalid-literal-length-symbol")
        if length:
            counts[length] += 1

    left = 1
    for length in range(1, 16):
        left = left * 2 - counts[length]
        if left < 0:
            raise _inflate_error("huffman-oversubscribed")

    next_code = [0] * 16
    code = 0
    for length in range(1, 16):
        code = (code + counts[length - 1]) << 1
        next_code[length] = code

    table: dict[tuple[int, int], int] = {}
    symbol_count = 0
    for symbol, length in enumerate(lengths):
        if not length:
            continue
        table[(length, next_code[length])] = symbol
        next_code[length] += 1
        symbol_count += 1
    return _HuffmanDecoder(table, left == 0, symbol_count, counts[1])


def _decode_symbol(decoder: _HuffmanDecoder, br: _BitReader, invalid_code: str) -> int:
    code = 0
    for length in range(1, 16):
        code = (code << 1) | _read_bits(br, 1)
        symbol = decoder.table.get((length, code))
        if symbol is not None:
            return symbol
    raise _inflate_error(invalid_code)


def _fixed_ll_lengths() -> list[int]:
    return [8] * 144 + [9] * 112 + [7] * 24 + [8] * 8


def _fixed_distance_lengths() -> list[int]:
    return [5] * 32


_CODE_LENGTH_ORDER = [
    16,
    17,
    18,
    0,
    8,
    7,
    9,
    6,
    10,
    5,
    11,
    4,
    12,
    3,
    13,
    2,
    14,
    1,
    15,
]


def _decode_code_lengths(
    decoder: _HuffmanDecoder, br: _BitReader, total: int
) -> list[int]:
    lengths: list[int] = []
    while len(lengths) < total:
        symbol = _decode_symbol(decoder, br, "invalid-literal-length-symbol")
        if symbol <= 15:
            lengths.append(symbol)
        elif symbol == 16:
            if not lengths:
                raise _inflate_error("repeat-without-previous")
            repeat = _read_bits(br, 2) + 3
            if repeat > total - len(lengths):
                raise _inflate_error("repeat-overrun")
            lengths.extend([lengths[-1]] * repeat)
        elif symbol in (17, 18):
            extra_bits, base = (3, 3) if symbol == 17 else (7, 11)
            repeat = _read_bits(br, extra_bits) + base
            if repeat > total - len(lengths):
                raise _inflate_error("repeat-overrun")
            lengths.extend([0] * repeat)
        else:
            raise _inflate_error("invalid-literal-length-symbol")
    return lengths


def _copy_back_reference(
    out: bytearray, distance: int, length: int, max_output: int
) -> None:
    if distance <= 0 or distance > len(out):
        raise _inflate_error("invalid-back-reference")
    if length > max_output - len(out):
        raise _inflate_error("output-limit-exceeded")
    start = len(out) - distance
    for index in range(length):
        out.append(out[start + index])


def _decode_compressed_block(
    literal_length: _HuffmanDecoder,
    distance: _HuffmanDecoder,
    br: _BitReader,
    out: bytearray,
    max_output: int,
) -> None:
    while True:
        symbol = _decode_symbol(literal_length, br, "invalid-literal-length-symbol")
        if symbol < 256:
            if len(out) >= max_output:
                raise _inflate_error("output-limit-exceeded")
            out.append(symbol)
        elif symbol == 256:
            return
        elif symbol > 285:
            raise _inflate_error("invalid-literal-length-symbol")
        else:
            base_length, length_bits = _LENGTH_TABLE[symbol - 257]
            length = base_length + _read_bits(br, length_bits)
            distance_symbol = _decode_symbol(distance, br, "reserved-distance-symbol")
            if distance_symbol >= len(_DIST_TABLE):
                raise _inflate_error("reserved-distance-symbol")
            base_distance, distance_bits = _DIST_TABLE[distance_symbol]
            back_distance = base_distance + _read_bits(br, distance_bits)
            _copy_back_reference(out, back_distance, length, max_output)


def raw_deflate(data: bytes | bytearray | memoryview) -> bytes:
    """Compress bytes as a raw RFC 1951 stream without container framing."""
    return _deflate_compress(data)


def raw_inflate_counted(
    data: bytes, max_output: int = RAW_INFLATE_MAX_OUTPUT
) -> RawInflateResult:
    """Strictly decode raw RFC 1951 bytes and report exact input consumption."""
    if type(max_output) is not int or not 0 <= max_output <= RAW_INFLATE_MAX_OUTPUT:
        raise _inflate_error("invalid-output-limit")

    br = _BitReader(data)
    out = bytearray()
    while True:
        final = _read_bits(br, 1)
        block_type = _read_bits(br, 2)

        if block_type == 0:
            br.align()
            stored_length = _read_bits(br, 16)
            complement = _read_bits(br, 16)
            if (complement ^ 0xFFFF) != stored_length:
                raise _inflate_error("stored-length-mismatch")
            if stored_length > max_output - len(out):
                raise _inflate_error("output-limit-exceeded")
            for _ in range(stored_length):
                if len(out) >= max_output:
                    raise _inflate_error("output-limit-exceeded")
                out.append(_read_bits(br, 8))
        elif block_type == 1:
            literal_length = _build_huffman_decoder(_fixed_ll_lengths())
            distance = _build_huffman_decoder(_fixed_distance_lengths())
            _decode_compressed_block(literal_length, distance, br, out, max_output)
        elif block_type == 2:
            literal_count = _read_bits(br, 5) + 257
            distance_count = _read_bits(br, 5) + 1
            code_length_count = _read_bits(br, 4) + 4
            if literal_count > 286:
                raise _inflate_error("invalid-literal-length-symbol")

            code_lengths = [0] * 19
            for index in range(code_length_count):
                code_lengths[_CODE_LENGTH_ORDER[index]] = _read_bits(br, 3)
            code_length_decoder = _build_huffman_decoder(code_lengths)
            if not code_length_decoder.complete:
                raise _inflate_error("incomplete-code-length-tree")

            lengths = _decode_code_lengths(
                code_length_decoder, br, literal_count + distance_count
            )
            literal_length = _build_huffman_decoder(lengths[:literal_count])
            if not literal_length.complete:
                raise _inflate_error("incomplete-literal-length-tree")
            distance = _build_huffman_decoder(lengths[literal_count:])
            permitted_distance = (
                distance.complete
                or (distance.symbol_count == 1 and distance.one_bit_count == 1)
                or distance.symbol_count == 0
            )
            if not permitted_distance:
                raise _inflate_error("incomplete-distance-tree")
            _decode_compressed_block(literal_length, distance, br, out, max_output)
        else:
            raise _inflate_error("reserved-block-type")

        if final:
            return RawInflateResult(bytes(out), br._pos)


def raw_inflate(data: bytes, max_output: int = RAW_INFLATE_MAX_OUTPUT) -> bytes:
    """Strictly decode a complete raw RFC 1951 stream."""
    return raw_inflate_counted(data, max_output=max_output).output


def _deflate_decompress(data: bytes) -> bytes:
    """Compatibility wrapper for the historical private decoder."""
    return raw_inflate(data)


# =============================================================================
# MS-DOS Date / Time Encoding
# =============================================================================
#
# ZIP stores timestamps in the 16-bit MS-DOS packed format inherited from FAT:
#   Time (16-bit): bits 15-11=hours, bits 10-5=minutes, bits 4-0=seconds/2
#   Date (16-bit): bits 15-9=year-1980, bits 8-5=month, bits 4-0=day
# The combined 32-bit value is (date << 16) | time.
# Year 0 in DOS time = 1980; max representable = 2107.


def dos_datetime(
    year: int, month: int, day: int, hour: int = 0, minute: int = 0, second: int = 0
) -> int:
    """Encode a timestamp into the 32-bit MS-DOS datetime used by ZIP headers.

    >>> dos_datetime(1980, 1, 1) >> 16    # date field: (0<<9)|(1<<5)|1 = 33
    33
    >>> dos_datetime(1980, 1, 1) & 0xFFFF  # time field: 0
    0
    """
    t = (hour << 11) | (minute << 5) | (second // 2)
    d = (max(0, year - 1980) << 9) | (month << 5) | day
    return ((d & 0xFFFF) << 16) | (t & 0xFFFF)


# Fixed timestamp (1980-01-01 00:00:00). Used when no real mtime is available.
# date field: (0<<9)|(1<<5)|1 = 33 = 0x0021; time = 0 → 0x00210000.
DOS_EPOCH: int = 0x0021_0000


# =============================================================================
# ZIP Write — ZipWriter
# =============================================================================
#
# ZipWriter accumulates entries in memory: for each file it writes a Local
# File Header immediately, then the (possibly compressed) data, records the
# metadata for the Central Directory, and assembles the full archive on finish().
#
# Auto-compression policy:
#   - Try DEFLATE. If compressed < original, use method=8 (DEFLATE).
#   - Otherwise use method=0 (Stored) — common for already-compressed formats.


def _pack_le16(v: int) -> bytes:
    return struct.pack("<H", v & 0xFFFF)


def _pack_le32(v: int) -> bytes:
    return struct.pack("<I", v & 0xFFFF_FFFF)


@dataclass
class _CdRecord:
    """Metadata per entry used to build the Central Directory."""

    name: bytes
    method: int
    dos_datetime: int
    crc: int
    compressed_size: int
    uncompressed_size: int
    local_offset: int
    external_attrs: int


class ZipWriter:
    """Builds a ZIP archive incrementally in memory.

    >>> w = ZipWriter()
    >>> w.add_file("hello.txt", b"hello, world!", compress=True)
    >>> w.add_directory("mydir/")
    >>> data = w.finish()  # bytes of a valid .zip file
    """

    def __init__(self) -> None:
        self._buf: bytearray = bytearray()
        self._entries: list[_CdRecord] = []

    def add_file(self, name: str, data: bytes, compress: bool = True) -> None:
        """Add a file entry.

        If ``compress`` is True, DEFLATE is attempted; the compressed form is used
        only if it is strictly smaller than the uncompressed original.
        """
        self._add_entry(name, data, compress, 0o100_644)

    def add_directory(self, name: str) -> None:
        """Add a directory entry (``name`` should end with ``'/'``)."""
        self._add_entry(name, b"", False, 0o040_755)

    def _add_entry(
        self, name: str, data: bytes, compress: bool, unix_mode: int
    ) -> None:
        name_bytes = name.encode("utf-8")
        checksum = crc32(data)
        uncompressed_size = len(data)

        if compress and data:
            compressed = _deflate_compress(data)
            if len(compressed) < len(data):
                method: int = 8
                file_data: bytes = compressed
            else:
                method = 0
                file_data = data
        else:
            method = 0
            file_data = data

        compressed_size = len(file_data)
        local_offset = len(self._buf)

        version_needed: int = 20 if method == 8 else 10
        flags: int = 0x0800  # GP flag bit 11 = UTF-8 filename

        # ── Local File Header ─────────────────────────────────────────────
        self._buf += _pack_le32(0x04034B50)  # signature
        self._buf += _pack_le16(version_needed)
        self._buf += _pack_le16(flags)
        self._buf += _pack_le16(method)
        self._buf += _pack_le16(DOS_EPOCH & 0xFFFF)  # mod_time
        self._buf += _pack_le16((DOS_EPOCH >> 16) & 0xFFFF)  # mod_date
        self._buf += _pack_le32(checksum)
        self._buf += _pack_le32(compressed_size)
        self._buf += _pack_le32(uncompressed_size)
        self._buf += _pack_le16(len(name_bytes))
        self._buf += _pack_le16(0)  # extra_field_length = 0
        self._buf += name_bytes
        self._buf += file_data

        self._entries.append(
            _CdRecord(
                name=name_bytes,
                method=method,
                dos_datetime=DOS_EPOCH,
                crc=checksum,
                compressed_size=compressed_size,
                uncompressed_size=uncompressed_size,
                local_offset=local_offset,
                external_attrs=(unix_mode << 16) & 0xFFFF_FFFF,
            )
        )

    def finish(self) -> bytes:
        """Append Central Directory and EOCD; return the complete archive bytes."""
        cd_offset = len(self._buf)
        num_entries = len(self._entries)

        # ── Central Directory ─────────────────────────────────────────────
        cd_start = len(self._buf)
        for e in self._entries:
            version_needed = 20 if e.method == 8 else 10
            self._buf += _pack_le32(0x02014B50)  # signature
            self._buf += _pack_le16(0x031E)  # version_made_by (Unix, v30)
            self._buf += _pack_le16(version_needed)
            self._buf += _pack_le16(0x0800)  # flags (UTF-8)
            self._buf += _pack_le16(e.method)
            self._buf += _pack_le16(e.dos_datetime & 0xFFFF)  # mod_time
            self._buf += _pack_le16((e.dos_datetime >> 16) & 0xFFFF)  # mod_date
            self._buf += _pack_le32(e.crc)
            self._buf += _pack_le32(e.compressed_size)
            self._buf += _pack_le32(e.uncompressed_size)
            self._buf += _pack_le16(len(e.name))
            self._buf += _pack_le16(0)  # extra_len
            self._buf += _pack_le16(0)  # comment_len
            self._buf += _pack_le16(0)  # disk_start
            self._buf += _pack_le16(0)  # internal_attrs
            self._buf += _pack_le32(e.external_attrs)
            self._buf += _pack_le32(e.local_offset)
            self._buf += e.name
        cd_size = len(self._buf) - cd_start

        # ── End of Central Directory Record ──────────────────────────────
        self._buf += _pack_le32(0x06054B50)  # signature
        self._buf += _pack_le16(0)  # disk_number
        self._buf += _pack_le16(0)  # cd_disk
        self._buf += _pack_le16(num_entries)  # entries this disk
        self._buf += _pack_le16(num_entries)  # entries total
        self._buf += _pack_le32(cd_size)
        self._buf += _pack_le32(cd_offset)
        self._buf += _pack_le16(0)  # comment_len

        return bytes(self._buf)


# =============================================================================
# ZIP Read — ZipEntry and ZipReader
# =============================================================================
#
# ZipReader uses the "EOCD-first" strategy for reliable random-access:
#   1. Scan backwards for the EOCD signature (PK\x05\x06), limit to last
#      65535 + 22 bytes (EOCD comment max = 65535).
#   2. Read CD offset and size from EOCD.
#   3. Parse all Central Directory headers into ZipEntry objects.
#   4. On read(entry): seek to Local Header via local_offset, skip variable-length
#      name + extra, read compressed data, decompress, verify CRC-32.
#
# Central Directory is authoritative for sizes/method; Local Header is consulted
# only to get name_len + extra_len for the skip calculation.


def _read_le16(data: bytes, offset: int) -> int | None:
    """Read a little-endian u16 from ``data`` at ``offset``. Returns None on OOB."""
    if offset + 2 > len(data):
        return None
    return cast(int, struct.unpack_from("<H", data, offset)[0])


def _read_le32(data: bytes, offset: int) -> int | None:
    """Read a little-endian u32 from ``data`` at ``offset``. Returns None on OOB."""
    if offset + 4 > len(data):
        return None
    return cast(int, struct.unpack_from("<I", data, offset)[0])


@dataclass
class ZipEntry:
    """Metadata for a single entry inside a ZIP archive."""

    name: str
    """File name (UTF-8)."""
    size: int
    """Uncompressed size in bytes."""
    compressed_size: int
    """Compressed size in bytes."""
    method: int
    """Compression method: 0 = Stored, 8 = DEFLATE."""
    crc32: int
    """CRC-32 of the uncompressed content."""
    is_directory: bool
    """True if this entry is a directory (name ends with '/')."""
    local_offset: int
    """Byte offset of the Local File Header within the archive."""


class ZipReader:
    """Reads entries from an in-memory ZIP archive.

    >>> w = ZipWriter()
    >>> w.add_file("f.txt", b"hello", compress=True)
    >>> data = w.finish()
    >>> r = ZipReader(data)
    >>> for e in r.entries():
    ...     print(e.name, e.size)
    f.txt 5
    """

    def __init__(self, data: bytes) -> None:
        """Parse an in-memory ZIP archive.

        Raises ``ValueError`` if no valid EOCD record is found or the archive
        is structurally malformed.
        """
        self._data = data
        eocd_offset = self._find_eocd()
        if eocd_offset is None:
            raise ValueError("zip: no End of Central Directory record found")

        cd_offset = _read_le32(data, eocd_offset + 16)
        cd_size = _read_le32(data, eocd_offset + 12)
        if cd_offset is None or cd_size is None:
            raise ValueError("zip: EOCD too short")
        if cd_offset + cd_size > len(data):
            raise ValueError(
                f"zip: Central Directory [{cd_offset}, {cd_offset + cd_size}) "
                f"out of bounds (file size {len(data)})"
            )

        self._entries: list[ZipEntry] = []
        pos = cd_offset
        while pos + 4 <= cd_offset + cd_size:
            sig = _read_le32(data, pos)
            if sig != 0x02014B50:
                break  # end of CD or padding

            method = _read_le16(data, pos + 10)
            crc32_val = _read_le32(data, pos + 16)
            compressed_size = _read_le32(data, pos + 20)
            size = _read_le32(data, pos + 24)
            name_len = _read_le16(data, pos + 28)
            extra_len = _read_le16(data, pos + 30)
            comment_len = _read_le16(data, pos + 32)
            local_offset = _read_le32(data, pos + 42)

            if any(
                v is None
                for v in [
                    method,
                    crc32_val,
                    compressed_size,
                    size,
                    name_len,
                    extra_len,
                    comment_len,
                    local_offset,
                ]
            ):
                raise ValueError("zip: CD entry truncated")

            # Narrowing for mypy — all None checks done above.
            assert method is not None
            assert crc32_val is not None
            assert compressed_size is not None
            assert size is not None
            assert name_len is not None
            assert extra_len is not None
            assert comment_len is not None
            assert local_offset is not None

            name_start = pos + 46
            name_end = name_start + name_len
            if name_end > len(data):
                raise ValueError("zip: CD entry name out of bounds")

            name = data[name_start:name_end].decode("utf-8", errors="replace")
            self._entries.append(
                ZipEntry(
                    name=name,
                    size=size,
                    compressed_size=compressed_size,
                    method=method,
                    crc32=crc32_val,
                    is_directory=name.endswith("/"),
                    local_offset=local_offset,
                )
            )
            pos = name_end + extra_len + comment_len

    def entries(self) -> list[ZipEntry]:
        """Return all entries in the archive (files and directories)."""
        return list(self._entries)

    def read(self, entry: ZipEntry) -> bytes:
        """Decompress and return the data for ``entry``. Verifies CRC-32.

        Raises ``ValueError`` on CRC mismatch, unsupported method, or corrupt data.
        """
        if entry.is_directory:
            return b""

        # Reject encrypted entries (GP flag bit 0).
        local_flags = _read_le16(self._data, entry.local_offset + 6)
        if local_flags is None:
            raise ValueError("zip: local header out of bounds")
        if local_flags & 1:
            raise ValueError(f"zip: entry '{entry.name}' is encrypted; not supported")

        # Skip Local Header to reach file data.
        # Re-read name_len + extra_len from the Local Header; they may differ
        # from CD values.
        lh_name_len = _read_le16(self._data, entry.local_offset + 26)
        lh_extra_len = _read_le16(self._data, entry.local_offset + 28)
        if lh_name_len is None or lh_extra_len is None:
            raise ValueError("zip: local header truncated")

        data_start = entry.local_offset + 30 + lh_name_len + lh_extra_len
        data_end = data_start + entry.compressed_size
        if data_end > len(self._data):
            raise ValueError(
                f"zip: entry '{entry.name}' data "
                f"[{data_start}, {data_end}) out of bounds"
            )

        compressed = self._data[data_start:data_end]

        if entry.method == 0:
            decompressed = bytes(compressed)
        elif entry.method == 8:
            result = raw_inflate_counted(compressed, max_output=entry.size)
            if result.bytes_consumed != len(compressed):
                raise ValueError("zip: compressed payload contains trailing bytes")
            decompressed = result.output
        else:
            raise ValueError(
                f"zip: unsupported compression method {entry.method} for '{entry.name}'"
            )

        if len(decompressed) != entry.size:
            raise ValueError("zip: uncompressed size does not match the directory")

        # Verify CRC-32.
        actual_crc = crc32(decompressed)
        if actual_crc != entry.crc32:
            raise ValueError(
                f"zip: CRC-32 mismatch for '{entry.name}': "
                f"expected {entry.crc32:08X}, got {actual_crc:08X}"
            )
        return decompressed

    def read_by_name(self, name: str) -> bytes:
        """Find an entry by name and return its decompressed data."""
        for e in self._entries:
            if e.name == name:
                return self.read(e)
        raise ValueError(f"zip: entry '{name}' not found")

    def _find_eocd(self) -> int | None:
        """Scan backwards for EOCD signature 0x06054B50.

        The EOCD record is at most 22 + 65535 bytes from the end. We limit the
        scan to prevent unbounded searches on crafted files.
        """
        eocd_sig = 0x06054B50
        max_comment = 65535
        eocd_min_size = 22
        data = self._data

        if len(data) < eocd_min_size:
            return None

        scan_start = max(0, len(data) - eocd_min_size - max_comment)
        for i in range(len(data) - eocd_min_size, scan_start - 1, -1):
            if _read_le32(data, i) == eocd_sig:
                comment_len = _read_le16(data, i + 20)
                if comment_len is not None and i + eocd_min_size + comment_len == len(
                    data
                ):
                    return i
        return None


# =============================================================================
# Convenience Functions
# =============================================================================


def zip_bytes(entries: list[tuple[str, bytes]], compress: bool = True) -> bytes:
    """Compress a list of ``(name, data)`` pairs into a ZIP archive.

    Each file is compressed with DEFLATE if it reduces size; otherwise stored.

    >>> archive = zip_bytes([("hello.txt", b"hello, world!")])
    >>> len(archive) > 0
    True
    """
    w = ZipWriter()
    for name, data in entries:
        w.add_file(name, data, compress)
    return w.finish()


def unzip(data: bytes) -> dict[str, bytes]:
    """Decompress all file entries from a ZIP archive.

    Returns a dict mapping ``name → data``. Directory entries are skipped.

    >>> archive = zip_bytes([("f.txt", b"hello")])
    >>> unzip(archive)["f.txt"]
    b'hello'
    """
    reader = ZipReader(data)
    return {
        entry.name: reader.read(entry)
        for entry in reader.entries()
        if not entry.is_directory
    }
