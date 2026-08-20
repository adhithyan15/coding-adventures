# Changelog

## [0.2.0] - 2026-08-14

### Added

- Public `rawDeflate`, `rawInflate`, and `rawInflateCounted` APIs with a typed
  stable error taxonomy, exact consumed-byte reporting, and a caller-lowerable
  256 MiB output ceiling.
- Strict stored, fixed, dynamic, and multi-block RFC 1951 decoding, including
  canonical Huffman validation, HDIST 32 zero reserved slots, symbol 285, and
  the full 32 KiB overlap window.
- All 34 language-neutral raw RFC 1951/CRC-32 cases, an independent test-only
  Python/zlib oracle, dynamic ZIP integration, and full-window coverage.
- Explicit empty capability metadata for the pure in-memory production library.

### Security

- ZIP method 8 now rejects compressed suffix cavities and any declared
  uncompressed-size mismatch before CRC-32 validation instead of trimming
  excess output.
- Raw-inflate failures return no partial output and use payload-blind messages.

## [0.1.0] - 2026-04-23

### Added

- Initial implementation of the ZIP archive format (CMP09 — PKZIP 1989) in Swift.
- `ZipWriter`: creates ZIP archives incrementally in memory.
  - `addFile(_:data:compress:)`: adds a file entry, auto-compresses if beneficial.
  - `addDirectory(_:)`: adds a directory entry.
  - `finish()`: appends Central Directory and EOCD, returns `[UInt8]`.
- `ZipReader`: parses ZIP archives using EOCD-first strategy.
  - `entries()`: lists all `ZipEntry` structs.
  - `read(_:)`: decompresses and CRC-validates one entry.
  - `readByName(_:)`: convenience wrapper.
- `zip(_:compress:)`: one-shot compression.
- `unzip(_:)`: one-shot decompression → `[String: [UInt8]]`.
- `crc32(_:initial:)`: table-driven CRC-32 (polynomial 0xEDB88320), chainable.
- `dosDatetime(year:month:day:hour:minute:second:)`: MS-DOS datetime encoder.
- `dosEpoch`: constant `0x00210000` for 1980-01-01 00:00:00.
- RFC 1951 DEFLATE inlined (fixed Huffman BTYPE=01), backed by `LZSS` package for
  LZ77 tokenization (32 KB window). Cannot reuse `coding_adventures_deflate` — it
  uses a custom non-RFC-1951 wire format.
- `BitWriter`/`BitReader` as private structs for LSB-first bit I/O.
- `ZipError` enum: `.malformed`, `.crcMismatch`, `.notFound`, `.unsupported`.
- 29 tests covering TC-1 through TC-12, CRC-32 vectors, DOS datetime, error paths.
- Zip-bomb guard: 256 MiB decompressed output limit.
