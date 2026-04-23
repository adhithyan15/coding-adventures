# Changelog — coding-adventures-zip

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
