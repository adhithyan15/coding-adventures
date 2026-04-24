# Changelog — kotlin/zip

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the ZIP archive format (CMP09) in Kotlin.
- `ZipWriter` — incremental in-memory writer supporting DEFLATE (method 8) and
  Stored (method 0) entries; file and directory entries.
- `ZipReader` — EOCD-first parser with random-access decompression and CRC-32
  verification.
- `ZipArchive` — convenience `zip()` and `unzip()` one-shot functions.
- `ZipEntry` — data class carrying `name` and `data` fields.
- `crc32()` — table-driven CRC-32 (polynomial 0xEDB88320) with incremental support.
- `dosDt()` — MS-DOS packed datetime encoder.
- Internal `deflateCompress()` / `deflateDecompress()` — raw RFC 1951 DEFLATE
  (fixed Huffman BTYPE=01) using the `lzss` package for LZ77 match-finding.
- Full test suite: TC-01 through TC-12 matching the Rust and C# reference
  implementations, plus CRC-32 and DEFLATE unit tests.
- Security limits: 256 MB decompression bomb guard, LEN/NLEN validation,
  CRC-32 mismatch detection.
