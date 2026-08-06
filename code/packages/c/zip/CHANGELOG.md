# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-05

### Added

- Pure ISO C17 port of the ZIP archive format (CMP09): `ZipWriter` /
  `ZipReader` opaque handles plus the one-shot `zip_bytes` / `zip_unzip`
  convenience API, matching the shape of the reference Rust `zip` crate.
- Local File Header, Central Directory Header, and End of Central Directory
  Record (EOCD) — full read/write, little-endian, per APPNOTE.TXT: Stored
  (method 0) and DEFLATE (method 8), UTF-8 filenames (GP flag bit 11 always
  set), MS-DOS packed date/time (`zip_dos_datetime`), Unix external file
  attributes.
- CRC-32 (polynomial 0xEDB88320, table-driven, standard pre/post-XOR
  `0xFFFFFFFF`), exposed as `zip_crc32` and verified against every decoded
  entry.
- **Depends directly on `c/deflate` (CMP05, RFC 1951)** for DEFLATE
  compress/decompress, rather than reimplementing DEFLATE framing inline —
  a deliberate divergence from the repo-wide precedent documented in
  `code/specs/CMP09-zip.md` (most language `zip` ports cannot do this because
  their sibling `deflate` package uses a private, non-standard wire format;
  `c/deflate` was specifically built and verified as genuine RFC 1951,
  including real `zlib`-produced dynamic-Huffman streams, so `c/zip` reuses
  it directly). `zip_writer_add_file` auto-selects DEFLATE only when it is
  strictly smaller than the original, else falls back to Stored.
  `zip_reader_read` decodes all three RFC 1951 block types via
  `deflate_decompress` (stored/fixed/dynamic Huffman), proven against a real
  Python-`zipfile`-produced dynamic-Huffman fixture.
- Security hardening for untrusted input, per `code/specs/CMP09-zip.md`
  "Security Considerations":
  - EOCD search bounded to the last `22 + 65535` bytes, never unbounded.
  - Every multi-byte field read is bounds-checked before use.
  - Central Directory offset/size arithmetic performed in a 64-bit
    intermediate before narrowing to `size_t`, so an adversarial 32-bit
    offset/size pair cannot wrap a 32-bit `size_t` and slip past a bounds
    check.
  - Parsed Central Directory entries hard-capped at `ZIP_MAX_ENTRIES`
    (65535).
  - `ZipReader` tracks an AGGREGATE decompressed-bytes budget across every
    `zip_reader_read` call made through it (default 256 MiB via
    `ZIP_DEFAULT_MAX_TOTAL_UNCOMPRESSED`, configurable via
    `zip_reader_new_with_budget`) — not just c/deflate's existing per-entry
    256 MiB cap — so many small entries that sum to a bomb are rejected too.
  - Encrypted entries (GP flag bit 0) rejected with `ZIP_ERR_ENCRYPTED`;
    unsupported methods rejected with `ZIP_ERR_UNSUPPORTED_METHOD`.
  - This package is purely in-memory and never writes to the filesystem, so
    zip-slip/path-traversal cannot occur inside it; `zip.h` documents that
    any disk-writing wrapper built on top must sanitise entry names itself.
- Tests (`tests/zip_test.c`, 158 checks): every TC-1..TC-12 from
  `code/specs/CMP09-zip.md` (Stored/DEFLATE round-trip, multi-file,
  directory entries, CRC-32 corruption detection, EOCD/random-access
  reading, incompressible-data-to-Stored fallback, empty file, 100KB
  large-file compression ratio, CLI interop with the system `zip`/`unzip`
  via `system()`, Unicode filenames, nested paths), plus the dynamic-Huffman
  fixture, CRC-32/`zip_dos_datetime` unit checks, and targeted robustness
  tests for the security hardening above (CD offset/size overflow, malformed
  EOCD, unsupported method, encrypted entry, single- and multi-entry
  aggregate decompression-bomb budgets). Verified clean under
  ASan+UBSan in addition to the harness's GCC/Clang/MSVC matrix.
