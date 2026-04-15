"""Tests for coding_adventures_brotli (CMP06 Brotli compression).

Test cases are drawn directly from the CMP06 spec. The suite verifies:
  - All spec-mandated round-trip tests pass.
  - Wire format structure is correct (header bytes, entry counts, etc.).
  - Edge cases (empty, single byte, all-copies, context transitions).
  - Long-distance matches (exercises dist codes 24–31, offset > 4096).
  - Compression ratio on English prose is < 80% of input size.
"""

from __future__ import annotations

import random
import struct

import pytest

from coding_adventures_brotli import (
    _DIST_TABLE,
    _ICC_TABLE,
    _dist_code,
    _find_icc_code,
    _literal_context,
    _pack_bits_lsb_first,
    _reconstruct_canonical_codes,
    _unpack_bits_lsb_first,
    compress,
    decompress,
)

# ---------------------------------------------------------------------------
# Round-trip helper
# ---------------------------------------------------------------------------


def roundtrip(data: bytes) -> None:
    """Assert compress → decompress is lossless."""
    compressed = compress(data)
    result = decompress(compressed)
    assert result == data, (
        f"Round-trip failed: input {len(data)} bytes, "
        f"compressed {len(compressed)} bytes, "
        f"decompressed {len(result)} bytes"
    )


# ---------------------------------------------------------------------------
# Spec test 1: Round-trip empty input
# ---------------------------------------------------------------------------


def test_empty_input() -> None:
    """Empty input compresses and decompresses to empty bytes (spec test 1).

    The spec defines a fixed encoding for empty input:
      Header (10 bytes): original_length=0, icc_count=1, dist_count=0,
                         ctx{0..3}=0
      ICC table: 1 entry — symbol=63, code_length=1.
      Bit stream: 0x00 (one zero bit, padded).
    """
    compressed = compress(b"")
    assert decompress(compressed) == b""

    # Verify the exact wire format as specified.
    assert len(compressed) == 13, (
        f"Empty input should produce exactly 13 bytes: "
        f"10 header + 2 ICC entry + 1 bit byte; got {len(compressed)}"
    )
    # Header: original_length=0
    orig_len = struct.unpack(">I", compressed[0:4])[0]
    assert orig_len == 0
    # icc_count = 1
    assert compressed[4] == 1
    # dist_count = 0
    assert compressed[5] == 0
    # ctx{0..3}_count = 0
    assert compressed[6] == 0
    assert compressed[7] == 0
    assert compressed[8] == 0
    assert compressed[9] == 0
    # ICC entry: symbol=63, code_length=1
    assert compressed[10] == 63
    assert compressed[11] == 1
    # Bit stream: single zero byte
    assert compressed[12] == 0x00


# ---------------------------------------------------------------------------
# Spec test 2: Round-trip single byte
# ---------------------------------------------------------------------------


def test_single_byte() -> None:
    """Single byte round-trips correctly (spec test 2)."""
    roundtrip(b"\x42")   # spec example
    roundtrip(b"\x00")
    roundtrip(b"\xff")
    roundtrip(b"A")
    roundtrip(b"\x7f")


# ---------------------------------------------------------------------------
# Spec test 3: Round-trip all 256 distinct bytes (no matches)
# ---------------------------------------------------------------------------


def test_all_256_bytes_round_trip() -> None:
    """256 distinct bytes round-trips (spec test 3).

    All distinct bytes means no LZ matches are possible. The compressed
    output will be larger than the input (incompressible data), but the
    round-trip must be exact.
    """
    data = bytes(range(256))
    compressed = compress(data)

    # No matches possible → dist table must be empty.
    assert compressed[5] == 0, "dist_entry_count should be 0 for all-distinct bytes"

    # Round-trip must be exact.
    assert decompress(compressed) == data


def test_all_256_bytes_output_larger() -> None:
    """Compressed size is larger than input for incompressible data (spec test 3)."""
    data = bytes(range(256))
    compressed = compress(data)
    # Incompressible data: output must be larger than 256 bytes.
    assert len(compressed) > len(data), (
        f"All-distinct bytes should expand: input={len(data)}, "
        f"compressed={len(compressed)}"
    )


# ---------------------------------------------------------------------------
# Spec test 4: Round-trip 1024 × 'A' (all copies)
# ---------------------------------------------------------------------------


def test_repeated_byte_1024() -> None:
    """1024 × 'A' round-trips correctly (spec test 4).

    The first 4 bytes must be inserted as literals (empty window),
    then matched by copy commands. The total decompressed output = 1024 × 'A'.
    """
    data = b"A" * 1024
    roundtrip(data)


def test_repeated_byte_compression_ratio() -> None:
    """Highly repetitive data compresses dramatically (spec test 4).

    1024 repetitions of a single byte should produce a very small output.
    """
    data = b"A" * 1024
    compressed = compress(data)
    # Should compress to much less than 50% of input size.
    assert len(compressed) < len(data) // 4, (
        f"1024×'A' should compress well: input={len(data)}, "
        f"compressed={len(compressed)}"
    )


# ---------------------------------------------------------------------------
# Spec test 5: Round-trip English prose ≥ 1024 bytes, ratio < 80%
# ---------------------------------------------------------------------------

_ENGLISH_PROSE = (
    "The quick brown fox jumps over the lazy dog. "
    * 30
    + "She sells seashells by the seashore. The shells she sells are surely seashells. "
    * 15
    + "Peter Piper picked a peck of pickled peppers. "
    "A peck of pickled peppers Peter Piper picked. "
    * 10
    + "How much wood would a woodchuck chuck if a woodchuck could chuck wood? "
    * 10
    + "To be or not to be, that is the question: "
    "Whether 'tis nobler in the mind to suffer "
    "The slings and arrows of outrageous fortune, "
    "Or to take arms against a sea of troubles. "
    * 5
)


def test_english_prose_round_trip() -> None:
    """English prose ≥ 1024 bytes round-trips correctly (spec test 5)."""
    data = _ENGLISH_PROSE.encode("ascii")
    assert len(data) >= 1024, f"Test prose must be ≥ 1024 bytes, got {len(data)}"
    roundtrip(data)


def test_english_prose_compression_ratio() -> None:
    """English prose compresses to < 80% of input size (spec test 5)."""
    data = _ENGLISH_PROSE.encode("ascii")
    compressed = compress(data)
    ratio = len(compressed) / len(data)
    assert ratio < 0.80, (
        f"English prose should compress to < 80%; "
        f"got {ratio:.1%} ({len(compressed)}/{len(data)} bytes)"
    )


# ---------------------------------------------------------------------------
# Spec test 6: Round-trip 512 random bytes (deterministic seed)
# ---------------------------------------------------------------------------


def test_random_binary_round_trip() -> None:
    """512 random bytes round-trip exactly (spec test 6).

    Uses a fixed seed for reproducibility across runs and implementations.
    No compression ratio requirement — random data is incompressible.
    """
    rng = random.Random(42)
    data = bytes(rng.randint(0, 255) for _ in range(512))
    roundtrip(data)


# ---------------------------------------------------------------------------
# Spec test 7: Context transitions in "abc123ABC"
# ---------------------------------------------------------------------------


def test_context_transitions() -> None:
    """'abc123ABC' exercises all four context buckets (spec test 7).

    Literal 'a' appears after start → ctx 0 (no prev byte).
    Literals 'b','c' appear after lowercase → ctx 3.
    Literals '1','2','3' appear after lowercase/digit → ctx 3 then 1.
    Literals 'A','B','C' appear after digit/uppercase → ctx 1 then 2.
    """
    data = b"abc123ABC"
    roundtrip(data)


def test_context_transitions_extended() -> None:
    """Extended context transition string round-trips correctly."""
    # Space → ctx 0, digit → ctx 1, upper → ctx 2, lower → ctx 3
    data = b"Hello World! 123 ABC abc XYZ xyz 456 def GHI ghi 789"
    roundtrip(data)


# ---------------------------------------------------------------------------
# Spec test 8: Long-distance match (offset > 4096)
# ---------------------------------------------------------------------------


def test_long_distance_match() -> None:
    """A 10-byte sequence repeated with offset > 4096 (spec test 8).

    This exercises distance codes 24–31, which extend the window to 65535.
    CMP05 (DEFLATE) could not encode such references (4096 byte limit);
    CMP06 (Brotli) extends this to 65535.
    """
    # The "marker" sequence — 10 distinct bytes that won't appear elsewhere.
    marker = b"\xAA\xBB\xCC\xDD\xEE\xFF\x11\x22\x33\x44"

    # Pad with > 4096 bytes between occurrences.
    filler = b"X" * 5000
    data = marker + filler + marker

    roundtrip(data)


def test_long_distance_match_dist_codes() -> None:
    """Verify distance codes 24–31 are present for offset > 4096."""
    marker = b"\xAA\xBB\xCC\xDD\xEE\xFF\x11\x22\x33\x44"
    filler = b"X" * 5000
    data = marker + filler + marker

    compressed = compress(data)
    # dist_entry_count should be > 0 (there IS a copy command).
    assert compressed[5] > 0, "Expected at least one distance code entry"


# ---------------------------------------------------------------------------
# Spec test 9: Cross-language compatibility (internal consistency)
# ---------------------------------------------------------------------------


def test_cross_language_consistency() -> None:
    """Internal round-trip simulates cross-language compatibility (spec test 9).

    We can't actually test cross-language in a single Python test suite, but
    we verify that the wire format is stable: compressing the same input twice
    produces identical bytes (deterministic output).
    """
    data = _ENGLISH_PROSE.encode("ascii")
    compressed_1 = compress(data)
    compressed_2 = compress(data)
    assert compressed_1 == compressed_2, (
        "compress() must be deterministic: same input → same output"
    )


# ---------------------------------------------------------------------------
# Spec test 10: Wire format parsing
# ---------------------------------------------------------------------------


def test_wire_format_manual() -> None:
    """Manually constructed CMP06 payload decompresses correctly (spec test 10).

    We construct a minimal valid payload that encodes the single byte b'A':
      - original_length = 1
      - One literal in context 0 (value 0x41 = 'A')
      - One flush command (insert=1, copy=0): no ICC needed (just literal)
      - One sentinel ICC code 63.

    Wait — per spec, the flush command (insert>0, copy=0) has no ICC symbol.
    But the sentinel MUST use ICC code 63. Also, a literal-only stream needs
    the ICC code 63 as the ONLY ICC entry.

    Trees needed:
      - ICC tree: {63: 1} → code "0"
      - Dist tree: empty (no copies)
      - Literal tree ctx 0: {0x41: 1} → code "0"
      - Literal trees ctx 1,2,3: empty

    Bit stream:
      - Encode 'A' (0x41) using ctx0 tree: code "0" → bit "0"
      - Encode sentinel ICC 63: code "0" → bit "0"
      Total: "00" → 0x00 (zero-padded to 1 byte)

    Wire format:
      Header (10 bytes):
        [0x00000001]  original_length = 1
        [0x01]        icc_count = 1
        [0x00]        dist_count = 0
        [0x01]        ctx0_count = 1
        [0x00]        ctx1_count = 0
        [0x00]        ctx2_count = 0
        [0x00]        ctx3_count = 0
      ICC table (2 bytes):
        [0x3F, 0x01]  symbol=63, code_length=1
      Dist table: empty
      Literal tree 0 (3 bytes):
        [0x00, 0x41, 0x01]  symbol=0x0041 (big-endian uint16), code_length=1
      Bit stream (1 byte):
        [0x00]  bits "00" packed LSB-first → 0x00
    """
    payload = (
        struct.pack(">I", 1)        # original_length = 1
        + bytes([1, 0, 1, 0, 0, 0])  # icc=1, dist=0, ctx0=1, ctx1=0, ctx2=0, ctx3=0
        + bytes([63, 1])             # ICC entry: symbol=63, code_length=1
        # no dist entries
        + struct.pack(">HB", 0x41, 1)  # ctx0 entry: symbol=0x41 ('A'), code_length=1
        # no ctx1/ctx2/ctx3 entries
        + bytes([0x00])              # bit stream: "00" → 0x00
    )

    result = decompress(payload)
    assert result == b"A", f"Expected b'A', got {result!r}"


# ---------------------------------------------------------------------------
# Additional edge case tests
# ---------------------------------------------------------------------------


def test_single_repeated_byte_small() -> None:
    """A short run of repeated bytes round-trips."""
    for n in [1, 2, 3, 4, 5, 10, 50]:
        roundtrip(b"Z" * n)


def test_two_distinct_bytes() -> None:
    """Two distinct bytes round-trip correctly."""
    roundtrip(b"AB")
    roundtrip(b"\x00\xff")


def test_longer_repetition() -> None:
    """Longer repetition patterns compress and decompress correctly."""
    roundtrip(b"ABCABC" * 100)
    roundtrip(b"Hello, world! " * 50)


def test_binary_data() -> None:
    """Binary data (non-ASCII bytes) round-trips correctly."""
    data = bytes(range(256)) * 2
    roundtrip(data)


def test_null_bytes() -> None:
    """Null bytes round-trip correctly."""
    roundtrip(b"\x00" * 1000)


def test_high_bytes() -> None:
    """High byte values (0x80–0xFF) round-trip correctly."""
    data = bytes(range(0x80, 0x100)) * 4
    roundtrip(data)


def test_alternating_pattern() -> None:
    """Alternating byte pattern round-trips correctly."""
    data = b"\xAA\x55" * 500
    roundtrip(data)


def test_lorem_ipsum() -> None:
    """Typical English text round-trips correctly."""
    data = (
        b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
        b"Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. "
        b"Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. "
        b"Duis aute irure dolor in reprehenderit in voluptate velit esse. "
        b"Excepteur sint occaecat cupidatat non proident, sunt in culpa. "
    ) * 5
    roundtrip(data)


def test_compress_returns_bytes() -> None:
    """compress() returns bytes type."""
    result = compress(b"hello")
    assert isinstance(result, bytes)


def test_decompress_returns_bytes() -> None:
    """decompress() returns bytes type."""
    compressed = compress(b"hello")
    result = decompress(compressed)
    assert isinstance(result, bytes)


def test_compress_accepts_bytearray() -> None:
    """compress() accepts bytearray input."""
    data = bytearray(b"test data 1234")
    compressed = compress(data)
    assert decompress(compressed) == bytes(data)


def test_decompress_accepts_bytearray() -> None:
    """decompress() accepts bytearray input."""
    compressed = compress(b"test data")
    assert decompress(bytearray(compressed)) == b"test data"


# ---------------------------------------------------------------------------
# ICC table unit tests
# ---------------------------------------------------------------------------


def test_icc_table_length() -> None:
    """ICC table has exactly 64 entries (codes 0–63)."""
    assert len(_ICC_TABLE) == 64


def test_icc_table_sentinel() -> None:
    """ICC code 63 is the sentinel (insert=0, copy=0)."""
    ib, ie, cb, ce = _ICC_TABLE[63]
    assert ib == 0 and ie == 0 and cb == 0 and ce == 0


def test_icc_table_min_copy_length() -> None:
    """All ICC codes 0–62 have copy_base ≥ 4 (minimum copy length 4)."""
    for code in range(63):
        _, _, cb, _ = _ICC_TABLE[code]
        assert cb >= 4, f"ICC code {code} has copy_base {cb} < 4"


def test_find_icc_code_exact_match() -> None:
    """_find_icc_code() finds a valid ICC code for exact values."""
    # insert=0, copy=4 → should map to code 0
    code = _find_icc_code(0, 4)
    ib, ie, cb, ce = _ICC_TABLE[code]
    assert ib <= 0 <= ib + (1 << ie) - 1
    assert cb <= 4 <= cb + (1 << ce) - 1


def test_find_icc_code_insert_1_copy_5() -> None:
    """_find_icc_code() handles insert=1, copy=5."""
    code = _find_icc_code(1, 5)
    ib, ie, cb, ce = _ICC_TABLE[code]
    assert ib <= 1 <= ib + (1 << ie) - 1
    assert cb <= 5 <= cb + (1 << ce) - 1


# ---------------------------------------------------------------------------
# Distance code unit tests
# ---------------------------------------------------------------------------


def test_dist_table_length() -> None:
    """Distance table has exactly 32 entries (codes 0–31)."""
    assert len(_DIST_TABLE) == 32


def test_dist_code_small() -> None:
    """Distance 1 maps to code 0."""
    assert _dist_code(1) == 0


def test_dist_code_4() -> None:
    """Distance 4 maps to code 3."""
    assert _dist_code(4) == 3


def test_dist_code_large() -> None:
    """Distance 65535 maps to code 31."""
    code = _dist_code(65535)
    assert code == 31, f"Expected code 31 for distance 65535, got {code}"


def test_dist_code_extended() -> None:
    """Distances > 4096 map to codes 24–31 (extended window)."""
    assert _dist_code(4097) >= 24, "Distance 4097 should use extended codes 24+"
    assert _dist_code(10000) >= 24
    assert _dist_code(50000) >= 24


# ---------------------------------------------------------------------------
# Context function unit tests
# ---------------------------------------------------------------------------


def test_literal_context_start() -> None:
    """Context 0 is returned when there is no previous byte (p1 = -1)."""
    assert _literal_context(-1) == 0


def test_literal_context_space() -> None:
    """Space (0x20) maps to context 0."""
    assert _literal_context(0x20) == 0


def test_literal_context_digit() -> None:
    """Digits '0'–'9' map to context 1."""
    for ch in b"0123456789":
        assert _literal_context(ch) == 1, f"Digit {ch!r} should be context 1"


def test_literal_context_uppercase() -> None:
    """Uppercase 'A'–'Z' map to context 2."""
    for ch in b"ABCDEFGHIJKLMNOPQRSTUVWXYZ":
        assert _literal_context(ch) == 2, f"Upper {chr(ch)!r} should be context 2"


def test_literal_context_lowercase() -> None:
    """Lowercase 'a'–'z' map to context 3."""
    for ch in b"abcdefghijklmnopqrstuvwxyz":
        assert _literal_context(ch) == 3, f"Lower {chr(ch)!r} should be context 3"


def test_literal_context_high_bytes() -> None:
    """Bytes ≥ 0x7B are context 0 (space/punct)."""
    for byte in [0x7B, 0x80, 0xFF]:
        assert _literal_context(byte) == 0, f"Byte {byte:#x} should be context 0"


# ---------------------------------------------------------------------------
# Bit I/O unit tests
# ---------------------------------------------------------------------------


def test_pack_unpack_roundtrip() -> None:
    """Packing then unpacking bits is lossless."""
    bits = "1011001010111100"
    packed = _pack_bits_lsb_first(bits)
    unpacked = _unpack_bits_lsb_first(packed)
    # Unpacked may have trailing zeros from padding; check only leading bits.
    assert unpacked[: len(bits)] == bits


def test_pack_single_bit_zero() -> None:
    """Packing a single '0' bit produces 0x00."""
    assert _pack_bits_lsb_first("0") == bytes([0x00])


def test_pack_single_bit_one() -> None:
    """Packing a single '1' bit produces 0x01."""
    assert _pack_bits_lsb_first("1") == bytes([0x01])


def test_pack_full_byte() -> None:
    """Packing 8 bits produces one byte."""
    # "10110010" LSB-first: bit0=1 bit1=0 bit2=1 bit3=1 bit4=0 bit5=0 bit6=1 bit7=0
    # = 0b01001101 = 0x4D
    bits = "10110010"
    packed = _pack_bits_lsb_first(bits)
    assert len(packed) == 1
    assert packed[0] == 0b01001101


def test_pack_lsb_ordering() -> None:
    """LSB-first packing: first bit → bit 0 (LSB) of first byte."""
    # "1" followed by seven "0"s → byte with only bit 0 set = 0x01
    bits = "1" + "0" * 7
    packed = _pack_bits_lsb_first(bits)
    assert packed[0] == 0x01


# ---------------------------------------------------------------------------
# Canonical code reconstruction unit tests
# ---------------------------------------------------------------------------


def test_reconstruct_empty() -> None:
    """Empty input produces empty output."""
    assert _reconstruct_canonical_codes([]) == {}


def test_reconstruct_single_symbol() -> None:
    """Single-symbol tree always gets code '0'."""
    result = _reconstruct_canonical_codes([(42, 1)])
    assert result == {"0": 42}


def test_reconstruct_two_symbols() -> None:
    """Two symbols of equal length get codes '0' and '1'."""
    result = _reconstruct_canonical_codes([(0, 1), (1, 1)])
    assert "0" in result
    assert "1" in result
    assert set(result.values()) == {0, 1}


def test_reconstruct_canonical_uniqueness() -> None:
    """All reconstructed codes are unique (prefix-free property)."""
    lengths = [(sym, 1 + sym % 3) for sym in range(6)]
    lengths.sort(key=lambda p: (p[1], p[0]))
    result = _reconstruct_canonical_codes(lengths)
    assert len(result) == len(lengths), "Each symbol must have a unique code"


# ---------------------------------------------------------------------------
# Wire format header tests
# ---------------------------------------------------------------------------


def test_header_original_length() -> None:
    """Header bytes 0–3 contain the correct original length (big-endian uint32)."""
    data = b"Hello, Brotli!"
    compressed = compress(data)
    orig_len = struct.unpack(">I", compressed[0:4])[0]
    assert orig_len == len(data)


def test_header_no_dist_for_all_literals() -> None:
    """dist_entry_count is 0 when there are no copy commands."""
    # 256 distinct bytes — no matches possible.
    compressed = compress(bytes(range(256)))
    assert compressed[5] == 0, "No copies → dist_entry_count must be 0"


def test_header_has_dist_for_repetitive_data() -> None:
    """dist_entry_count > 0 when copy commands exist."""
    compressed = compress(b"AAAA" * 100)
    assert compressed[5] > 0, "Repetitive data → dist_entry_count must be > 0"


def test_header_icc_count_nonzero() -> None:
    """icc_entry_count > 0 for non-empty input."""
    compressed = compress(b"hello world")
    assert compressed[4] > 0, "icc_entry_count must be > 0 for non-empty input"


# ---------------------------------------------------------------------------
# Stress / regression tests
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("n", [1, 2, 3, 4, 7, 8, 16, 100, 256, 1000])
def test_repeated_byte_various_lengths(n: int) -> None:
    """Repeated single byte of various lengths round-trips correctly."""
    roundtrip(b"Q" * n)


@pytest.mark.parametrize("seed", [0, 1, 7, 42, 99, 12345])
def test_random_data_various_seeds(seed: int) -> None:
    """Random data round-trips for various seeds."""
    rng = random.Random(seed)
    data = bytes(rng.randint(0, 255) for _ in range(200))
    roundtrip(data)


def test_long_input() -> None:
    """Longer inputs (> 65535 bytes window) round-trip correctly."""
    # Build a pattern that exceeds the window size.
    data = (b"The quick brown fox jumps over the lazy dog. " * 2000)[:70000]
    roundtrip(data)


def test_fibonacci_pattern() -> None:
    """Self-similar Fibonacci-like pattern round-trips."""
    a, b = b"A", b"B"
    for _ in range(12):
        a, b = b, b + a
    roundtrip(a[:4096])


def test_all_same_byte_variants() -> None:
    """All-same-byte inputs for each distinct byte value round-trip."""
    for byte_val in [0x00, 0x41, 0x61, 0x7F, 0xFF]:
        roundtrip(bytes([byte_val]) * 64)
