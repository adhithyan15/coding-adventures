# Changelog

## Unreleased

### Changed

- Store counted raw-inflate output in completed 4 KiB byte strings plus one
  bounded numeric tail. This preserves overlapping back-references and exact
  consumption while preventing multi-gigabyte table amplification near the
  256 MiB output ceiling.

## [0.2.0] - 2026-08-14

### Added

- Public `raw_deflate`, `raw_inflate`, and `raw_inflate_counted` APIs for the
  required ZIP-owned raw RFC 1951 profile.
- Stored, fixed-Huffman, dynamic-Huffman, and multi-block decoding, including
  canonical tree validation, symbol 285, overlapping copies, and the full
  32 KiB distance window.
- Stable payload-blind error identifiers, an exact consumed-byte result, and a
  caller-lowerable 256 MiB output ceiling with checks before every write.
- All 34 language-neutral conformance cases plus independent pure-Lua encoder
  interoperability and full-window coverage.

### Changed

- ZIP method 8 now rejects compressed suffix cavities and exact uncompressed
  size mismatches before CRC validation instead of silently trimming output.
- Both build front doors now run syntax, lint, full tests, and focused LuaCov
  coverage while keeping JSON and LibDeflate dependencies test-only.

## [0.1.0] - 2026-04-23

### Added

- Initial implementation of the ZIP archive format (CMP09 — PKZIP 1989) in Lua 5.4.
- `CodingAdventures.Zip.new_writer()`: creates a ZipWriter (plain table).
  - `add_file(writer, name, data, compress)`: adds a file entry, auto-compresses with DEFLATE if beneficial.
  - `add_directory(writer, name)`: adds a directory entry (name must end with `/`).
  - `finish(writer)`: appends Central Directory and EOCD, returns binary string.
- `CodingAdventures.Zip.new_reader(data)`: parses ZIP archives using EOCD-first strategy.
  - `reader_entries(reader)`: returns array of entry tables.
  - `reader_read(reader, entry)`: decompresses and CRC-validates one entry.
  - `read_by_name(reader, name)`: convenience wrapper.
- `zip(entries, compress)`: one-shot compression.
- `unzip(data)`: one-shot decompression → table of name → data.
- `crc32(data, initial)`: table-driven CRC-32 (polynomial 0xEDB88320), chainable via `initial` parameter.
- `dos_datetime(year, month, day, hour, minute, second)`: MS-DOS datetime encoder.
- `DOS_EPOCH`: constant `0x00210000` for 1980-01-01 00:00:00.
- RFC 1951 DEFLATE inlined (fixed Huffman BTYPE=01), backed by `coding_adventures_lzss` for LZ77
  tokenization (32 KB window). Cannot reuse the repo's `coding_adventures_deflate` — it uses a
  custom non-RFC-1951 wire format.
- `BitWriter`/`BitReader` as private module-local functions for LSB-first bit I/O.
- Decompressor uses integer byte array for O(1) back-reference indexing (avoids O(n²) string
  rebuilding on each match-copy byte).
- Security guards: path traversal rejection (`..`, `/` prefix, backslash), null-byte rejection,
  zip-bomb guard (256 MiB output limit), duplicate entry name rejection in `unzip()`.
- 28 tests covering TC-1 through TC-12, CRC-32 vectors, DOS datetime, security edge cases, and
  the DEFLATE stored-block decode path.
