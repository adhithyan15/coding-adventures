# Changelog

## [0.2.0] - 2026-08-13

### Added

- Public `raw_deflate`, `raw_inflate`, and `raw_inflate_counted` APIs for the
  ZIP-owned, unframed RFC 1951 codec.
- Dynamic Huffman decoding, exact final-byte consumption, the full 32 KiB
  distance window, symbol 285, and strict stored/fixed/dynamic multi-block
  behavior.
- Caller-lowerable output bounds under a hard 256 MiB ceiling, typed
  payload-blind errors with the shared 14-code contract, and no partial output.
- All 34 language-neutral `zip-raw-rfc1951-v1` cases, independent zlib encoder
  interoperability, dynamic ZIP integration, suffix/size rejection, and a
  full-window foreign stream.
- Explicit empty capability metadata for the pure in-memory production package.

### Changed

- Method-8 ZIP reads now require exact compressed-payload consumption and exact
  declared uncompressed size before CRC-32 validation; excess output is no
  longer silently trimmed.
- Package version is now 0.2.0. CRC-32 remains accidental-corruption detection,
  not authentication.

## [0.1.0] - 2026-04-23

### Added

- Initial implementation of the ZIP archive format (CMP09 — PKZIP 1989) in Perl 5.26+.
- `new_writer()`: creates a ZipWriter hashref.
  - `add_file($w, $name, $data, $compress)`: adds a file entry, auto-compresses with DEFLATE if beneficial.
  - `add_directory($w, $name)`: adds a directory entry (name must end with `/`).
  - `finish($w)`: appends Central Directory and EOCD, returns binary string.
- `new_reader($data)`: parses ZIP archives using EOCD-first strategy. Dies on malformed input.
  - `reader_entries($r)`: returns arrayref of entry hashrefs.
  - `reader_read($r, $entry)`: decompresses and CRC-validates one entry. Dies on error.
  - `read_by_name($r, $name)`: convenience wrapper.
- `zip($entries, $compress)`: one-shot compression.
- `unzip($data)`: one-shot decompression → hashref of name → data.
- `crc32($data, $initial)`: table-driven CRC-32 (polynomial 0xEDB88320), chainable via `$initial`.
- `dos_datetime($year, $month, $day, $hour, $minute, $second)`: MS-DOS datetime encoder.
- `dos_epoch()`: returns `0x00210000` for 1980-01-01 00:00:00.
- RFC 1951 DEFLATE inlined (fixed Huffman BTYPE=01), backed by `CodingAdventures::LZSS` for LZ77
  tokenization (32 KB window). Cannot reuse the repo's `CodingAdventures::Deflate` — it uses a
  custom non-RFC-1951 wire format.
- `_bw_*` / `_br_*` private functions for LSB-first bit I/O with Huffman bit-reversal.
- Decompressor uses integer array for O(1) back-reference indexing.
- Security guards: path traversal rejection (`.`, `/prefix`, backslash) on both read AND write
  paths, null-byte rejection, zip-bomb guard (256 MiB output limit), `local_offset < cd_offset`
  validation (prevents CD-confusion attacks), sign-extension guards for 32-bit fields,
  duplicate entry name rejection in `unzip()`, entry count < 65535.
- 30 tests covering TC-1 through TC-12, CRC-32 vectors, DOS datetime, EOCD scanning,
  path traversal security, duplicate entry rejection, stored-block decode path.
