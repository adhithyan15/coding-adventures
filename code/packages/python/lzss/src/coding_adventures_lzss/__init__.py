"""lzss — LZSS lossless compression algorithm (1982) from scratch.

What Is LZSS?
=============

LZSS (Lempel-Ziv-Storer-Szymanski, 1982) is a refinement of LZ77 (CMP00) that
eliminates a systematic waste: in LZ77, every token emits a trailing `next_char`
byte even when a long back-reference was just found. LZSS replaces the fixed
(offset, length, next_char) triple with a **flag-bit** scheme, where each symbol
is either a bare literal byte or a bare back-reference — never both.

The Fix in One Line
===================

    LZ77:  every token = 4 bytes  (offset=2 + length=1 + next_char=1)
    LZSS:  literal   = 1 byte     (just the byte value)
            match    = 3 bytes     (offset=2 + length=1)

A flag byte precedes every group of 8 symbols to tell the decoder which of its
8 children are literals and which are matches.

The Flag-Byte Scheme
====================

Symbols are grouped in chunks of 8. Each chunk is preceded by one flag byte:

    bit 0 (LSB) = type of symbol 0 in this chunk
    bit 1       = type of symbol 1
    ...
    bit 7       = type of symbol 7

    0 = Literal   → 1 byte  (the actual byte value)
    1 = Match     → 3 bytes (offset as big-endian uint16 + length as uint8)

Example flag byte 0b00000011:
    symbol 0 → Literal (bit 0 = 0)
    symbol 1 → Literal (bit 1 = 0)
    symbol 2 → Match   (bit 2 = 1)
    ... all others 0 = Literal

The Break-Even Point
====================

A Match costs 3 bytes. Three Literals also cost 3 bytes. So a match of length 3
breaks even; length ≥ 4 yields a net saving. Traditionally `min_match = 3` is
used (break-even threshold, same convention as LZ77).

Overlapping Matches
====================

LZSS inherits LZ77's ability to encode self-referential matches. When offset < length,
the source region overlaps the destination being written. Example:

    Output so far: [A]
    Token: Match(offset=1, length=6)
    → start = 0 (one byte back); copy 6 bytes one-at-a-time:
      A → [A,A] → [A,A,A] → ... → [A,A,A,A,A,A,A]

This is run-length encoding as a degenerate case of LZ77-style matching.
The decoder must copy byte-by-byte, NOT as a bulk memmove.

Wire Format (CMP02)
====================

    Bytes 0–3:  original_length (big-endian uint32)
    Bytes 4–7:  block_count     (big-endian uint32)
    Bytes 8+:   blocks

    Each block:
      [1 byte]  flag_byte (bits 0-7 correspond to symbols 0-7 in the block)
      [variable] symbol data:
          Literal: 1 byte  (the byte value)
          Match:   3 bytes (offset BE uint16 + length uint8)

    The last block may have < 8 symbols; unused flag bits are 0.

`original_length` is stored because LZSS has no sentinel — the decoder needs the
exact count to know when to stop (unlike LZ77 where next_char always terminated).

The Series: CMP00 → CMP05
==========================

CMP00 (LZ77, 1977)    — Sliding-window with (offset, length, next_char) tokens.
CMP01 (LZ78, 1978)    — Explicit trie dictionary; no sliding window.
CMP02 (LZSS, 1982)    — LZ77 + flag bits; this module.
CMP03 (LZW, 1984)     — LZ78 + pre-initialised alphabet; powers GIF.
CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.

References
==========

Storer, J.A., & Szymanski, T.G. (1982). "Data Compression via Textual Substitution".
Journal of the ACM, 29(4), 928–951.

Lempel, A., & Ziv, J. (1977). "A Universal Algorithm for Sequential Data Compression".
IEEE Transactions on Information Theory, 23(3), 337–343.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass

__all__ = [
    "Literal",
    "Match",
    "Token",
    "encode",
    "decode",
    "compress",
    "decompress",
]


# ─── Token types ──────────────────────────────────────────────────────────────


@dataclass(frozen=True)
class Literal:
    """A single literal byte in the LZSS token stream.

    Attributes:
        byte: The byte value (0–255).
    """

    byte: int


@dataclass(frozen=True)
class Match:
    """A back-reference match in the LZSS token stream.

    Attributes:
        offset: Distance back in the output where the match begins (1..window_size).
        length: Number of bytes to copy (min_match..max_match).
    """

    offset: int
    length: int


# Union type alias for documentation clarity.
Token = Literal | Match


# ─── Sliding-window encoder ───────────────────────────────────────────────────


def _find_longest_match(
    data: bytes,
    cursor: int,
    window_size: int,
    max_match: int,
) -> tuple[int, int]:
    """Find the longest match for data[cursor:] in the search buffer.

    Scans the last window_size bytes before cursor for the longest prefix of
    data[cursor:] that also appears in the search buffer. Matches may overlap
    (extend past cursor) — this is intentional and enables run-length encoding
    as a degenerate case.

    Args:
        data: Full input byte sequence.
        cursor: Current encoding position (start of lookahead).
        window_size: Maximum lookback distance.
        max_match: Maximum match length.

    Returns:
        (best_offset, best_length) — distance back and match length.
        Returns (0, 0) when no match exists.
    """
    best_offset = 0
    best_length = 0

    search_start = max(0, cursor - window_size)
    # Lookahead may extend to the end of input (no next_char reservation).
    lookahead_end = min(cursor + max_match, len(data))

    for pos in range(search_start, cursor):
        length = 0
        while (
            cursor + length < lookahead_end
            and data[pos + length] == data[cursor + length]
        ):
            length += 1

        if length > best_length:
            best_length = length
            best_offset = cursor - pos

    return best_offset, best_length


def encode(
    data: bytes,
    window_size: int = 4096,
    max_match: int = 255,
    min_match: int = 3,
) -> list[Token]:
    """Encode bytes into an LZSS token stream.

    Scans input left-to-right. At each cursor position, searches the search
    buffer (last window_size bytes) for the longest match. If the match is
    long enough (≥ min_match), emits a Match token and advances cursor by
    match length. Otherwise, emits a Literal and advances by 1.

    Key difference from LZ77: no `next_char` appended after a match. The
    cursor advances by exactly `length` bytes on a match (not `length + 1`).

    Args:
        data: Input bytes to compress.
        window_size: Maximum lookback distance (default 4096).
        max_match: Maximum match length (default 255, fits in uint8).
        min_match: Minimum match length for a Match token (default 3).

    Returns:
        List of Token objects (Literal or Match instances).

    Example::

        >>> encode(b"ABABAB")
        [Literal(byte=65), Literal(byte=66), Match(offset=2, length=4)]
    """
    tokens: list[Token] = []
    cursor = 0

    while cursor < len(data):
        best_offset, best_length = _find_longest_match(
            data, cursor, window_size, max_match
        )

        if best_length >= min_match:
            # Emit a back-reference — no trailing literal.
            tokens.append(Match(offset=best_offset, length=best_length))
            cursor += best_length
        else:
            # No useful match — emit the raw byte.
            tokens.append(Literal(byte=data[cursor]))
            cursor += 1

    return tokens


# ─── Decoder ──────────────────────────────────────────────────────────────────


def decode(
    tokens: list[Token],
    original_length: int = -1,
) -> bytes:
    """Decode an LZSS token stream back into the original bytes.

    Processes each token:
    - Literal  → append that byte to output.
    - Match    → copy `length` bytes from `offset` positions back, byte-by-byte.

    Byte-by-byte copying handles overlapping matches (offset < length) which
    naturally encode runs; a bulk copy would produce incorrect output.

    Args:
        tokens: Token stream from encode().
        original_length: If ≥ 0, truncates output to this length. Pass -1 to
            return all bytes. The compress/decompress API always passes the
            stored original length.

    Returns:
        Reconstructed bytes.

    Example::

        >>> decode([Literal(65), Match(offset=1, length=6)])
        b'AAAAAAA'
    """
    output = bytearray()

    for token in tokens:
        if isinstance(token, Literal):
            output.append(token.byte)
        else:
            # Match — copy byte-by-byte for overlap safety.
            start = len(output) - token.offset
            for _ in range(token.length):
                output.append(output[start])
                start += 1

    if original_length >= 0:
        return bytes(output[:original_length])
    return bytes(output)


# ─── Serialisation ────────────────────────────────────────────────────────────


def _serialise_tokens(tokens: list[Token], original_length: int) -> bytes:
    """Serialise an LZSS token list to the CMP02 wire format.

    Groups tokens into blocks of up to 8. Each block is preceded by a flag byte
    (one bit per token, LSB = first token; 0 = Literal, 1 = Match). After the
    flag byte, each token's data follows:
        Literal: 1 byte (the byte value)
        Match:   3 bytes (offset as big-endian uint16 + length as uint8)

    Header:
        original_length: 4 bytes (big-endian uint32)
        block_count:     4 bytes (big-endian uint32)
    """
    blocks: list[bytes] = []

    i = 0
    while i < len(tokens):
        chunk = tokens[i : i + 8]
        flag = 0
        data_parts: list[bytes] = []

        for bit, tok in enumerate(chunk):
            if isinstance(tok, Match):
                flag |= 1 << bit
                data_parts.append(struct.pack(">HB", tok.offset, tok.length))
            else:
                data_parts.append(struct.pack("B", tok.byte))

        blocks.append(bytes([flag]) + b"".join(data_parts))
        i += 8

    header = struct.pack(">II", original_length, len(blocks))
    return header + b"".join(blocks)


def _deserialise_tokens(data: bytes) -> tuple[list[Token], int]:
    """Deserialise CMP02 wire-format bytes back into tokens and original length.

    Security note: block_count from the header is capped against the actual
    payload size to prevent a crafted header from causing excessive allocation.

    Returns:
        (tokens, original_length) tuple.
    """
    if len(data) < 8:
        return [], 0

    original_length, block_count = struct.unpack(">II", data[:8])

    # Each block is at minimum 1 byte (flag only, all literals with 0 value).
    # Cap to prevent DoS from crafted headers claiming billions of blocks.
    max_possible_blocks = len(data) - 8  # 1 byte minimum per block
    safe_block_count = min(block_count, max_possible_blocks)

    tokens: list[Token] = []
    pos = 8

    for _ in range(safe_block_count):
        if pos >= len(data):
            break

        flag = data[pos]
        pos += 1

        for bit in range(8):
            if pos >= len(data):
                break
            if flag & (1 << bit):
                # Match: 3 bytes (offset uint16 BE + length uint8)
                if pos + 3 > len(data):
                    break
                offset, length = struct.unpack(">HB", data[pos : pos + 3])
                tokens.append(Match(offset=offset, length=length))
                pos += 3
            else:
                # Literal: 1 byte
                tokens.append(Literal(byte=data[pos]))
                pos += 1

    return tokens, original_length


# ─── One-shot API ─────────────────────────────────────────────────────────────


def compress(
    data: bytes,
    window_size: int = 4096,
    max_match: int = 255,
    min_match: int = 3,
) -> bytes:
    """Compress bytes using LZSS, returning the CMP02 wire format.

    One-shot API: encode() then serialise. The wire format includes the
    original data length so decompress() can recover the exact byte count.

    Args:
        data: Input bytes to compress.
        window_size: Maximum lookback distance (default 4096).
        max_match: Maximum match length (default 255).
        min_match: Minimum match length for a Match token (default 3).

    Returns:
        Compressed bytes in CMP02 wire format.

    Example::

        >>> compressed = compress(b"AAAAAAA")
        >>> decompress(compressed)
        b'AAAAAAA'
    """
    tokens = encode(data, window_size, max_match, min_match)
    return _serialise_tokens(tokens, len(data))


def decompress(data: bytes) -> bytes:
    """Decompress bytes produced by compress().

    Deserialises the CMP02 wire format back into tokens, then decodes to bytes.
    Uses the stored original_length to return exactly the right number of bytes.

    Args:
        data: Compressed bytes from compress().

    Returns:
        Original uncompressed bytes.

    Example::

        >>> original = b"hello hello hello"
        >>> decompress(compress(original)) == original
        True
    """
    tokens, original_length = _deserialise_tokens(data)
    return decode(tokens, original_length)
