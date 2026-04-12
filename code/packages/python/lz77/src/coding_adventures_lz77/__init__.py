"""lz77 — LZ77 lossless compression algorithm (1977) from scratch.

What Is LZ77?
=============

LZ77 is the foundational sliding-window compression algorithm published by
Abraham Lempel and Jacob Ziv in 1977. It is the ancestor of LZSS, LZW,
DEFLATE, zstd, LZ4, and virtually every modern compressor used in ZIP, gzip,
PNG, and zlib.

The core idea is simple: instead of storing every byte verbatim, notice when
a sequence of bytes has appeared recently. Replace that sequence with a cheap
reference to where it was — the "offset" (how far back) and "length" (how many
bytes). This exploits the locality of real data: a repeated word in a document,
a copied instruction in a binary, an adjacent colour run in an image — all
trigger compression.

The Sliding Window Model
=========================

LZ77 processes input left-to-right, maintaining two conceptual buffers:

    ┌─────────────────────────────────┬──────────────────┐
    │         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
    │  (already processed — the       │  (not yet seen —  │
    │   last window_size bytes)       │  next max_match)  │
    └─────────────────────────────────┴──────────────────┘
                                      ↑
                                  cursor (current position)

— The search buffer is the bytes already encoded (up to window_size bytes back).
— The lookahead buffer is the next unprocessed input.

At each step, the encoder searches the search buffer for the longest sequence
that matches the start of the lookahead buffer. If found and long enough
(length ≥ min_match), emit a backreference token. Otherwise, emit a literal
token for the current byte and advance.

The Token: (offset, length, next_char)
======================================

The encoder outputs a stream of tokens. Each token is a triple:

    (offset, length, next_char)

| Field     | Type | Meaning                                    |
| --------- | ---- | ------------------------------------------ |
| offset    | int  | Distance back the match starts (1..window  |
|           |      | _size), or 0 for no match                  |
| length    | int  | How many bytes the match covers (0 = no    |
|           |      | match), up to max_match                    |
| next_char | int  | The literal byte immediately after match   |

The next_char is always emitted to advance the stream by length+1 bytes.

Overlapping Matches (Why Byte-by-Byte Copy Matters)
=====================================================

A match is allowed to extend into bytes that haven't been written yet. This
happens when offset < length. For example:

    If output so far is [A, B] and the token is (2, 5, 'Z'):
        offset=2 means "go back 2 bytes" → position 0 (byte A)
        length=5 means "copy 5 bytes starting there"
        But we only have 2 bytes in the buffer!

    The decoder must copy byte-by-byte, not all at once:
        1. Copy output[0] (A) → [A, B, A]
        2. Copy output[1] (B) → [A, B, A, B]
        3. Copy output[2] (A, just written) → [A, B, A, B, A]  ← copy what we just wrote
        4. Copy output[3] (B, just written) → [A, B, A, B, A, B]
        5. Copy output[4] (A, just written) → [A, B, A, B, A, B, A]
        Finally, append next_char 'Z' → [A, B, A, B, A, B, A, Z]

This byte-by-byte copy automatically handles the self-referential match and
acts as run-length encoding for repeating patterns.

The Series: CMP00 → CMP05
==========================

CMP00 (LZ77, 1977) — Sliding-window backreferences. This foundation.
CMP01 (LZ78, 1978) — Explicit dictionary (trie), no sliding window.
CMP02 (LZSS, 1982) — LZ77 + flag bits; eliminates wasted next_char.
CMP03 (LZW, 1984) — Pre-initialized dictionary; powers GIF.
CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.

Understanding LZ77 builds the mental model for all of them.

Reference
==========

Lempel, A., & Ziv, J. (1977). "A Universal Algorithm for Sequential Data
Compression". IEEE Transactions on Information Theory, 23(3), 337–343.
"""

from typing import NamedTuple

__all__ = [
    "Token",
    "encode",
    "decode",
    "compress",
    "decompress",
]


class Token(NamedTuple):
    """A single LZ77 token: (offset, length, next_char).

    Represents one unit of the compressed stream.

    Attributes:
        offset: Distance back the match starts (1..window_size), or 0.
        length: Number of bytes the match covers (0 = no match).
        next_char: Literal byte immediately after the match (0..255).
    """

    offset: int
    length: int
    next_char: int


def _find_longest_match(
    data: bytes,
    cursor: int,
    window_size: int,
    max_match: int,
) -> tuple[int, int]:
    """Find the longest match in the search buffer.

    Scans the search buffer (the last window_size bytes before cursor) for the
    longest substring that matches the start of the lookahead buffer
    (starting at cursor).

    Args:
        data: The input data.
        cursor: Current position in the input (start of lookahead).
        window_size: Maximum number of bytes to search backwards.
        max_match: Maximum match length (limited by parameter or end of input).

    Returns:
        A tuple (best_offset, best_length) where best_offset is the distance
        back from cursor (1-indexed), and best_length is the number of bytes
        matched. Returns (0, 0) if no match found.
    """
    best_offset = 0
    best_length = 0

    # The search buffer starts at most window_size bytes back.
    search_start = max(0, cursor - window_size)

    # The lookahead cannot extend past the end of input.
    # We must reserve 1 byte for next_char.
    lookahead_end = min(cursor + max_match, len(data) - 1)

    # Try every possible match start in the search buffer.
    for pos in range(search_start, cursor):
        length = 0
        # Match byte by byte. Matches may overlap (extend past cursor).
        while (
            cursor + length < lookahead_end
            and data[pos + length] == data[cursor + length]
        ):
            length += 1

        if length > best_length:
            best_length = length
            best_offset = cursor - pos  # Distance back from cursor.

    return best_offset, best_length


def encode(
    data: bytes,
    window_size: int = 4096,
    max_match: int = 255,
    min_match: int = 3,
) -> list[Token]:
    """Encode data into an LZ77 token stream.

    Scans the input left-to-right, finding the longest match in the search
    buffer for each position. If a match is long enough (≥ min_match),
    emits a backreference token; otherwise, emits a literal token.

    Args:
        data: The input bytes to compress.
        window_size: Maximum offset (default 4096). Larger = better compression,
            more memory.
        max_match: Maximum match length (default 255). Limited by how many bits
            the length field uses during serialisation.
        min_match: Minimum match length to emit a backreference (default 3).
            A match of length < 3 does not save space (a token costs the same
            whether offset=0 or offset>0 in the output).

    Returns:
        A list of Token objects representing the compressed stream.

    Example::

        >>> tokens = encode(b"ABABABAB")
        >>> len(tokens)
        3
        >>> decode(tokens)
        b'ABABABAB'
    """
    tokens: list[Token] = []
    cursor = 0

    while cursor < len(data):
        best_offset, best_length = _find_longest_match(
            data, cursor, window_size, max_match
        )

        if best_length >= min_match:
            # Emit a backreference token.
            next_char = data[cursor + best_length]
            tokens.append(Token(best_offset, best_length, next_char))
            cursor += best_length + 1
        else:
            # Emit a literal token (no match or too short).
            tokens.append(Token(0, 0, data[cursor]))
            cursor += 1

    return tokens


def decode(
    tokens: list[Token],
    initial_buffer: bytes = b"",
) -> bytes:
    """Decode a token stream back into the original data.

    Processes each token: if length > 0, copies length bytes from the search
    buffer (initial_buffer + output so far) starting at position
    (current_output_length - offset). Then appends next_char.

    Args:
        tokens: The token stream (output of encode()).
        initial_buffer: Optional seed for the search buffer (useful for
            streaming decompression). Default is empty.

    Returns:
        The reconstructed bytes.

    Example::

        >>> tokens = [Token(0, 0, 65), Token(1, 3, 68)]
        >>> decode(tokens)
        b'AAAD'
    """
    output = bytearray(initial_buffer)

    for token in tokens:
        if token.length > 0:
            # Copy length bytes from position (current_output_length - offset).
            start = len(output) - token.offset
            # Copy byte-by-byte to handle overlapping matches.
            for _ in range(token.length):
                output.append(output[start])
                start += 1

        # Always append next_char.
        output.append(token.next_char)

    return bytes(output)


def _serialise_tokens(tokens: list[Token]) -> bytes:
    """Serialise a token list to bytes using a fixed-width format.

    Format:
        [4 bytes: token count (big-endian uint32)]
        [N × 4 bytes: each token as (offset, length, next_char)]
            - 2 bytes: offset (big-endian uint16)
            - 1 byte: length (uint8)
            - 1 byte: next_char (uint8)

    This is a teaching format, not an industry one. Production compressors use
    variable-width bit-packing (see DEFLATE, zstd).
    """
    import struct

    data = struct.pack(">I", len(tokens))

    for token in tokens:
        # Pack offset (2 bytes), length (1 byte), next_char (1 byte).
        data += struct.pack(">HBB", token.offset, token.length, token.next_char)

    return data


def _deserialise_tokens(data: bytes) -> list[Token]:
    """Deserialise bytes back into a token list.

    Inverse of _serialise_tokens().
    """
    import struct

    if len(data) < 4:
        return []

    token_count = struct.unpack(">I", data[:4])[0]
    tokens: list[Token] = []

    for i in range(token_count):
        offset_bytes = 4 + i * 4
        if offset_bytes + 4 > len(data):
            break
        offset, length, next_char = struct.unpack(
            ">HBB", data[offset_bytes : offset_bytes + 4]
        )
        tokens.append(Token(offset, length, next_char))

    return tokens


def compress(
    data: bytes,
    window_size: int = 4096,
    max_match: int = 255,
    min_match: int = 3,
) -> bytes:
    """Compress data using LZ77.

    One-shot API: encode() then serialise the token stream to bytes.

    Args:
        data: The input bytes to compress.
        window_size: Maximum offset (default 4096).
        max_match: Maximum match length (default 255).
        min_match: Minimum match length for backreferences (default 3).

    Returns:
        The compressed bytes.

    Example::

        >>> original = b"AAAAAAA"
        >>> compressed = compress(original)
        >>> decompress(compressed)
        b'AAAAAAA'
    """
    tokens = encode(data, window_size, max_match, min_match)
    return _serialise_tokens(tokens)


def decompress(data: bytes) -> bytes:
    """Decompress data that was compressed with compress().

    Deserialises the byte stream into tokens, then decodes.

    Args:
        data: The compressed bytes.

    Returns:
        The original uncompressed data.
    """
    tokens = _deserialise_tokens(data)
    return decode(tokens)
