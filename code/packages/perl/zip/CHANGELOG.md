# Changelog

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
