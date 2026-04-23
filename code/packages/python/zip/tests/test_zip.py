"""Tests for the coding_adventures_zip package — CMP09 ZIP format.

Covers TC-1 through TC-12 from the CMP09 specification, plus CRC-32 known
vectors, DEFLATE round-trips, dos_datetime encoding, and read_by_name.
"""
from __future__ import annotations

import pytest

from coding_adventures_zip import (
    DOS_EPOCH,
    ZipEntry,
    ZipReader,
    ZipWriter,
    crc32,
    dos_datetime,
    unzip,
    zip_bytes,
)


# =============================================================================
# CRC-32
# =============================================================================


def test_crc32_known_value() -> None:
    # CRC-32 of "hello world" — verified against Python's binascii.crc32().
    assert crc32(b"hello world") == 0x0D4A_1185
    # Standard test vector: CRC-32 of "123456789" = 0xCBF43926.
    assert crc32(b"123456789") == 0xCBF4_3926


def test_crc32_empty() -> None:
    assert crc32(b"") == 0x0000_0000


def test_crc32_incremental() -> None:
    full = crc32(b"hello world")
    part1 = crc32(b"hello ")
    part2 = crc32(b"world", part1)
    assert part2 == full


# =============================================================================
# DEFLATE round-trips (internal, via ZIP)
# =============================================================================


def _deflate_rt(data: bytes) -> None:
    """Round-trip data through DEFLATE inside a ZIP archive."""
    archive = zip_bytes([("f.bin", data)], compress=True)
    files = unzip(archive)
    assert files["f.bin"] == data, "DEFLATE round-trip mismatch"


def test_deflate_empty() -> None:
    _deflate_rt(b"")


def test_deflate_single_byte() -> None:
    _deflate_rt(b"A")


def test_deflate_all_bytes() -> None:
    _deflate_rt(bytes(range(256)))


def test_deflate_repetitive() -> None:
    data = b"ABCABCABC" * 100
    archive = zip_bytes([("f.bin", data)], compress=True)
    files = unzip(archive)
    assert files["f.bin"] == data
    # Repetitive data must compress.
    assert len(archive) < len(data), "DEFLATE must compress repetitive data"


# =============================================================================
# TC-1: Round-trip single file (Stored)
# =============================================================================


def test_zip_stored_roundtrip() -> None:
    data = b"hello, world"
    w = ZipWriter()
    w.add_file("hello.txt", data, compress=False)
    archive = w.finish()
    files = unzip(archive)
    assert len(files) == 1
    assert files["hello.txt"] == data


# =============================================================================
# TC-2: Round-trip single file (DEFLATE)
# =============================================================================


def test_zip_deflate_roundtrip() -> None:
    text = b"the quick brown fox jumps over the lazy dog " * 10
    archive = zip_bytes([("text.txt", text)])
    files = unzip(archive)
    assert files["text.txt"] == text


# =============================================================================
# TC-3: Multiple files in one archive
# =============================================================================


def test_zip_multiple_files() -> None:
    all_bytes = bytes(range(256))
    entries: list[tuple[str, bytes]] = [
        ("a.txt", b"file A content"),
        ("b.txt", b"file B content"),
        ("c.bin", all_bytes),
    ]
    archive = zip_bytes(entries)
    files = unzip(archive)
    assert len(files) == 3
    for name, data in entries:
        assert files[name] == data, f"mismatch for {name}"


# =============================================================================
# TC-4: Directory entry
# =============================================================================


def test_zip_directory_entry() -> None:
    w = ZipWriter()
    w.add_directory("mydir/")
    w.add_file("mydir/file.txt", b"contents", compress=True)
    archive = w.finish()

    reader = ZipReader(archive)
    names = {e.name for e in reader.entries()}
    assert "mydir/" in names, "directory entry missing"
    assert "mydir/file.txt" in names, "file inside dir missing"

    dir_entry = next(e for e in reader.entries() if e.name == "mydir/")
    assert dir_entry.is_directory


# =============================================================================
# TC-5: CRC-32 mismatch detected
# =============================================================================


def test_zip_crc_mismatch_detected() -> None:
    archive = zip_bytes([("f.txt", b"test data")])
    corrupted = bytearray(archive)
    # Offset 35 = 30-byte fixed header + 5-byte name "f.txt".
    # Corrupting the actual data byte changes what we decompress.
    corrupted[35] ^= 0xFF

    with pytest.raises(ValueError, match="CRC"):
        unzip(bytes(corrupted))


# =============================================================================
# TC-6: Random-access read (read single entry)
# =============================================================================


def test_zip_random_access() -> None:
    entries = [(f"f{i}.txt", f"content {i}".encode()) for i in range(10)]
    archive = zip_bytes(entries)

    reader = ZipReader(archive)
    entry5 = next(e for e in reader.entries() if e.name == "f5.txt")
    data5 = reader.read(entry5)
    assert data5 == b"content 5"


# =============================================================================
# TC-7: Incompressible data stored without compression
# =============================================================================


def test_zip_incompressible_stored() -> None:
    # LCG pseudo-random data (seed=42): compresses poorly.
    seed = 42
    data_list: list[int] = []
    for _ in range(1024):
        seed = (seed * 1_664_525 + 1_013_904_223) & 0xFFFF_FFFF
        data_list.append(seed >> 24)
    data = bytes(data_list)

    archive = zip_bytes([("random.bin", data)])
    reader = ZipReader(archive)
    entry = next(e for e in reader.entries() if e.name == "random.bin")
    # DEFLATE would make it larger, so method must be 0 (Stored).
    assert entry.method == 0, f"expected Stored (0), got {entry.method}"
    assert reader.read(entry) == data


# =============================================================================
# TC-8: Empty file
# =============================================================================


def test_zip_empty_file() -> None:
    archive = zip_bytes([("empty.txt", b"")])
    files = unzip(archive)
    assert files["empty.txt"] == b""


# =============================================================================
# TC-9: Large file with compression
# =============================================================================


def test_zip_large_file_compressed() -> None:
    data = b"abcdefghij" * 10_000  # 100 KB
    archive = zip_bytes([("big.bin", data)])
    files = unzip(archive)
    assert files["big.bin"] == data
    assert len(archive) < len(data), (
        f"repetitive 100 KB must compress: archive={len(archive)} data={len(data)}"
    )


# =============================================================================
# TC-10: Unicode filename
# =============================================================================


def test_zip_unicode_filename() -> None:
    archive = zip_bytes([("日本語/résumé.txt", b"content")])
    files = unzip(archive)
    assert "日本語/résumé.txt" in files
    assert files["日本語/résumé.txt"] == b"content"


# =============================================================================
# TC-11: Nested paths
# =============================================================================


def test_zip_nested_paths() -> None:
    entries: list[tuple[str, bytes]] = [
        ("root.txt", b"root"),
        ("dir/file.txt", b"nested"),
        ("dir/sub/deep.txt", b"deep"),
    ]
    archive = zip_bytes(entries)
    files = unzip(archive)
    for name, data in entries:
        assert files[name] == data, f"mismatch for {name}"


# =============================================================================
# TC-12: Empty archive
# =============================================================================


def test_zip_empty_archive() -> None:
    archive = zip_bytes([])
    files = unzip(archive)
    assert len(files) == 0


# =============================================================================
# read_by_name
# =============================================================================


def test_zip_read_by_name() -> None:
    archive = zip_bytes([("alpha.txt", b"AAA"), ("beta.txt", b"BBB")])
    reader = ZipReader(archive)
    assert reader.read_by_name("beta.txt") == b"BBB"
    with pytest.raises(ValueError, match="not found"):
        reader.read_by_name("nope.txt")


# =============================================================================
# dos_datetime / DOS_EPOCH
# =============================================================================


def test_dos_datetime_epoch() -> None:
    # 1980-01-01 00:00:00 → year_offset=0, month=1, day=1
    # date = (0<<9)|(1<<5)|1 = 33
    dt = dos_datetime(1980, 1, 1, 0, 0, 0)
    assert dt >> 16 == 33   # date field
    assert dt & 0xFFFF == 0  # time field


def test_dos_epoch_constant() -> None:
    assert DOS_EPOCH == dos_datetime(1980, 1, 1, 0, 0, 0)


# =============================================================================
# DEFLATE internals — error paths and edge cases
# =============================================================================


def test_deflate_decompress_stored_block() -> None:
    """Stored block (BTYPE=00) round-trip through raw DEFLATE bytes."""
    # Build a minimal stored block manually:
    # BFINAL=1 (bit), BTYPE=00 (2 bits), align, LEN, NLEN, data.
    # In the bit-stream: BFINAL=1 BTYPE=00 in the low 3 bits of the first byte.
    # bit layout: bit0=BFINAL, bit1-2=BTYPE → 0b001 = 0x01 in the first byte,
    # remaining 5 bits are padding.
    payload = b"stored"
    n = len(payload)
    # First byte encodes BFINAL=1, BTYPE=00 (00 in bits 1-2) → 0x01.
    # Then LEN as LE u16, NLEN as LE u16, then data.
    block = bytes([0x01]) + n.to_bytes(2, "little") + (n ^ 0xFFFF).to_bytes(2, "little") + payload

    from coding_adventures_zip import _deflate_decompress  # type: ignore[attr-defined]
    result = _deflate_decompress(block)
    assert result == payload


def test_deflate_compress_empty_direct() -> None:
    """_deflate_compress(b'') produces a valid stored block that decompresses."""
    from coding_adventures_zip import _deflate_compress, _deflate_decompress  # type: ignore[attr-defined]
    compressed = _deflate_compress(b"")
    result = _deflate_decompress(compressed)
    assert result == b""


def test_deflate_decompress_btype10_raises() -> None:
    """BTYPE=10 (dynamic Huffman) is not supported."""
    from coding_adventures_zip import _deflate_decompress  # type: ignore[attr-defined]
    # BFINAL=1, BTYPE=10 → bits 0-2 = 0b101 = 0x05
    with pytest.raises(ValueError, match="BTYPE=10"):
        _deflate_decompress(bytes([0x05]))


def test_deflate_decompress_btype11_raises() -> None:
    """BTYPE=11 (reserved) is not supported."""
    from coding_adventures_zip import _deflate_decompress  # type: ignore[attr-defined]
    # BFINAL=1, BTYPE=11 → bits 0-2 = 0b111 = 0x07
    with pytest.raises(ValueError, match="reserved BTYPE"):
        _deflate_decompress(bytes([0x07]))


def test_deflate_decompress_eof_raises() -> None:
    """Empty input raises on missing BFINAL."""
    from coding_adventures_zip import _deflate_decompress  # type: ignore[attr-defined]
    with pytest.raises(ValueError, match="EOF"):
        _deflate_decompress(b"")


# =============================================================================
# ZipReader — error paths
# =============================================================================


def test_zipreader_no_eocd_raises() -> None:
    """ZipReader raises ValueError when data has no EOCD signature."""
    with pytest.raises(ValueError, match="End of Central Directory"):
        ZipReader(b"\x00" * 100)


def test_zipreader_too_short_raises() -> None:
    """ZipReader raises on data shorter than minimum EOCD."""
    with pytest.raises(ValueError, match="End of Central Directory"):
        ZipReader(b"\x00" * 10)


def test_zipreader_unsupported_method_raises() -> None:
    """Reading an entry with unsupported compression method raises ValueError."""
    archive = zip_bytes([("f.txt", b"hello")])
    reader = ZipReader(archive)
    entry = reader.entries()[0]
    # Forge an entry with unsupported method.
    bad_entry = ZipEntry(
        name=entry.name,
        size=entry.size,
        compressed_size=entry.compressed_size,
        method=12,  # bzip2 — not supported
        crc32=entry.crc32,
        is_directory=False,
        local_offset=entry.local_offset,
    )
    with pytest.raises(ValueError, match="unsupported compression method"):
        reader.read(bad_entry)


def test_zipreader_read_directory_returns_empty() -> None:
    """Reading a directory entry returns empty bytes."""
    w = ZipWriter()
    w.add_directory("empty_dir/")
    archive = w.finish()
    reader = ZipReader(archive)
    dir_entry = next(e for e in reader.entries() if e.is_directory)
    assert reader.read(dir_entry) == b""


def test_zip_align_with_partial_bits() -> None:
    """_BitReader.align() is exercised via the stored-block decompressor path."""
    # Stored block decompression calls br.align() after reading BFINAL+BTYPE.
    # This exercises the align() path when bits % 8 != 0.
    from coding_adventures_zip import _deflate_compress, _deflate_decompress  # type: ignore[attr-defined]
    data = b"hello"
    compressed = _deflate_compress(data)
    # The fixed Huffman compressed stream doesn't use align() during decompress.
    # We can manually create a stored block that hits align() mid-stream.
    payload = b"abc"
    n = len(payload)
    # Stored block (BFINAL=1, BTYPE=00): byte = 0x01 (1 in bits 0-2, rest padding)
    block = bytes([0x01]) + n.to_bytes(2, "little") + (n ^ 0xFFFF).to_bytes(2, "little") + payload
    assert _deflate_decompress(block) == payload
