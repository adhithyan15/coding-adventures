# Changelog

## [0.2.0] - 2026-08-13

### Added

- Public `raw_deflate/1`, `raw_inflate/2`, and `raw_inflate_counted/2` APIs for
  ZIP-owned raw RFC 1951 streams.
- Strict dynamic-Huffman decoding, exact BFINAL byte consumption, caller output
  caps, full 32 KiB overlapping back-references, and 14 stable payload-blind
  error identifiers.
- All 34 `zip-raw-rfc1951-v1` neutral conformance cases plus independent Erlang
  `:zlib` encoder interoperability and full-window coverage.
- Explicit empty production capability metadata.

### Changed

- ZIP method 8 extraction now rejects compressed-payload suffix cavities and
  declared uncompressed-size mismatches before CRC validation.
- Test inputs are deterministic and the BUILD front door enforces formatting,
  warning-free test compilation, coverage, and the complete suite.

### Security

- Output limits are validated before decoding and enforced before every stored
  byte, literal, and back-reference write; failures expose no partial output or
  attacker-controlled details. CRC-32 remains non-authentication.

## [0.1.0] - 2026-04-23

### Added

- Initial implementation of the ZIP archive format (CMP09 — PKZIP 1989) in Elixir.
- `CodingAdventures.Zip.new_writer/0`: creates a ZipWriter (plain map).
  - `add_file/4`: adds a file entry, auto-compresses if beneficial.
  - `add_directory/2`: adds a directory entry.
  - `finish/1`: appends Central Directory and EOCD, returns binary.
- `CodingAdventures.Zip.new_reader/1`: parses ZIP archives using EOCD-first strategy.
  - `reader_entries/1`: lists all ZipEntry maps.
  - `reader_read/2`: decompresses and CRC-validates one entry.
  - `read_by_name/2`: convenience wrapper.
- `CodingAdventures.Zip.zip/2`: one-shot compression.
- `CodingAdventures.Zip.unzip/1`: one-shot decompression → map of name → data.
- `CodingAdventures.Zip.crc32/2`: table-driven CRC-32 (polynomial 0xEDB88320), chainable via `initial` parameter.
- `CodingAdventures.Zip.dos_datetime/6`: MS-DOS datetime encoder.
- `CodingAdventures.Zip.dos_epoch/0`: constant `0x00210000` for 1980-01-01 00:00:00.
- RFC 1951 DEFLATE inlined (fixed Huffman BTYPE=01), backed by `coding_adventures_lzss` for LZ77 tokenization (32 KB window). Cannot reuse the repo's `coding_adventures_deflate` gem — it uses a custom non-RFC-1951 wire format.
- `BitWriter`/`BitReader` as private functions for LSB-first bit I/O.
- 26 tests covering TC-1 through TC-12, CRC-32 vectors, DOS datetime, BTYPE=00 decode path, EOCD scan, error paths.
