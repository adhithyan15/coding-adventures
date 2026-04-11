"""
coding_adventures_lz78 — LZ78 Lossless Compression Algorithm (1978)
=====================================================================

LZ78 (Lempel & Ziv, 1978) is the second paper in the Lempel-Ziv series,
published in *IEEE Transactions on Information Theory*. Where LZ77 finds matches
inside a fixed-size sliding window of recent bytes, LZ78 builds an **explicit
dictionary** that grows as encoding proceeds. Both the encoder and decoder build
the same dictionary independently — so no dictionary is transmitted on the wire.

=== The Core Idea ===

Think of encoding as navigating a trie (prefix tree) built from byte sequences
seen so far. The trie starts with a single root node (representing the empty
sequence, dictionary ID 0). As the encoder reads bytes:

  1. Follow the edge for the current byte from the current node.
  2. If the edge exists → move along it (extend the current match).
  3. If the edge does NOT exist → we've found the longest match:
       • Emit Token(current_node.id, current_byte).
       • Add a new trie node for this extension (new dictionary entry).
       • Reset to root and start a fresh match.

Example: encoding "ABABAB"

    Trie grows:
    root ──'A'──> node 1 ("A")
         ──'B'──> node 2 ("B")
    node 1 ──'B'──> node 3 ("AB")

    Tokens: [(0,'A'), (0,'B'), (1,'B'), flush(3, sentinel)]

The decoder mirrors this exactly: for each token (dict_index, next_char), it
reconstructs the sequence at dict_index by walking the parent chain, emits those
bytes, emits next_char, then adds the same new dictionary entry.

=== Token Structure ===

    Token(dict_index, next_char)

    dict_index  ∈ [0, max_dict_size)
                0 = pure literal (no dictionary match)
                k > 0 = matches the k-th dictionary entry
    next_char   ∈ [0, 255]
                The byte that follows the match.
                Also used as the flush sentinel (value=0) when input ends
                mid-match.

=== End-of-Stream Handling ===

If the input ends while the encoder is in the middle of a dictionary match,
a "flush token" is emitted: Token(current_node.id, next_char=0). The value 0
is a sentinel. The compress() function stores the original data length in the
wire format so decompress() can truncate the output and discard the sentinel.

=== Wire Format ===

    compress() output:
        Bytes 0–3:   original_length (big-endian uint32)
        Bytes 4–7:   token_count (big-endian uint32)
        Bytes 8+:    token_count × 4 bytes each:
                       [0..1]  dict_index (big-endian uint16)
                       [2]     next_char (uint8)
                       [3]     reserved (0x00)

=== Series Context ===

    CMP00 (LZ77, 1977) — Sliding-window backreferences.    ← predecessor
    CMP01 (LZ78, 1978) — Explicit dictionary (trie).       ← this module
    CMP02 (LZSS, 1982) — LZ77 + flag bits; no wasted literals.
    CMP03 (LZW,  1984) — LZ78 + pre-initialised 256-entry alphabet; GIF.
    CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
    CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.

LZW (CMP03) is essentially LZ78 with a pre-seeded dictionary of all 256
single-byte entries and a fixed maximum size with reset logic.
"""

from __future__ import annotations

import struct
from typing import NamedTuple

from trie import TrieCursor

__all__ = [
    "Token",
    "encode",
    "decode",
    "compress",
    "decompress",
]


# ─── Token ────────────────────────────────────────────────────────────────────


class Token(NamedTuple):
    """
    One LZ78 token: (dict_index, next_char).

    dict_index: ID of the longest dictionary prefix that matches. 0 = no match.
    next_char:  Byte following the match. 0 is also the flush sentinel for
                end-of-stream partial matches.

    Example:
        Token(0, 65)   → pure literal 'A'
        Token(3, 66)   → emit sequence for dict entry 3, then byte 'B'
    """

    dict_index: int
    next_char: int


# ─── Encoder ──────────────────────────────────────────────────────────────────


def encode(data: bytes | bytearray, max_dict_size: int = 65536) -> list[Token]:
    """
    Encode bytes into an LZ78 token stream.

    Uses a ``TrieCursor[int, int]`` to walk the dictionary one byte at a time.
    The cursor starts at the root (representing the empty sequence, dict ID 0)
    and advances along child edges as bytes arrive. When a byte has no child
    edge (``cursor.step(byte)`` returns False), the current cursor position
    represents the longest match — emit that match's dict ID plus the new byte
    as a token, record the new sequence, and reset the cursor to root.

    This cursor-based design keeps the encoding loop free of low-level trie
    node manipulation. The same ``TrieCursor`` abstraction applies to LZW
    (CMP03) and any other streaming dictionary algorithm.

    Args:
        data:          Input bytes to compress.
        max_dict_size: Maximum dictionary entries (IDs 0 to max-1).
                       Beyond this limit, new sequences are no longer recorded.

    Returns:
        List of Token(dict_index, next_char) in emission order.

    Example:
        >>> encode(b"ABCDE")
        [Token(0, 65), Token(0, 66), Token(0, 67), Token(0, 68), Token(0, 69)]

        >>> encode(b"AABCBBABC")
        [Token(0, 65), Token(1, 66), Token(0, 67), Token(0, 66), Token(4, 65), Token(4, 67)]
    """
    # TrieCursor[int, int]: key elements are byte values (int 0–255),
    # values are dictionary IDs (int 1..max_dict_size-1).
    # The root represents dict entry 0 (the empty sequence / no match).
    cursor: TrieCursor[int, int] = TrieCursor()
    next_id = 1
    tokens: list[Token] = []

    for byte in data:
        byte = int(byte)  # ensure int even from bytearray

        if cursor.step(byte):
            # Edge exists — the current match can be extended.
            pass
        else:
            # No edge for this byte — emit token (current dict ID + byte).
            tokens.append(Token(dict_index=cursor.value or 0, next_char=byte))

            # Record the new sequence in the dictionary if there's room.
            if next_id < max_dict_size:
                cursor.insert(byte, next_id)
                next_id += 1

            # Reset cursor to root to start a fresh match.
            cursor.reset()

    # End-of-stream: if cursor is not at root, a partial match needs flushing.
    # next_char=0 is the sentinel; compress() stores original_length so that
    # decompress() can truncate and discard this sentinel byte.
    if not cursor.at_root:
        tokens.append(Token(dict_index=cursor.value or 0, next_char=0))

    return tokens


# ─── Decoder ──────────────────────────────────────────────────────────────────


def decode(tokens: list[Token], original_length: int = -1) -> bytes:
    """
    Decode an LZ78 token stream back into the original bytes.

    Mirrors the encoder: maintains the same dictionary as a list of
    (parent_id, byte) pairs. For each token, reconstructs the sequence for
    dict_index, emits it, emits next_char, then adds a new dictionary entry.

    Args:
        tokens:          Token stream from encode().
        original_length: If >= 0, truncate output to this many bytes. Used by
                         decompress() to strip the flush sentinel byte.
                         If -1 (default), return all output bytes.

    Returns:
        Reconstructed bytes.

    Example:
        >>> decode([Token(0, 65), Token(0, 66), Token(1, 66)])
        b'AABB'

        # Note: Token(1, 66) emits sequence for entry 1 ("A") then 'B'.
        # Entry 1 is "A" (added after first token).
    """
    # dict_table[i] = (parent_id, byte): entry i is parent_id's sequence + byte.
    # Index 0 is the root (empty sequence, no parent). We use a sentinel entry.
    dict_table: list[tuple[int, int]] = [(0, 0)]  # entry 0 = root sentinel

    output: bytearray = bytearray()

    for token in tokens:
        dict_index, next_char = token

        # Reconstruct the sequence for dict_index.
        # Walk the parent chain, collecting bytes, then reverse.
        sequence: list[int] = []
        idx = dict_index
        while idx != 0:
            parent_id, byte = dict_table[idx]
            sequence.append(byte)
            idx = parent_id
        sequence.reverse()

        # Emit the reconstructed sequence.
        output.extend(sequence)

        # Emit next_char unless we've already reached the original length.
        # This correctly handles the flush sentinel (next_char=0) by letting
        # the original_length truncation strip it.
        if original_length < 0 or len(output) < original_length:
            output.append(next_char)

        # Add the new dictionary entry (same as the encoder added).
        dict_table.append((dict_index, next_char))

    if original_length >= 0:
        return bytes(output[:original_length])
    return bytes(output)


# ─── Serialisation ────────────────────────────────────────────────────────────


def _serialise_tokens(tokens: list[Token], original_length: int) -> bytes:
    """
    Serialise tokens to the CMP01 wire format.

    Wire format:
        4 bytes  — original_length (big-endian uint32)
        4 bytes  — token_count (big-endian uint32)
        N × 4    — tokens: uint16 dict_index (BE) + uint8 next_char + uint8 0x00

    The original_length field allows decompress() to strip the flush sentinel
    byte that may appear when the input ends mid-dictionary-match.

    Args:
        tokens:          Token list from encode().
        original_length: Length of the original uncompressed data.

    Returns:
        Serialised byte string.
    """
    buf = bytearray()
    # 4-byte header: original length.
    buf += struct.pack(">I", original_length)
    # 4-byte header: token count.
    buf += struct.pack(">I", len(tokens))
    # Tokens: 2 bytes dict_index + 1 byte next_char + 1 byte reserved.
    for token in tokens:
        buf += struct.pack(">H", token.dict_index)
        buf.append(token.next_char)
        buf.append(0x00)  # reserved
    return bytes(buf)


def _deserialise_tokens(data: bytes | bytearray) -> tuple[list[Token], int]:
    """
    Deserialise bytes back into a token list and original length.

    Inverse of _serialise_tokens.

    Args:
        data: Bytes from compress().

    Returns:
        (token_list, original_length)

    Raises:
        ValueError: If the data is too short to be a valid header.
    """
    if len(data) < 8:
        return [], 0

    original_length = struct.unpack(">I", data[0:4])[0]
    token_count = struct.unpack(">I", data[4:8])[0]
    tokens: list[Token] = []

    for i in range(token_count):
        base = 8 + i * 4
        if base + 4 > len(data):
            break
        dict_index = struct.unpack(">H", data[base : base + 2])[0]
        next_char = data[base + 2]
        tokens.append(Token(dict_index=dict_index, next_char=next_char))

    return tokens, original_length


# ─── One-shot API ─────────────────────────────────────────────────────────────


def compress(data: bytes | bytearray, max_dict_size: int = 65536) -> bytes:
    """
    Compress bytes using LZ78.

    Encodes data into a token stream and serialises it to the CMP01 wire
    format. The wire format stores the original length so that decompress()
    can exactly reconstruct the input even when the last token is a flush
    token with a sentinel next_char=0.

    Args:
        data:          Input bytes to compress.
        max_dict_size: Maximum dictionary entries (default 65536).

    Returns:
        Compressed bytes in CMP01 wire format.

    Example:
        >>> original = b"hello hello hello"
        >>> compressed = compress(original)
        >>> decompress(compressed) == original
        True
    """
    data = bytes(data)
    tokens = encode(data, max_dict_size=max_dict_size)
    return _serialise_tokens(tokens, len(data))


def decompress(data: bytes | bytearray) -> bytes:
    """
    Decompress bytes that were compressed with compress().

    Deserialises the wire format into a token list and original length, then
    decodes, truncating to the original length to strip any flush sentinel.

    Args:
        data: Bytes from compress().

    Returns:
        Original uncompressed bytes.

    Example:
        >>> decompress(compress(b"AAAAAAA")) == b"AAAAAAA"
        True
    """
    tokens, original_length = _deserialise_tokens(bytes(data))
    return decode(tokens, original_length=original_length)
