"""Tests for coding_adventures_deflate (CMP05 DEFLATE compression)."""

from __future__ import annotations

import pytest
from coding_adventures_deflate import compress, decompress


# ---------------------------------------------------------------------------
# Round-trip helpers
# ---------------------------------------------------------------------------


def roundtrip(data: bytes) -> None:
    """Assert compress → decompress is lossless."""
    compressed = compress(data)
    assert decompress(compressed) == data


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------


def test_empty():
    """Empty input compresses and decompresses to empty bytes."""
    compressed = compress(b"")
    assert decompress(compressed) == b""


def test_single_byte():
    """A single byte round-trips correctly."""
    roundtrip(b"\x00")
    roundtrip(b"\xff")
    roundtrip(b"A")


def test_single_byte_repeated():
    """A run of the same byte (run-length encoded by overlapping match)."""
    roundtrip(b"A" * 20)
    roundtrip(b"\x00" * 100)


def test_all_literals_aaabbc():
    """Spec example 'AAABBC' — no LZSS matches, only literals."""
    data = b"AAABBC"
    roundtrip(data)
    compressed = compress(data)
    # Verify: dist_entry_count should be 0 (no matches)
    import struct
    _, ll_count, dist_count = struct.unpack(">IHH", compressed[:8])
    assert dist_count == 0
    assert ll_count > 0


def test_one_match_aabcbbabc():
    """Spec example 'AABCBBABC' — one LZSS match."""
    data = b"AABCBBABC"
    roundtrip(data)
    compressed = compress(data)
    import struct
    _, ll_count, dist_count = struct.unpack(">IHH", compressed[:8])
    assert dist_count > 0  # has a match


def test_overlapping_match():
    """Overlapping match: offset < length encodes a run."""
    # "AAAAAAA": after Lit('A'), Match(offset=1, length=6)
    roundtrip(b"AAAAAAA")
    roundtrip(b"ABABABABABAB")


def test_multiple_matches():
    """Multiple back-references."""
    roundtrip(b"ABCABCABCABC")
    roundtrip(b"hello hello hello world")


def test_all_bytes():
    """All 256 possible byte values."""
    roundtrip(bytes(range(256)))


def test_binary_data():
    """Binary data with no ASCII bias."""
    data = bytes([i % 256 for i in range(1000)])
    roundtrip(data)


def test_longer_text():
    """Longer text with significant repetition."""
    text = b"the quick brown fox jumps over the lazy dog " * 10
    roundtrip(text)


def test_lorem_ipsum():
    """Lorem ipsum — realistic text compression."""
    text = (
        b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
        b"Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. "
        b"Ut enim ad minim veniam, quis nostrud exercitation ullamco. " * 5
    )
    roundtrip(text)


def test_compression_ratio():
    """Highly repetitive data should compress to less than 50% original size."""
    data = b"ABCABC" * 100  # 600 bytes, very repetitive
    compressed = compress(data)
    assert len(compressed) < len(data) * 0.5, (
        f"Expected significant compression: {len(compressed)} < {len(data) * 0.5}"
    )


def test_header_format():
    """Verify wire format header bytes."""
    import struct
    data = b"AAABBC"
    compressed = compress(data)
    original_len, ll_count, dist_count = struct.unpack(">IHH", compressed[:8])
    assert original_len == 6
    assert ll_count > 0
    assert dist_count == 0  # no matches in "AAABBC"


def test_aabcbbabc_header():
    """Verify wire format for the spec example with a match."""
    import struct
    data = b"AABCBBABC"
    compressed = compress(data)
    original_len, ll_count, dist_count = struct.unpack(">IHH", compressed[:8])
    assert original_len == 9
    assert ll_count > 0
    assert dist_count > 0


def test_decompress_compressed_output():
    """Decompress the output of compress — basic sanity check."""
    for data in [b"", b"A", b"hello", b"DEFLATE" * 50]:
        assert decompress(compress(data)) == data


def test_window_size():
    """Large data spanning multiple window sizes."""
    data = bytes(range(256)) * 20  # 5120 bytes
    roundtrip(data)


def test_max_match_length():
    """Matches at the maximum length boundary (255)."""
    data = b"A" * 300  # will produce a Match(offset=1, length=255) + more
    roundtrip(data)


def test_distance_near_boundary():
    """Offsets near distance code boundaries."""
    # distance 4 = code 3 (base 4, extra 0)
    # distance 5 = code 4 (base 5, extra 1)
    data = b"ABCD" + b"X" * 10 + b"ABCD"  # offset ~14 to the first ABCD
    roundtrip(data)


def test_single_match_various_lengths():
    """Test various match lengths to exercise the length code table."""
    for length in [3, 4, 10, 11, 13, 19, 35, 67, 131, 227, 255]:
        # Create data where a match of exactly `length` occurs.
        prefix = b"A" * length
        data = prefix + b"B" * 3 + prefix
        roundtrip(data)


def test_no_regression_compress_decompress():
    """Round-trip a diverse set of inputs."""
    inputs = [
        b"\x00" * 100,
        b"\xff" * 100,
        b"The quick brown fox " * 20,
        bytes(range(256)),
        b"abcdefghijklmnopqrstuvwxyz" * 10,
    ]
    for inp in inputs:
        roundtrip(inp)
