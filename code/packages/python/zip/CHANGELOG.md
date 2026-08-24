# Changelog — coding-adventures-zip

## [0.2.1] — 2026-08-21

### Changed

- Large raw-DEFLATE inputs now avoid the educational boxed LZSS token path.
  Incompressible blocks use RFC 1951 stored framing, while repetitive blocks
  use fixed-Huffman output with constant-size match state. This keeps PNG and
  other bounded consumers from turning byte-sized ceilings into multi-gigabyte
  token allocations or exhaustive match scans.
- `raw_deflate` now accepts `bytes`, `bytearray`, and byte-oriented
  `memoryview` inputs without requiring callers to copy large buffers first.
- A deterministic 2 MiB incompressible regression proves that the large-input
  path bypasses LZSS token materialization and remains compatible with a
  foreign zlib inflater.

## [0.2.0] — 2026-08-13

### Added

- Public `raw_deflate`, `raw_inflate`, and `raw_inflate_counted` APIs for the
  portable raw RFC 1951 profile, including exact compressed-byte consumption.
- Stored, fixed-Huffman, dynamic-Huffman, multi-block, symbol-285, and full
  32 KiB overlapping back-reference decode support.
- Typed `RawInflateError` failures with 14 stable, payload-blind error IDs and
  a caller-lowerable 256 MiB output ceiling.
- All 34 language-neutral conformance vectors plus dynamic ZIP, suffix-cavity,
  declared-size, compatibility-wrapper, and full-window integration coverage.
- Explicit empty capability metadata for the pure in-memory production library.

### Changed

- ZIP method 8 reads now enforce exact compressed consumption and declared
  uncompressed size before CRC verification instead of trimming excess output.
- Package BUILD entry points now run Ruff, strict MyPy, branch coverage, and
  the complete pytest suite on Unix and Windows.

## [0.1.0] — 2026-04-23

### Added

- `ZipWriter` — incremental in-memory ZIP writer with `add_file`, `add_directory`, `finish`.
- `ZipReader` — EOCD-first random-access ZIP reader with `entries`, `read`, `read_by_name`.
- `ZipEntry` — metadata dataclass for a single archive entry.
- `zip_bytes(entries)` — convenience function to create a ZIP from a list of `(name, data)` pairs.
- `unzip(data)` — convenience function to extract all files as a `dict[str, bytes]`.
- `crc32(data, initial)` — table-driven CRC-32 (polynomial 0xEDB88320), supports incremental updates.
- `dos_datetime(year, month, day, hour, minute, second)` — MS-DOS timestamp encoder.
- `DOS_EPOCH` — fixed timestamp constant `0x00210000` (1980-01-01 00:00:00).
- Raw RFC 1951 DEFLATE codec (fixed Huffman BTYPE=01) inlined; depends on `coding-adventures-lzss` for LZ77 tokenization with a 32 KB window.
- Auto-compression: DEFLATE is used only if the output is strictly smaller than the original.
- Security: 256 MB decompression bomb cap; encrypted entries raise `ValueError`.
- 95%+ test coverage across TC-1 through TC-12 plus CRC-32 and DEFLATE round-trip tests.
