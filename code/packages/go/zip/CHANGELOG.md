# Changelog — coding-adventures/go/zip

## [0.1.0] — 2026-04-23

### Added

- `ZipWriter` — in-memory ZIP writer: `AddFile`, `AddDirectory`, `Finish`.
- `ZipReader` — EOCD-first random-access reader: `Entries`, `Read`, `ReadByName`.
- `ZipEntry` — metadata struct for a single archive entry.
- `Zip(entries)` / `Unzip(data)` — convenience functions.
- `CRC32(data, initial)` — table-driven CRC-32 (polynomial 0xEDB88320).
- `DOSDatetime` / `DOSEpoch` — MS-DOS timestamp encoder and epoch constant.
- Raw RFC 1951 DEFLATE (fixed Huffman BTYPE=01) inlined; uses `lzss` for LZ77 tokenization with a 32 KB window.
- Auto-compression: DEFLATE only when output < original.
- 256 MB decompression bomb cap; encrypted-entry rejection.
- All 12 spec test cases (TC-1 through TC-12) pass.
