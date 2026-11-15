# Changelog

## [0.3.0] - 2026-11-15

### Added

- Hardened raw RFC 1951 DEFLATE encoder and bounded streaming decoder to handle IC18 PNG ceiling of 134,234,112 bytes ($33,554,432 \times 4 + 16,384$) in bounded time and RAM.
- 4 KiB paged sliding-window output buffer with fast `:binary.copy/2` pattern replication for run-length decoding and zero-allocation cross-page lookups.
- Centralized protocol constants and bitwise parameters in `CodingAdventures.Zip.Constants` with zero inline magic literals or masks.
- Consolidated closed 14-code RFC 1951 taxonomy and payload-blind diagnostic strings in `CodingAdventures.Zip.Messages` adhering to Standing Rule #20 (no payload or untrusted input reflection in error strings).
- 7-class ZIP and RFC 1951 CVE resilience showcase covering zip bombs, Huffman tree oversubscription, window underflow, stored-block NLEN tampering, trailing payload cavities, Zip-Slip traversal, and payload data leakage.
- Exhaustive test suite in `test/png_scale_hardening_test.exs` including 134 MB roundtrip, 10,000-iteration deterministic PRNG mutation fuzz loop, and bidirectional property tests against Erlang `:zlib`.
- Raised test coverage threshold to >=95% across all `CodingAdventures.Zip*` modules.

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
