# Changelog — coding-adventures/go/zip

## [0.2.0] — 2026-08-13

### Added

- Public `RawDeflate`, `RawInflate`, and `RawInflateCounted` RFC 1951 APIs.
- Strict stored, fixed, dynamic, and multi-block decoding with exact byte
  consumption, caller-lowerable output caps, full-window overlapping matches,
  symbol 285, and RFC-correct 32-slot dynamic distance headers.
- Stable typed `RawInflateError` failures for the 14 language-neutral error IDs.
- All 34 `zip-raw-rfc1951-v1` fixture cases, independent standard-library decoder
  interoperability, dynamic ZIP reading, and covert suffix-cavity rejection.
- Explicit empty capability metadata for the pure in-memory production package.

### Changed

- `ZipReader.Read` now requires exact compressed-payload consumption and exact
  declared uncompressed size instead of silently trimming excess output.

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
