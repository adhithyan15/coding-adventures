# =============================================================================
# CodingAdventures.HuffmanCompression
# =============================================================================
#
# Huffman (1952) lossless compression algorithm.
# Part of the CMP compression series in the coding-adventures monorepo.
#
# What Is Huffman Compression?
# ----------------------------
#
# Huffman coding is an **entropy coding** algorithm: it assigns variable-length,
# prefix-free binary codes to symbols based on their frequency of occurrence.
# Frequent symbols get short codes; rare symbols get long codes. The resulting
# code is provably optimal — no other prefix-free code can achieve a smaller
# expected bit-length for the same symbol distribution.
#
# Unlike the LZ-family algorithms (CMP00–CMP03) which exploit **repetition**
# (duplicate substrings), Huffman coding exploits **symbol statistics**. It
# works on individual symbol frequencies, not patterns of repetition. This
# makes it complementary to LZ compression and explains why DEFLATE (CMP05)
# combines both: LZ77 to eliminate repeated substrings, then Huffman to
# optimally encode the remaining symbol stream.
#
# Dependency on DT27
# ------------------
#
# This package does NOT build its own Huffman tree. It imports
# `coding-adventures-huffman-tree` (DT27) and delegates all tree construction
# and code derivation to that package. This mirrors the pattern used by LZ78
# (CMP01) which delegates trie operations to `trie` (DT13).
#
#   CMP01 (LZ78)    →  uses DT13 (Trie)        for dictionary management
#   CMP04 (Huffman) →  uses DT27 (HuffmanTree)  for code construction/decode
#
# Wire Format (CMP04)
# -------------------
#
#   Bytes 0–3:    original_length  (big-endian uint32)
#   Bytes 4–7:    symbol_count     (big-endian uint32) — number of distinct bytes
#   Bytes 8–8+2N: code-lengths table — N entries, each 2 bytes:
#                   [0] symbol value  (uint8, 0–255)
#                   [1] code length   (uint8, 1–16)
#                 Sorted by (code_length, symbol_value) ascending.
#   Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
#
# The Series: CMP00 -> CMP05
# --------------------------
#
#   CMP00 (LZ77,    1977) — Sliding-window backreferences.
#   CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
#   CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals.
#   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
#   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE. (this module)
#   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
# =============================================================================

from __future__ import annotations

import struct
from collections import Counter

from coding_adventures_huffman_tree import HuffmanTree

__all__ = ["compress", "decompress"]


# ---------------------------------------------------------------------------
# Bit I/O helpers
# ---------------------------------------------------------------------------

def _pack_bits_lsb_first(bits: str) -> bytes:
    """Pack a bit string into bytes, filling each byte from LSB upward.

    This is the same convention used by LZW (CMP03) and GIF: the first bit
    of the stream occupies bit 0 (the least-significant bit) of the first byte.

    Example — packing "000101011" (9 bits):
      Byte 0: bits[0..7] → 0b10101000 = 0xA8
        bit 0 ('0') → byte bit 0
        bit 1 ('0') → byte bit 1
        bit 2 ('0') → byte bit 2
        bit 3 ('1') → byte bit 3
        bit 4 ('0') → byte bit 4
        bit 5 ('1') → byte bit 5
        bit 6 ('0') → byte bit 6
        bit 7 ('1') → byte bit 7
      Byte 1: bit[8] ('1') → 0b00000001 = 0x01
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
        # Final partial byte — remaining high bits are zero-padded.
        output.append(buffer)
    return bytes(output)


def _unpack_bits_lsb_first(data: bytes) -> str:
    """Unpack bytes into a bit string, reading each byte from LSB upward.

    Mirrors _pack_bits_lsb_first exactly.  The decoder reads only the bits it
    needs (tracked by original_length) and ignores any zero-padding in the
    final byte.
    """
    bits = ""
    for byte_val in data:
        for i in range(8):
            bits += str((byte_val >> i) & 1)
    return bits


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def compress(data: bytes | bytearray) -> bytes:
    """Compress *data* using Huffman coding and return CMP04 wire-format bytes.

    Algorithm
    ---------
    1. Count symbol frequencies (byte histogram).
    2. Build a Huffman tree via DT27 (HuffmanTree.build).
    3. Obtain canonical codes via DT27 (tree.canonical_code_table).
    4. Build the code-lengths table for the wire-format header: pairs of
       (symbol, code_length) sorted by (code_length, symbol_value).
    5. Encode the input byte-by-byte using the canonical code table.
    6. Pack the resulting bit string LSB-first into bytes.
    7. Assemble header + code-lengths table + bit stream.

    Parameters
    ----------
    data:
        The raw bytes to compress.

    Returns
    -------
    bytes
        Compressed data in CMP04 wire format.

    Edge cases
    ----------
    - Empty input: returns an 8-byte header with original_length=0,
      symbol_count=0, and no bit data.
    - Single distinct byte: DT27 assigns it code "0"; each occurrence
      encodes to 1 bit.
    """
    data = bytes(data)
    original_length = len(data)

    # Empty input — 8-byte header only, no bit stream.
    if original_length == 0:
        return struct.pack(">II", 0, 0)

    # Step 1: Count frequencies.
    freq = Counter(data)

    # Step 2: Build Huffman tree via DT27.
    tree = HuffmanTree.build(list(freq.items()))

    # Step 3: Canonical code table {symbol: bit_string}.
    table = tree.canonical_code_table()

    # Step 4: Code-lengths list sorted by (length, symbol) for the header.
    lengths: list[tuple[int, int]] = sorted(
        ((sym, len(bits)) for sym, bits in table.items()),
        key=lambda p: (p[1], p[0]),
    )

    # Step 5: Encode each byte using its canonical code.
    bit_string = "".join(table[b] for b in data)

    # Step 6: Pack bits LSB-first.
    bit_bytes = _pack_bits_lsb_first(bit_string)

    # Step 7: Assemble wire format.
    #   header:             original_length (4B) + symbol_count (4B)
    #   code-lengths table: symbol_count × 2 bytes
    #   bit stream:         variable
    header = struct.pack(">II", original_length, len(lengths))
    code_table_bytes = b"".join(bytes([sym, length]) for sym, length in lengths)
    return header + code_table_bytes + bit_bytes


def decompress(data: bytes | bytearray) -> bytes:
    """Decompress CMP04 wire-format *data* and return the original bytes.

    Algorithm
    ---------
    1. Parse the 8-byte header: original_length and symbol_count.
    2. Parse the code-lengths table (symbol_count × 2 bytes).
    3. Reconstruct canonical codes from the sorted (symbol, length) list.
    4. Unpack the LSB-first bit stream.
    5. Decode original_length symbols by matching accumulated bits against the
       canonical code table (prefix-free, so no separator needed).

    Parameters
    ----------
    data:
        Compressed bytes produced by :func:`compress`.

    Returns
    -------
    bytes
        The original, uncompressed data.
    """
    data = bytes(data)

    if len(data) < 8:
        return b""

    original_length, symbol_count = struct.unpack(">II", data[:8])

    if original_length == 0:
        return b""

    # Parse code-lengths table.
    # Each entry: 2 bytes — symbol (uint8), code_length (uint8).
    # Wire format guarantees entries are sorted by (code_length, symbol_value).
    table_offset = 8
    lengths: list[tuple[int, int]] = []
    for i in range(symbol_count):
        off = table_offset + 2 * i
        symbol = data[off]
        length = data[off + 1]
        lengths.append((symbol, length))

    # Reconstruct canonical codes from the sorted lengths list.
    #
    # The canonical reconstruction rule (same as DEFLATE):
    #   code = 0
    #   prev_length = first entry's length
    #   for each entry in sorted order:
    #     if length > prev_length: code <<= (length - prev_length)
    #     assign bit_string = zero-padded binary of code
    #     code += 1
    #
    # This produces a prefix-free code table guaranteed to match the encoder.
    code_to_symbol: dict[str, int] = {}
    code = 0
    prev_length = lengths[0][1] if lengths else 0
    for symbol, length in lengths:
        if length > prev_length:
            code <<= (length - prev_length)
        bit_string = format(code, f"0{length}b")
        code_to_symbol[bit_string] = symbol
        code += 1
        prev_length = length

    # Unpack the bit stream.
    bits_offset = table_offset + 2 * symbol_count
    bit_string = _unpack_bits_lsb_first(data[bits_offset:])

    # Decode exactly original_length symbols using the canonical code table.
    #
    # Because the code is prefix-free, we can scan left-to-right: accumulate
    # bits one at a time until we get a hit in code_to_symbol, emit the symbol,
    # reset the accumulator, and repeat.
    output: list[int] = []
    pos = 0
    accumulated = ""
    while len(output) < original_length:
        if pos >= len(bit_string):
            msg = "bit stream exhausted before decoding all symbols"
            raise ValueError(msg)
        accumulated += bit_string[pos]
        pos += 1
        if accumulated in code_to_symbol:
            output.append(code_to_symbol[accumulated])
            accumulated = ""

    return bytes(output)
