# =============================================================================
# CodingAdventures.Deflate
# =============================================================================
#
# DEFLATE (1996, RFC 1951) lossless compression algorithm.
# Part of the CMP compression series in the coding-adventures monorepo.
#
# What Is DEFLATE?
# ----------------
#
# DEFLATE is the dominant general-purpose lossless compression algorithm. It
# combines two complementary techniques:
#
#   Pass 1 — LZSS tokenization (CMP02): replaces repeated substrings with
#            back-references into a sliding window (window_size=4096 bytes,
#            match lengths 3–255).
#
#   Pass 2 — Dual Huffman coding (CMP04/DT27): entropy-codes the resulting
#            token stream using TWO canonical Huffman trees:
#              LL tree:   covers literals (0–255), end-of-data (256),
#                         and length codes (257–284).
#              Dist tree: covers distance codes (0–23, for offsets 1–4096).
#
# Together, they achieve compression that neither technique can match alone:
# LZ removes patterns; Huffman removes symbol-frequency bias in the residual.
#
# Historical Context
# ------------------
#
# Phil Katz designed DEFLATE for PKZIP in 1989. RFC 1951 (Peter Deutsch, 1996)
# formalised the specification. The same year, Deutsch and Jean-Loup Gailly
# published zlib as a reference implementation. DEFLATE powers ZIP, gzip, PNG,
# and HTTP/2 HPACK header compression to this day.
#
# Phil Katz released the PKZIP specification publicly, which is why DEFLATE
# became an open standard rather than a proprietary format. He died in 2000.
#
# Two-Pass Design
# ---------------
#
# Pass 1 — LZSS tokenization:
#   "ABCABCABC" → [Lit('A'), Lit('B'), Lit('C'), Match(offset=3, length=6)]
#   4 tokens instead of 9 bytes; repetition eliminated.
#
# Pass 2 — Huffman coding of the token stream:
#   Frequent tokens get shorter codes, rare tokens get longer codes.
#   Exploits skewed frequency distribution between literals and match codes.
#
# The Expanded LL Alphabet (285 symbols)
# ----------------------------------------
#
# DEFLATE combines literals and length information into one "LL" alphabet:
#
#   Symbols 0–255:    Literal byte values (raw bytes from the input).
#   Symbol  256:      End-of-data marker (replaces original_length counting).
#   Symbols 257–284:  Length codes — each covers a range of match lengths.
#                     Extra bits appended after the Huffman code select the
#                     exact length within the range.
#
# Length codes with extra bits (symbols 257–284):
#
#   Symbol  Extra  Base  Max
#   ──────  ─────  ────  ───
#   257      0      3    3
#   258      0      4    4
#   ...      ...   ...  ...
#   265      1     11   12
#   266      1     13   14
#   ...
#   284      5    227   255
#
# Distance Codes (0–23, separate Huffman tree)
# ---------------------------------------------
#
# The LZSS back-reference offset (distance) ranges from 1 to 4096.
# 24 distance codes cover this range, again with extra bits:
#
#   Code  Extra  Base  Max
#   ────  ─────  ────  ────
#    0     0      1    1
#    1     0      2    2
#    ...
#    4     1      5    6
#    5     1      7    8
#    ...
#   23    10    3073  4096
#
# Wire Format (CMP05)
# -------------------
#
#   [4B] original_length  — big-endian uint32
#   [2B] ll_entry_count   — big-endian uint16
#   [2B] dist_entry_count — big-endian uint16 (0 if no matches)
#   [ll_entry_count × 3B] LL table:   (symbol BE uint16, code_length uint8)
#                          sorted by (code_length ASC, symbol ASC)
#   [dist_entry_count × 3B] dist table: same format
#   [remaining bytes]      LSB-first bit stream
#
# The key difference from CMP04: table entries are 3 bytes (symbol=uint16)
# instead of 2 bytes (symbol=uint8), because the LL alphabet goes up to 284
# which exceeds uint8 range (0–255).
#
# Dependencies
# ------------
#
#   coding-adventures-lzss       (CMP02) — LZSS tokenizer
#   coding-adventures-huffman-tree (DT27) — Huffman tree construction
#
# =============================================================================

from __future__ import annotations

import struct
from collections import Counter

from coding_adventures_huffman_tree import HuffmanTree
from coding_adventures_lzss import Literal, Match, encode as lzss_encode

__all__ = ["compress", "decompress"]


# ---------------------------------------------------------------------------
# Length code table (symbols 257–284)
# ---------------------------------------------------------------------------
#
# Each length symbol encodes a range of match lengths. The exact length is
# determined by the Huffman symbol (the base) plus a small integer transmitted
# as raw extra bits after the Huffman code.
#
# Layout: LENGTH_TABLE[i] = (symbol, base_length, extra_bits)
# Symbol 257 covers length 3, symbol 258 covers length 4, etc.
# Symbols with extra_bits > 0 cover a range of 2^extra_bits lengths.

_LENGTH_TABLE: list[tuple[int, int, int]] = [
    # (symbol, base_length, extra_bits)
    (257,   3, 0),
    (258,   4, 0),
    (259,   5, 0),
    (260,   6, 0),
    (261,   7, 0),
    (262,   8, 0),
    (263,   9, 0),
    (264,  10, 0),
    (265,  11, 1),
    (266,  13, 1),
    (267,  15, 1),
    (268,  17, 1),
    (269,  19, 2),
    (270,  23, 2),
    (271,  27, 2),
    (272,  31, 2),
    (273,  35, 3),
    (274,  43, 3),
    (275,  51, 3),
    (276,  59, 3),
    (277,  67, 4),
    (278,  83, 4),
    (279,  99, 4),
    (280, 115, 4),
    (281, 131, 5),
    (282, 163, 5),
    (283, 195, 5),
    (284, 227, 5),
]

# Build fast lookup: base and extra_bits by symbol
_LENGTH_BASE: dict[int, int] = {sym: base for sym, base, _ in _LENGTH_TABLE}
_LENGTH_EXTRA: dict[int, int] = {sym: extra for sym, _, extra in _LENGTH_TABLE}


# ---------------------------------------------------------------------------
# Distance code table (codes 0–23)
# ---------------------------------------------------------------------------
#
# The sliding-window offset (1–4096) is encoded as a distance code (0–23)
# followed by raw extra bits. Analogous to the length code scheme.
#
# DIST_TABLE[i] = (code, base_distance, extra_bits)

_DIST_TABLE: list[tuple[int, int, int]] = [
    # (code, base_dist, extra_bits)
    ( 0,    1, 0),
    ( 1,    2, 0),
    ( 2,    3, 0),
    ( 3,    4, 0),
    ( 4,    5, 1),
    ( 5,    7, 1),
    ( 6,    9, 2),
    ( 7,   13, 2),
    ( 8,   17, 3),
    ( 9,   25, 3),
    (10,   33, 4),
    (11,   49, 4),
    (12,   65, 5),
    (13,   97, 5),
    (14,  129, 6),
    (15,  193, 6),
    (16,  257, 7),
    (17,  385, 7),
    (18,  513, 8),
    (19,  769, 8),
    (20, 1025, 9),
    (21, 1537, 9),
    (22, 2049, 10),
    (23, 3073, 10),
]

_DIST_BASE: dict[int, int]  = {code: base for code, base, _ in _DIST_TABLE}
_DIST_EXTRA: dict[int, int] = {code: extra for code, _, extra in _DIST_TABLE}


# ---------------------------------------------------------------------------
# Helper: length_symbol(length) → LL symbol (257–284)
# ---------------------------------------------------------------------------

def _length_symbol(length: int) -> int:
    """Map a match length (3–255) to the corresponding LL alphabet symbol (257–284).

    Each length symbol covers a range of lengths via extra bits. We scan the
    table from low to high and return the first symbol whose range includes
    the target length.

    Example:
        length=3  → symbol 257 (base=3,  extra=0, range=[3,3])
        length=13 → symbol 266 (base=13, extra=1, range=[13,14])
        length=50 → symbol 274 (base=43, extra=3, range=[43,50])
    """
    for sym, base, extra in _LENGTH_TABLE:
        if length <= base + (1 << extra) - 1:
            return sym
    return 284  # maximum symbol


# ---------------------------------------------------------------------------
# Helper: dist_code(offset) → distance code (0–23)
# ---------------------------------------------------------------------------

def _dist_code(offset: int) -> int:
    """Map a back-reference offset (1–4096) to a distance code (0–23).

    Example:
        offset=1    → code 0  (base=1,   extra=0)
        offset=5    → code 4  (base=5,   extra=1, extra_value=0)
        offset=4096 → code 23 (base=3073, extra=10, extra_value=1023)
    """
    for code, base, extra in _DIST_TABLE:
        if offset <= base + (1 << extra) - 1:
            return code
    return 23  # maximum code


# ---------------------------------------------------------------------------
# Bit I/O helpers
# ---------------------------------------------------------------------------

def _pack_bits_lsb_first(bits: str) -> bytes:
    """Pack a string of '0'/'1' characters into bytes, LSB-first.

    The first bit in the string occupies bit 0 (LSB) of the first byte.
    Bits fill a byte from LSB to MSB before spilling into the next byte.
    The final byte is zero-padded in its high bits if needed.

    LSB-first packing is the same convention used by GIF, LZW (CMP03),
    Huffman (CMP04), and DEFLATE. It means bit 0 of each code is always
    the least-significant bit of the current byte position.

    Example — packing "10110" (5 bits):
      Byte 0: bit 0 = '1' → 0b???????1
              bit 1 = '0' → 0b??????01
              bit 2 = '1' → 0b?????101
              bit 3 = '1' → 0b????1101
              bit 4 = '0' → 0b???01101
              (bits 5-7 zero-padded) → 0b00001101 = 0x0D
    """
    output: list[int] = []
    buffer = 0
    bit_pos = 0
    for b in bits:
        buffer |= int(b) << bit_pos
        bit_pos += 1
        if bit_pos == 8:
            output.append(buffer)
            buffer = 0
            bit_pos = 0
    if bit_pos > 0:
        output.append(buffer)
    return bytes(output)


def _unpack_bits_lsb_first(data: bytes) -> str:
    """Expand bytes into a bit string, reading each byte LSB-first.

    The inverse of _pack_bits_lsb_first. Bit 0 (LSB) of each byte becomes
    the next character '0' or '1' in the output string.

    The caller is responsible for consuming only as many bits as are
    meaningful; zero-padding bits at the end of the last byte are ignored
    naturally because decoding stops at the end-of-data symbol (256).
    """
    bits = ""
    for byte_val in data:
        for i in range(8):
            bits += str((byte_val >> i) & 1)
    return bits


# ---------------------------------------------------------------------------
# Canonical code reconstruction (shared by compress and decompress)
# ---------------------------------------------------------------------------

def _reconstruct_canonical_codes(
    lengths: list[tuple[int, int]],
) -> dict[str, int]:
    """Reconstruct a bitstring→symbol map from sorted (symbol, code_length) pairs.

    Given the table stored in the wire format — pairs sorted by
    (code_length ASC, symbol ASC) — this function assigns bit strings
    deterministically using the canonical algorithm:

        code = 0
        prev_len = first entry's code_length
        for each (symbol, code_length):
            if code_length > prev_len:
                code <<= (code_length - prev_len)
            assign binary string of 'code' padded to 'code_length' bits
            code += 1

    The result is a map {bit_string → symbol} used by the decoder.
    Single-symbol edge case: assign code "0".
    """
    if not lengths:
        return {}
    if len(lengths) == 1:
        return {"0": lengths[0][0]}

    result: dict[str, int] = {}
    code = 0
    prev_len = lengths[0][1]
    for symbol, code_len in lengths:
        if code_len > prev_len:
            code <<= (code_len - prev_len)
        bit_str = format(code, f"0{code_len}b")
        result[bit_str] = symbol
        code += 1
        prev_len = code_len
    return result


# ---------------------------------------------------------------------------
# Public API: compress
# ---------------------------------------------------------------------------

def compress(
    data: bytes | bytearray,
    window_size: int = 4096,
    max_match: int = 255,
    min_match: int = 3,
) -> bytes:
    """Compress *data* using DEFLATE and return CMP05 wire-format bytes.

    Algorithm
    ---------
    Pass 1 — LZSS tokenization:
        Call lzss.encode() with window_size=4096, max_match=255, min_match=3.
        This produces a list of Literal(byte) and Match(offset, length) tokens.

    Pass 2a — Frequency counting:
        Walk the token list and tally:
          - ll_freq[byte] for each Literal(byte)
          - ll_freq[length_symbol(length)] for each Match
          - ll_freq[256] += 1 for the end-of-data marker
          - dist_freq[dist_code(offset)] for each Match

    Pass 2b — Huffman tree construction:
        Build canonical Huffman trees via DT27 for both ll_freq and dist_freq.
        If there are no matches, dist_freq is empty and dist_table is empty.

    Pass 2c — Bit stream assembly:
        For each token:
          Literal(byte):   emit LL code for byte
          Match(off, len): emit LL code for length_symbol(len)
                           + extra_bits for exact length (LSB-first)
                           + dist code for dist_code(off)
                           + extra_bits for exact offset (LSB-first)
        Emit LL code for 256 (end-of-data).
        Pack bits LSB-first into bytes.

    Wire format assembly:
        Header (8 bytes): original_length, ll_entry_count, dist_entry_count
        LL table: ll_entry_count × 3 bytes (symbol uint16 BE, code_length uint8)
        Dist table: dist_entry_count × 3 bytes (same format)
        Bit stream: packed bytes

    Parameters
    ----------
    data:
        The raw bytes to compress.
    window_size:
        LZSS search window size (default 4096).
    max_match:
        Maximum match length (default 255).
    min_match:
        Minimum match length (default 3).

    Returns
    -------
    bytes
        Compressed data in CMP05 wire format.

    Edge Cases
    ----------
    - Empty input: returns 8-byte header (original_length=0, ll_entry_count=1
      for the lone end-of-data symbol, dist_entry_count=0).
    - All distinct bytes (no matches): dist table omitted (dist_entry_count=0).
    - Single distinct byte: DT27 assigns code "0"; all occurrences encode to
      one bit each.
    """
    data = bytes(data)
    original_length = len(data)

    if original_length == 0:
        # Empty input: LL tree has only the end-of-data symbol (256).
        # Single-symbol tree → code length 1.
        header = struct.pack(">IHH", 0, 1, 0)
        ll_table_bytes = struct.pack(">HB", 256, 1)
        # Bit stream: just the end-of-data code "0" → 1 bit → 0x00
        bit_bytes = bytes([0x00])
        return header + ll_table_bytes + bit_bytes

    # ── Pass 1: LZSS tokenization ────────────────────────────────────────────
    tokens = lzss_encode(data, window_size, max_match, min_match)

    # ── Pass 2a: Tally frequencies ───────────────────────────────────────────
    ll_freq: Counter[int] = Counter()
    dist_freq: Counter[int] = Counter()

    for tok in tokens:
        if isinstance(tok, Literal):
            ll_freq[tok.byte] += 1
        else:  # Match
            sym = _length_symbol(tok.length)
            ll_freq[sym] += 1
            dc = _dist_code(tok.offset)
            dist_freq[dc] += 1

    # End-of-data marker always gets frequency 1.
    ll_freq[256] += 1

    # ── Pass 2b: Build canonical Huffman trees via DT27 ──────────────────────
    ll_tree = HuffmanTree.build(list(ll_freq.items()))
    ll_table = ll_tree.canonical_code_table()  # {symbol: bit_string}

    dist_table: dict[int, str] = {}
    if dist_freq:
        dist_tree = HuffmanTree.build(list(dist_freq.items()))
        dist_table = dist_tree.canonical_code_table()

    # ── Pass 2c: Encode token stream to bit string ───────────────────────────
    bits = ""
    for tok in tokens:
        if isinstance(tok, Literal):
            bits += ll_table[tok.byte]
        else:  # Match(offset, length)
            sym = _length_symbol(tok.length)
            extra_bits = _LENGTH_EXTRA[sym]
            extra_val = tok.length - _LENGTH_BASE[sym]

            bits += ll_table[sym]
            if extra_bits > 0:
                # Emit extra_bits raw bits LSB-first.
                # format(val, "0Nb")[::-1] reverses to get LSB-first order.
                bits += format(extra_val, f"0{extra_bits}b")[::-1]

            dc = _dist_code(tok.offset)
            dist_extra_bits = _DIST_EXTRA[dc]
            dist_extra_val = tok.offset - _DIST_BASE[dc]

            bits += dist_table[dc]
            if dist_extra_bits > 0:
                bits += format(dist_extra_val, f"0{dist_extra_bits}b")[::-1]

    # End-of-data symbol.
    bits += ll_table[256]

    bit_bytes = _pack_bits_lsb_first(bits)

    # ── Assemble wire format ─────────────────────────────────────────────────
    # Code-length tables sorted by (code_length, symbol) ascending.
    ll_lengths = sorted(
        ((sym, len(code)) for sym, code in ll_table.items()),
        key=lambda p: (p[1], p[0]),
    )
    dist_lengths = sorted(
        ((sym, len(code)) for sym, code in dist_table.items()),
        key=lambda p: (p[1], p[0]),
    )

    header = struct.pack(">IHH", original_length, len(ll_lengths), len(dist_lengths))
    ll_bytes = b"".join(struct.pack(">HB", s, ln) for s, ln in ll_lengths)
    dt_bytes = b"".join(struct.pack(">HB", s, ln) for s, ln in dist_lengths)

    return header + ll_bytes + dt_bytes + bit_bytes


# ---------------------------------------------------------------------------
# Public API: decompress
# ---------------------------------------------------------------------------

def decompress(data: bytes | bytearray) -> bytes:
    """Decompress CMP05 wire-format *data* and return the original bytes.

    Algorithm
    ---------
    1. Parse the 8-byte header: original_length, ll_entry_count, dist_entry_count.
    2. Parse ll_entry_count × 3-byte entries → (symbol, code_length) pairs.
    3. Parse dist_entry_count × 3-byte entries → (symbol, code_length) pairs.
    4. Reconstruct canonical codes for both trees.
    5. Unpack the bit stream LSB-first.
    6. Decode token stream:
         - Read Huffman symbol from LL tree.
         - If < 256: output the literal byte.
         - If == 256: end-of-data, stop.
         - If 257–284: read extra_bits raw bits for exact length,
                       read Huffman symbol from dist tree,
                       read extra_bits raw bits for exact offset,
                       copy length bytes from output[-offset] byte-by-byte
                       (byte-by-byte to handle overlapping matches correctly).

    Parameters
    ----------
    data:
        Compressed bytes produced by :func:`compress`.

    Returns
    -------
    bytes
        The original, uncompressed data.

    Notes
    -----
    Overlapping matches (offset < length) encode runs. Example:
        output = [A], Match(offset=1, length=6) → AAAAAAA
    This only works correctly when copying byte-by-byte, NOT with bulk copy.
    """
    data = bytes(data)

    if len(data) < 8:
        return b""

    original_length, ll_entry_count, dist_entry_count = struct.unpack(">IHH", data[:8])

    if original_length == 0:
        return b""

    offset = 8

    # Parse LL code-length table.
    ll_lengths: list[tuple[int, int]] = []
    for _ in range(ll_entry_count):
        sym = struct.unpack(">H", data[offset : offset + 2])[0]
        code_len = data[offset + 2]
        ll_lengths.append((sym, code_len))
        offset += 3

    # Parse distance code-length table.
    dist_lengths: list[tuple[int, int]] = []
    for _ in range(dist_entry_count):
        sym = struct.unpack(">H", data[offset : offset + 2])[0]
        code_len = data[offset + 2]
        dist_lengths.append((sym, code_len))
        offset += 3

    # Reconstruct canonical code tables (bit_string → symbol).
    ll_code_table = _reconstruct_canonical_codes(ll_lengths)
    dist_code_table = _reconstruct_canonical_codes(dist_lengths)

    # Unpack bit stream.
    bits = _unpack_bits_lsb_first(data[offset:])
    bit_pos = 0

    def read_bits(n: int) -> int:
        """Read n raw bits LSB-first, return as integer.

        Bit at bit_pos+0 is the LSB (weight 2^0),
        bit at bit_pos+1 has weight 2^1, etc.
        """
        nonlocal bit_pos
        val = 0
        for i in range(n):
            val |= int(bits[bit_pos + i]) << i
        bit_pos += n
        return val

    def next_huffman_symbol(code_table: dict[str, int]) -> int:
        """Decode one Huffman symbol by reading bits until a prefix match."""
        nonlocal bit_pos
        acc = ""
        while True:
            acc += bits[bit_pos]
            bit_pos += 1
            if acc in code_table:
                return code_table[acc]

    # Decode token stream.
    output: list[int] = []
    while True:
        ll_sym = next_huffman_symbol(ll_code_table)

        if ll_sym == 256:
            break  # end-of-data

        elif ll_sym < 256:
            output.append(ll_sym)  # literal byte

        else:  # ll_sym 257–284: length code
            extra = _LENGTH_EXTRA[ll_sym]
            length = _LENGTH_BASE[ll_sym] + read_bits(extra)

            dist_sym = next_huffman_symbol(dist_code_table)
            dextra = _DIST_EXTRA[dist_sym]
            dist_offset = _DIST_BASE[dist_sym] + read_bits(dextra)

            # Copy byte-by-byte from the back-reference position.
            # byte-by-byte is required for overlapping matches.
            start = len(output) - dist_offset
            for _ in range(length):
                output.append(output[start])
                start += 1

    return bytes(output)
