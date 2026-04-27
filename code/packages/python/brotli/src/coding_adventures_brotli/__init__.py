# =============================================================================
# CodingAdventures.Brotli
# =============================================================================
#
# Brotli (2013, RFC 7932) lossless compression algorithm.
# Part of the CMP compression series in the coding-adventures monorepo.
#
# What Is Brotli?
# ---------------
#
# Brotli is a general-purpose lossless compression algorithm developed at
# Google by Jyrki Alakuijärvi and Zoltán Szabadka. It became the dominant
# algorithm for HTTP `Content-Encoding: br` (2015) and the WOFF2 font format.
#
# Three major improvements over DEFLATE (CMP05):
#
#   1. Context-dependent literal trees — 4 Huffman trees for literals, one per
#      context bucket based on the preceding byte category.
#
#   2. Insert-and-copy commands — bundles insert_length + copy_length into one
#      ICC Huffman symbol, reducing overhead.
#
#   3. Larger sliding window — 65535 bytes vs DEFLATE's 4096 bytes.
#
# Encoding Order
# --------------
#
# Each non-flush command in the bit stream:
#
#   [ICC code] [insert_extras (LSB-first)] [copy_extras (LSB-first)]
#   [insert_length literal bytes, each via per-context Huffman tree]
#   [distance code] [dist_extras (LSB-first)]
#
# The bit stream ends with the sentinel ICC code (63), which may be followed
# by "flush literal" bytes if the input did not end on a copy boundary:
#
#   [... last real command ...] [ICC=63] [flush literal bytes, if any]
#
# This design allows pure-literal inputs (no LZ matches) to be encoded
# correctly: the sentinel terminates the command loop, then the decompressor
# reads any remaining literals up to original_length.
#
# Wire Format (CMP06)
# -------------------
#
# Header (10 bytes):
#   [4B] original_length  — big-endian uint32
#   [1B] icc_entry_count  — uint8 (1–64)
#   [1B] dist_entry_count — uint8 (0–32)
#   [1B] ctx0_entry_count — uint8 (0–256)
#   [1B] ctx1_entry_count — uint8 (0–256)
#   [1B] ctx2_entry_count — uint8 (0–256)
#   [1B] ctx3_entry_count — uint8 (0–256)
#
# ICC code-length table  (icc_entry_count  × 2 bytes): symbol, code_length
# Dist code-length table (dist_entry_count × 2 bytes): symbol, code_length
# Lit tree 0..3 tables   (ctx{n}_count × 3 bytes each): symbol uint16 BE, code_length
# Bit stream: LSB-first packed, zero-padded to byte boundary.
#
# Empty Input
# -----------
#
# Header: [0][1][0][0][0][0][0]  (original_length=0, icc_count=1, rest=0)
# ICC table: symbol=63, code_length=1
# Bit stream: 0x00
#
# =============================================================================

from __future__ import annotations

import struct
from collections import Counter
from dataclasses import dataclass, field

from coding_adventures_huffman_tree import HuffmanTree

__all__ = ["compress", "decompress"]


# ---------------------------------------------------------------------------
# ICC (Insert-Copy Code) table — 64 codes
# ---------------------------------------------------------------------------
#
# Each ICC code encodes a range of (insert_length, copy_length) pairs.
# Code 63 is the end-of-data sentinel: insert=0, copy=0.
#
# _ICC_TABLE[code] = (insert_base, insert_extra, copy_base, copy_extra)
#
# Insert ranges:
#   Codes  0–15: insert=0 (no insert)
#   Codes 16–23: insert=1
#   Codes 24–31: insert=2
#   Codes 32–39: insert range 3–4  (base=3, extra=1)
#   Codes 40–47: insert range 5–8  (base=5, extra=2)
#   Codes 48–55: insert range 9–16 (base=9, extra=3)
#   Codes 56–62: insert range 17–32 (base=17, extra=4)
#   Code  63:    sentinel (insert=0, copy=0)
#
# Copy ranges (8 options, same set for each insert group):
#   copy=4, 5, 6, 8-9, 10-11, 14-17, 18-21, 26-33, 34-41, ...
#   (there are gaps: 7, 12-13, 22-25 are not representable directly)

_ICC_TABLE: list[tuple[int, int, int, int]] = [
    # (insert_base, insert_extra, copy_base, copy_extra)
    (0,  0,   4, 0),  # 0
    (0,  0,   5, 0),  # 1
    (0,  0,   6, 0),  # 2
    (0,  0,   8, 1),  # 3
    (0,  0,  10, 1),  # 4
    (0,  0,  14, 2),  # 5
    (0,  0,  18, 2),  # 6
    (0,  0,  26, 3),  # 7
    (0,  0,  34, 3),  # 8
    (0,  0,  50, 4),  # 9
    (0,  0,  66, 4),  # 10
    (0,  0,  98, 5),  # 11
    (0,  0, 130, 5),  # 12
    (0,  0, 194, 6),  # 13
    (0,  0, 258, 7),  # 14
    (0,  0, 514, 8),  # 15
    (1,  0,   4, 0),  # 16
    (1,  0,   5, 0),  # 17
    (1,  0,   6, 0),  # 18
    (1,  0,   8, 1),  # 19
    (1,  0,  10, 1),  # 20
    (1,  0,  14, 2),  # 21
    (1,  0,  18, 2),  # 22
    (1,  0,  26, 3),  # 23
    (2,  0,   4, 0),  # 24
    (2,  0,   5, 0),  # 25
    (2,  0,   6, 0),  # 26
    (2,  0,   8, 1),  # 27
    (2,  0,  10, 1),  # 28
    (2,  0,  14, 2),  # 29
    (2,  0,  18, 2),  # 30
    (2,  0,  26, 3),  # 31
    (3,  1,   4, 0),  # 32
    (3,  1,   5, 0),  # 33
    (3,  1,   6, 0),  # 34
    (3,  1,   8, 1),  # 35
    (3,  1,  10, 1),  # 36
    (3,  1,  14, 2),  # 37
    (3,  1,  18, 2),  # 38
    (3,  1,  26, 3),  # 39
    (5,  2,   4, 0),  # 40
    (5,  2,   5, 0),  # 41
    (5,  2,   6, 0),  # 42
    (5,  2,   8, 1),  # 43
    (5,  2,  10, 1),  # 44
    (5,  2,  14, 2),  # 45
    (5,  2,  18, 2),  # 46
    (5,  2,  26, 3),  # 47
    (9,  3,   4, 0),  # 48
    (9,  3,   5, 0),  # 49
    (9,  3,   6, 0),  # 50
    (9,  3,   8, 1),  # 51
    (9,  3,  10, 1),  # 52
    (9,  3,  14, 2),  # 53
    (9,  3,  18, 2),  # 54
    (9,  3,  26, 3),  # 55
    (17, 4,   4, 0),  # 56
    (17, 4,   5, 0),  # 57
    (17, 4,   6, 0),  # 58
    (17, 4,   8, 1),  # 59
    (17, 4,  10, 1),  # 60
    (17, 4,  14, 2),  # 61
    (17, 4,  18, 2),  # 62
    (0,  0,   0, 0),  # 63 sentinel
]

_ICC_INSERT_BASE:  list[int] = [r[0] for r in _ICC_TABLE]
_ICC_INSERT_EXTRA: list[int] = [r[1] for r in _ICC_TABLE]
_ICC_COPY_BASE:    list[int] = [r[2] for r in _ICC_TABLE]
_ICC_COPY_EXTRA:   list[int] = [r[3] for r in _ICC_TABLE]


# ---------------------------------------------------------------------------
# Distance code table (codes 0–31)
# ---------------------------------------------------------------------------

_DIST_TABLE: list[tuple[int, int]] = [
    (    1, 0),  #  0
    (    2, 0),  #  1
    (    3, 0),  #  2
    (    4, 0),  #  3
    (    5, 1),  #  4
    (    7, 1),  #  5
    (    9, 2),  #  6
    (   13, 2),  #  7
    (   17, 3),  #  8
    (   25, 3),  #  9
    (   33, 4),  # 10
    (   49, 4),  # 11
    (   65, 5),  # 12
    (   97, 5),  # 13
    (  129, 6),  # 14
    (  193, 6),  # 15
    (  257, 7),  # 16
    (  385, 7),  # 17
    (  513, 8),  # 18
    (  769, 8),  # 19
    ( 1025, 9),  # 20
    ( 1537, 9),  # 21
    ( 2049, 10), # 22
    ( 3073, 10), # 23
    ( 4097, 11), # 24
    ( 6145, 11), # 25
    ( 8193, 12), # 26
    (12289, 12), # 27
    (16385, 13), # 28
    (24577, 13), # 29
    (32769, 14), # 30
    (49153, 14), # 31
]

_DIST_BASE:  list[int] = [r[0] for r in _DIST_TABLE]
_DIST_EXTRA: list[int] = [r[1] for r in _DIST_TABLE]


# ---------------------------------------------------------------------------
# Context function
# ---------------------------------------------------------------------------
#
# Returns the context bucket (0-3) for a literal based on the preceding byte.
# Bucket 0: space/punctuation or start-of-stream (p1 < 0)
# Bucket 1: digit ('0'–'9')
# Bucket 2: uppercase ('A'–'Z')
# Bucket 3: lowercase ('a'–'z')

def _literal_context(p1: int) -> int:
    """Map the previous byte (or -1 for start-of-stream) to a context bucket.

    Examples:
        _literal_context(-1)        → 0   # start of stream
        _literal_context(ord(' ')) → 0   # space/punct
        _literal_context(ord('5')) → 1   # digit
        _literal_context(ord('T')) → 2   # uppercase
        _literal_context(ord('h')) → 3   # lowercase
    """
    if 0x61 <= p1 <= 0x7A:
        return 3      # 'a'–'z'
    if 0x41 <= p1 <= 0x5A:
        return 2      # 'A'–'Z'
    if 0x30 <= p1 <= 0x39:
        return 1      # '0'–'9'
    return 0          # space/punct or p1 < 0


# ---------------------------------------------------------------------------
# ICC code lookup
# ---------------------------------------------------------------------------

def _find_icc_code(insert_length: int, copy_length: int) -> int:
    """Find the ICC code whose ranges contain both insert_length and copy_length.

    Scans all 63 non-sentinel codes and returns the first match.
    The ICC table has gaps in copy-length coverage (e.g., copy=7 is not
    representable). The caller must ensure copy_length is encodable via
    _find_best_icc_copy() before calling this function.

    Examples:
        _find_icc_code(0, 4)  → 0   (insert=0, copy=4)
        _find_icc_code(1, 5)  → 17  (insert=1, copy=5)
        _find_icc_code(2, 4)  → 24  (insert=2, copy=4)
    """
    for code in range(63):
        ib, ie, cb, ce = _ICC_TABLE[code]
        in_range = ib <= insert_length <= ib + (1 << ie) - 1
        co_range = cb <= copy_length <= cb + (1 << ce) - 1
        if in_range and co_range:
            return code

    # Fallback: copy-only code (insert=0) for this copy_length.
    for code in range(16):
        cb, ce = _ICC_COPY_BASE[code], _ICC_COPY_EXTRA[code]
        if cb <= copy_length <= cb + (1 << ce) - 1:
            return code

    return 0


def _find_best_icc_copy(insert_length: int, copy_length: int) -> int:
    """Find the largest copy_length ≤ requested that has a valid ICC code.

    The ICC table has gaps (e.g., copy=7 is not representable for any code).
    This returns the largest encodable copy ≤ requested for the given insert.

    Examples:
        _find_best_icc_copy(0, 4)   → 4  (exact match)
        _find_best_icc_copy(0, 7)   → 6  (best below the gap 7)
        _find_best_icc_copy(0, 258) → 258 (exact match)
    """
    best = 0
    for code in range(63):
        ib, ie, cb, ce = _ICC_TABLE[code]
        if not (ib <= insert_length <= ib + (1 << ie) - 1):
            continue
        copy_max = cb + (1 << ce) - 1
        if cb <= copy_length <= copy_max:
            return copy_length        # exact match
        if copy_max <= copy_length and copy_max > best:
            best = copy_max
    return max(best, _MIN_MATCH)


# ---------------------------------------------------------------------------
# Distance code lookup
# ---------------------------------------------------------------------------

def _dist_code(distance: int) -> int:
    """Map a distance (1–65535) to a distance code (0–31).

    Examples:
        _dist_code(1)     → 0
        _dist_code(5)     → 4
        _dist_code(65535) → 31
    """
    for code in range(32):
        if distance <= _DIST_BASE[code] + (1 << _DIST_EXTRA[code]) - 1:
            return code
    return 31


# ---------------------------------------------------------------------------
# Bit I/O helpers (LSB-first, same convention as CMP05)
# ---------------------------------------------------------------------------

def _pack_bits_lsb_first(bits: str) -> bytes:
    """Pack a '0'/'1' bit string into bytes, LSB-first.

    The first bit occupies bit 0 (LSB) of the first byte.
    The last byte is zero-padded in its high bits.

    Example: "10110" → byte 0b00001101 = 0x0D
    """
    output: list[int] = []
    buf = 0
    pos = 0
    for b in bits:
        buf |= int(b) << pos
        pos += 1
        if pos == 8:
            output.append(buf)
            buf = 0
            pos = 0
    if pos > 0:
        output.append(buf)
    return bytes(output)


def _unpack_bits_lsb_first(data: bytes) -> str:
    """Expand bytes into a '0'/'1' bit string, reading each byte LSB-first."""
    bits = ""
    for byte_val in data:
        for i in range(8):
            bits += str((byte_val >> i) & 1)
    return bits


# ---------------------------------------------------------------------------
# Canonical code reconstruction
# ---------------------------------------------------------------------------

def _reconstruct_canonical_codes(
    lengths: list[tuple[int, int]],
) -> dict[str, int]:
    """Reconstruct {bit_string → symbol} from sorted (symbol, length) pairs.

    Returns {} for empty input.
    Returns {"0": symbol} for a single-entry list (single-symbol tree).
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
        result[format(code, f"0{code_len}b")] = symbol
        code += 1
        prev_len = code_len
    return result


# ---------------------------------------------------------------------------
# LZ matching (O(n²) sliding window scan)
# ---------------------------------------------------------------------------

_MAX_WINDOW: int = 65535
_MIN_MATCH:  int = 4
_MAX_MATCH:  int = 258


def _find_longest_match(data: bytes, pos: int, window_start: int) -> tuple[int, int]:
    """Find the longest match for data[pos:] in data[window_start:pos].

    Returns (distance, length) with distance = pos - match_start (1-based),
    and length ≥ _MIN_MATCH; or (0, 0) if no valid match found.
    """
    data_len = len(data)
    best_distance = 0
    best_length = 0

    for start in range(pos - 1, window_start - 1, -1):
        if data[start] != data[pos]:
            continue
        length = 0
        max_len = min(_MAX_MATCH, data_len - pos)
        while length < max_len and data[start + length] == data[pos + length]:
            length += 1
        if length > best_length:
            best_length = length
            best_distance = pos - start
            if best_length == _MAX_MATCH:
                break

    if best_length < _MIN_MATCH:
        return (0, 0)
    return (best_distance, best_length)


# ---------------------------------------------------------------------------
# Command dataclass
# ---------------------------------------------------------------------------

@dataclass
class _Command:
    """One Brotli insert-and-copy command.

    Bit stream for commands with copy_length > 0:
      [ICC][insert extras][copy extras][literals...][dist][dist extras]

    For the flush command (insert_length > 0, copy_length == 0):
      Literals are written AFTER the sentinel ICC=63 in the bit stream.

    Fields:
        insert_length:  Number of literal bytes.
        copy_length:    Number of bytes to copy (≥ 4), or 0 for flush/sentinel.
        copy_distance:  Distance (1-based) into history. 0 for flush/sentinel.
        literals:       The literal bytes.
    """

    insert_length: int
    copy_length:   int
    copy_distance: int
    literals:      list[int] = field(default_factory=list)


# ---------------------------------------------------------------------------
# Pass 1: LZ matching → command list
# ---------------------------------------------------------------------------

_MAX_INSERT_PER_ICC: int = 32   # max insert for codes 56-62: 17 + (1<<4) - 1 = 32


def _build_commands(data: bytes) -> tuple[list[_Command], list[int]]:
    """Scan *data* and produce a list of insert-and-copy commands plus flush literals.

    Returns
    -------
    (commands, flush_literals)
        commands       — regular ICC commands (each has copy_length >= 4), ending
                         with the sentinel Command(0, 0, 0, []).
        flush_literals — trailing literal bytes that could not be bundled into
                         an ICC command (e.g., the whole input for pure-literal
                         inputs, or the tail after the last LZ match).

    Encoding order:
        [regular commands] [sentinel ICC=63] [flush_literals…]

    All regular commands have a valid ICC code; copy_length is constrained to
    representable values via _find_best_icc_copy().
    """
    commands: list[_Command] = []
    insert_buf: list[int] = []
    pos = 0
    n = len(data)

    while pos < n:
        window_start = max(0, pos - _MAX_WINDOW)
        distance, length = _find_longest_match(data, pos, window_start)

        if length >= _MIN_MATCH and len(insert_buf) <= _MAX_INSERT_PER_ICC:
            # Only take an LZ match when insert_buf fits in a single ICC code
            # (insert_length ≤ 32). This avoids invalid copy distances that
            # would result from trying to split a large insert buffer across
            # multiple commands where the first copies reference bytes not yet
            # in the history.
            #
            # The ICC table has gaps in copy-length coverage (e.g., copy=7 is
            # not representable). Find the largest encodable copy ≤ length.
            actual_copy = _find_best_icc_copy(len(insert_buf), length)

            commands.append(
                _Command(
                    insert_length=len(insert_buf),
                    copy_length=actual_copy,
                    copy_distance=distance,
                    literals=insert_buf.copy(),
                )
            )
            insert_buf = []
            pos += actual_copy
        else:
            insert_buf.append(data[pos])
            pos += 1

    # Any remaining bytes in insert_buf become flush_literals.
    # They are encoded AFTER the sentinel in the bit stream.
    # This also handles the case where insert_buf exceeded _MAX_INSERT_PER_ICC:
    # those bytes are emitted as literals after the sentinel rather than
    # risking an invalid copy that would corrupt the output.
    flush_literals = insert_buf.copy()

    # Sentinel (insert=0, copy=0) — marks end of ICC command stream.
    commands.append(_Command(0, 0, 0, []))
    return commands, flush_literals


# ---------------------------------------------------------------------------
# Pass 2a: Frequency tallying
# ---------------------------------------------------------------------------

def _tally_frequencies(
    commands: list[_Command],
    flush_literals: list[int],
) -> tuple[list[Counter[int]], Counter[int], Counter[int]]:
    """Tally symbol frequencies for Huffman tree construction.

    Processes regular commands (those with copy_length > 0) in encoding order:
      [ICC → insert literals → copy]

    The sentinel (ICC=63) always gets a count of 1.

    Then tallies flush_literals (encoded AFTER the sentinel in the bit stream)
    using per-context frequency counters, with the correct p1 value derived by
    simulating the full regular-command sequence first.

    Args:
        commands:       Commands from _build_commands (regular commands + sentinel).
        flush_literals: Trailing literal bytes encoded after the sentinel.

    Returns:
        (lit_freq, icc_freq, dist_freq)
    """
    lit_freq: list[Counter[int]] = [Counter() for _ in range(4)]
    icc_freq: Counter[int] = Counter()
    dist_freq: Counter[int] = Counter()

    history: list[int] = []   # simulated output for context tracking

    for cmd in commands:
        if cmd.copy_length == 0:
            break   # sentinel — regular commands always have copy_length > 0

        # Regular command: ICC + insert literals + copy.
        icc = _find_icc_code(cmd.insert_length, cmd.copy_length)
        icc_freq[icc] += 1
        dc = _dist_code(cmd.copy_distance)
        dist_freq[dc] += 1

        # Tally insert literals.
        for byte in cmd.literals:
            p1 = history[-1] if history else -1
            ctx = _literal_context(p1)
            lit_freq[ctx][byte] += 1
            history.append(byte)

        # Simulate copy (updates history for context tracking).
        start = len(history) - cmd.copy_distance
        for i in range(cmd.copy_length):
            history.append(history[start + i])

    # Sentinel always counts.
    icc_freq[63] += 1

    # Tally flush literals (come AFTER the sentinel in the bit stream).
    # p1 is the last byte emitted by the regular-command phase.
    p1 = history[-1] if history else -1
    for byte in flush_literals:
        ctx = _literal_context(p1)
        lit_freq[ctx][byte] += 1
        p1 = byte

    return lit_freq, icc_freq, dist_freq


# ---------------------------------------------------------------------------
# Pass 2b: Huffman tree construction
# ---------------------------------------------------------------------------

def _build_huffman_tables(
    lit_freq: list[Counter[int]],
    icc_freq: Counter[int],
    dist_freq: Counter[int],
) -> tuple[list[dict[int, str]], dict[int, str], dict[int, str]]:
    """Build canonical Huffman code tables from frequency counters."""
    lit_tables: list[dict[int, str]] = []
    for ctx in range(4):
        if lit_freq[ctx]:
            tree = HuffmanTree.build(list(lit_freq[ctx].items()))
            lit_tables.append(tree.canonical_code_table())
        else:
            lit_tables.append({})

    icc_tree = HuffmanTree.build(list(icc_freq.items()))
    icc_table = icc_tree.canonical_code_table()

    dist_table: dict[int, str] = {}
    if dist_freq:
        dist_tree = HuffmanTree.build(list(dist_freq.items()))
        dist_table = dist_tree.canonical_code_table()

    return lit_tables, icc_table, dist_table


# ---------------------------------------------------------------------------
# Pass 2c: Bit stream encoding
# ---------------------------------------------------------------------------

def _encode_commands(
    commands: list[_Command],
    flush_literals: list[int],
    lit_tables: list[dict[int, str]],
    icc_table: dict[int, str],
    dist_table: dict[int, str],
) -> str:
    """Encode all commands into a bit string.

    Encoding order per regular command (copy_length > 0):
      [ICC][insert extras][copy extras][literals][dist][dist extras]

    End of stream:
      [ICC=63]  [flush literals, if any]

    Regular commands always have copy_length >= 4 (the minimum ICC copy range).
    The sentinel is the only command with copy_length == 0.

    Args:
        commands:       Command list from _build_commands (regular + sentinel).
        flush_literals: Trailing literals to emit AFTER the sentinel.
        lit_tables:     Four per-context literal Huffman tables.
        icc_table:      ICC Huffman table.
        dist_table:     Distance Huffman table.
    """
    bits = ""
    history: list[int] = []   # simulated output bytes for p1 tracking

    for cmd in commands:
        if cmd.copy_length == 0:
            # Sentinel — end of ICC command stream.
            bits += icc_table[63]

            # Emit flush literals after the sentinel.
            # p1 is the last byte in history (or -1 at start of stream).
            p1_flush = history[-1] if history else -1
            for byte in flush_literals:
                ctx = _literal_context(p1_flush)
                bits += lit_tables[ctx][byte]
                p1_flush = byte

            break   # sentinel terminates the loop

        # Regular command (copy_length > 0, i.e., a real ICC command).
        icc = _find_icc_code(cmd.insert_length, cmd.copy_length)
        ib, ie, cb, ce = _ICC_TABLE[icc]

        bits += icc_table[icc]
        if ie > 0:
            bits += format(cmd.insert_length - ib, f"0{ie}b")[::-1]
        if ce > 0:
            bits += format(cmd.copy_length - cb, f"0{ce}b")[::-1]

        # Encode insert literals (one per-context Huffman code each).
        for byte in cmd.literals:
            p1 = history[-1] if history else -1
            bits += lit_tables[_literal_context(p1)][byte]
            history.append(byte)

        # Encode distance (code + extra bits).
        dc = _dist_code(cmd.copy_distance)
        de = _DIST_EXTRA[dc]
        bits += dist_table[dc]
        if de > 0:
            bits += format(cmd.copy_distance - _DIST_BASE[dc], f"0{de}b")[::-1]

        # Simulate copy to keep history up-to-date for p1 tracking.
        start = len(history) - cmd.copy_distance
        for i in range(cmd.copy_length):
            history.append(history[start + i])

    return bits


# ---------------------------------------------------------------------------
# Wire format helpers
# ---------------------------------------------------------------------------

def _sorted_lengths_1b(table: dict[int, str]) -> list[tuple[int, int]]:
    """Sorted [(symbol, code_length)] for 1-byte (ICC/dist) tables."""
    return sorted(((s, len(c)) for s, c in table.items()), key=lambda p: (p[1], p[0]))


def _sorted_lengths_2b(table: dict[int, str]) -> list[tuple[int, int]]:
    """Sorted [(symbol, code_length)] for 2-byte (literal) tables."""
    return sorted(((s, len(c)) for s, c in table.items()), key=lambda p: (p[1], p[0]))


# ---------------------------------------------------------------------------
# Public API: compress
# ---------------------------------------------------------------------------

def compress(data: bytes | bytearray) -> bytes:
    """Compress *data* using Brotli (CMP06) and return wire-format bytes.

    Algorithm
    ---------
    Pass 1: LZ matching → commands (including optional flush command).
    Pass 2a: Frequency counting (literals, ICC codes, distance codes).
    Pass 2b: Huffman tree construction (4 literal trees + ICC + dist).
    Pass 2c: Bit stream encoding:
        [ICC+extras+literals+dist] per regular command
        [ICC=63] [flush literals]  at the end
    Wire format: 10-byte header + tables + bit stream.

    Parameters
    ----------
    data : bytes | bytearray
        Raw bytes to compress.

    Returns
    -------
    bytes
        CMP06 wire-format bytes.
    """
    data = bytes(data)
    original_length = len(data)

    if original_length == 0:
        header = struct.pack(">IBBBBBB", 0, 1, 0, 0, 0, 0, 0)
        return header + bytes([63, 1, 0x00])

    # Pass 1: LZ matching → (regular commands + sentinel, flush_literals).
    # Regular commands have copy_length >= 4; flush_literals are trailing
    # bytes that couldn't be bundled into an ICC command (encoded AFTER
    # the sentinel ICC=63 in the bit stream).
    commands, flush_literals = _build_commands(data)

    # Pass 2a: Tally symbol frequencies.
    lit_freq, icc_freq, dist_freq = _tally_frequencies(commands, flush_literals)

    # Pass 2b: Build canonical Huffman tables.
    lit_tables, icc_table, dist_table = _build_huffman_tables(
        lit_freq, icc_freq, dist_freq
    )

    # Pass 2c: Encode bit stream.
    bits = _encode_commands(
        commands, flush_literals,
        lit_tables, icc_table, dist_table
    )
    bit_bytes = _pack_bits_lsb_first(bits)

    # Wire format.
    icc_lengths  = _sorted_lengths_1b(icc_table)
    dist_lengths = _sorted_lengths_1b(dist_table)
    lit_lengths  = [_sorted_lengths_2b(lit_tables[ctx]) for ctx in range(4)]

    header = struct.pack(
        ">IBBBBBB",
        original_length,
        len(icc_lengths), len(dist_lengths),
        len(lit_lengths[0]), len(lit_lengths[1]),
        len(lit_lengths[2]), len(lit_lengths[3]),
    )

    icc_bytes  = b"".join(bytes([s, c]) for s, c in icc_lengths)
    dist_bytes = b"".join(bytes([s, c]) for s, c in dist_lengths)
    lit_bytes  = b"".join(
        b"".join(struct.pack(">HB", s, c) for s, c in lit_lengths[ctx])
        for ctx in range(4)
    )

    return header + icc_bytes + dist_bytes + lit_bytes + bit_bytes


# ---------------------------------------------------------------------------
# Public API: decompress
# ---------------------------------------------------------------------------

def decompress(data: bytes | bytearray) -> bytes:
    """Decompress CMP06 wire-format *data* and return the original bytes.

    Algorithm
    ---------
    1. Parse the 10-byte header.
    2. Parse code-length tables (ICC, dist, 4 literal contexts).
    3. Reconstruct canonical Huffman codes.
    4. Unpack the bit stream LSB-first.
    5. Decode loop:
         a. Read ICC symbol.
         b. If ICC == 63: read any remaining flush literals, then break.
         c. Read insert_length and copy_length from extras.
         d. Read insert_length literals from per-context trees.
         e. If copy_length > 0: read dist code + extras, copy bytes.
    6. Return output trimmed to original_length.

    Notes
    -----
    The flush phase: after reading ICC=63, the decoder reads
    (original_length - len(output)) more literals from the context trees.
    These were written by the encoder AFTER the sentinel in the bit stream.

    Overlapping copies (distance < length) are handled byte-by-byte.
    """
    data = bytes(data)
    if len(data) < 10:
        return b""

    original_length = struct.unpack(">I", data[0:4])[0]
    icc_count  = data[4]
    dist_count = data[5]
    ctx_counts = [data[6], data[7], data[8], data[9]]

    if original_length == 0:
        return b""

    offset = 10

    # Parse ICC table.
    icc_lengths: list[tuple[int, int]] = []
    for _ in range(icc_count):
        icc_lengths.append((data[offset], data[offset + 1]))
        offset += 2

    # Parse dist table.
    dist_lengths: list[tuple[int, int]] = []
    for _ in range(dist_count):
        dist_lengths.append((data[offset], data[offset + 1]))
        offset += 2

    # Parse literal tables.
    lit_lengths: list[list[tuple[int, int]]] = []
    for ctx in range(4):
        lst: list[tuple[int, int]] = []
        for _ in range(ctx_counts[ctx]):
            sym  = struct.unpack(">H", data[offset: offset + 2])[0]
            clen = data[offset + 2]
            lst.append((sym, clen))
            offset += 3
        lit_lengths.append(lst)

    # Reconstruct canonical codes.
    icc_codes  = _reconstruct_canonical_codes(icc_lengths)
    dist_codes = _reconstruct_canonical_codes(dist_lengths)
    lit_codes  = [_reconstruct_canonical_codes(lit_lengths[ctx]) for ctx in range(4)]

    # Unpack bit stream.
    bits = _unpack_bits_lsb_first(data[offset:])
    bit_pos = 0

    def read_bits(n: int) -> int:
        nonlocal bit_pos
        val = 0
        for i in range(n):
            val |= int(bits[bit_pos + i]) << i
        bit_pos += n
        return val

    def next_symbol(code_table: dict[str, int]) -> int:
        nonlocal bit_pos
        acc = ""
        while True:
            acc += bits[bit_pos]
            bit_pos += 1
            if acc in code_table:
                return code_table[acc]

    # Decode loop.
    output: list[int] = []
    p1 = -1

    while True:
        icc = next_symbol(icc_codes)

        if icc == 63:
            # End-of-data. Decode any flush literals that follow.
            # The encoder wrote flush literals AFTER the sentinel.
            while len(output) < original_length:
                ctx  = _literal_context(p1)
                byte = next_symbol(lit_codes[ctx])
                output.append(byte)
                p1 = byte
            break

        ie = _ICC_INSERT_EXTRA[icc]
        ce = _ICC_COPY_EXTRA[icc]
        insert_length = _ICC_INSERT_BASE[icc] + read_bits(ie)
        copy_length   = _ICC_COPY_BASE[icc]   + read_bits(ce)

        # Decode insert literals.
        for _ in range(insert_length):
            ctx  = _literal_context(p1)
            byte = next_symbol(lit_codes[ctx])
            output.append(byte)
            p1 = byte

        # Decode copy.
        if copy_length > 0:
            dc   = next_symbol(dist_codes)
            dist = _DIST_BASE[dc] + read_bits(_DIST_EXTRA[dc])
            start = len(output) - dist
            for _ in range(copy_length):
                b = output[start]
                output.append(b)
                p1 = b
                start += 1

    return bytes(output[:original_length])
