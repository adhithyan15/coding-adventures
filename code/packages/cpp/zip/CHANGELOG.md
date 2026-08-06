# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-05

### Added

- Pure ISO C++17, header-only port of the Rust `zip` crate (CMP09), in
  namespace `ca::zip`: the ZIP archive format (PKZIP, 1989) as a DEFLATE
  container — Local File Headers, Central Directory, End of Central
  Directory record, CRC-32, MS-DOS date/time packing.
- `ZipWriter`: `add_file(name, data, compress=true)` (auto-fallback to
  Stored when DEFLATE would not shrink the entry), `add_directory(name)`,
  `finish() -> Bytes`. Never fails.
- `ZipReader`: EOCD-first parsing for reliable random access — scans
  backward (bounded to `22 + 65535` bytes) for the EOCD signature, parses
  the Central Directory (authoritative for size/method, per
  code/specs/CMP09-zip.md's Security Considerations), and only touches a
  Local Header to find its entry's data offset. `entries()`, `read(entry)`,
  `read_by_name(name)`. Throws `ZipException` (carrying a `ZipError`) on
  malformed input, mirroring the sibling `deflate` package's
  `DeflateException` convention.
- `zip::crc32(data, initial=0)` — table-driven CRC-32 (poly `0xEDB88320`),
  inlined per code/specs/CMP09-zip.md ("no separate package — it's a
  trivial table-driven function").
- `zip::dos_datetime(...)` / `DOS_EPOCH` — MS-DOS packed date/time encoding.
- Convenience `zip::zip(entries)` / `zip::unzip(data,
  max_total_uncompressed_bytes=256MB)`.
- Depends directly on the sibling `deflate` package (CMP05, PR #9938) for
  both `compress` and `inflate` — `cpp/deflate` was independently verified
  against a real `zlib`-produced dynamic-Huffman stream this session, so
  (unlike `dart/zip`, which had to self-contain DEFLATE after discovering
  `dart/deflate`'s private wire format — see lessons.md) no reimplementation
  was needed here.
- Robustness: every multi-byte field read from untrusted bytes is
  bounds-checked; `detail::read_u16`/`read_u32` take a `uint64_t` offset
  (not `size_t`) so that CALLERS computing `base + N` for an untrusted
  `uint32_t`-derived `base` (e.g. a Central Directory `local_offset`) are
  forced to do that addition in a width that cannot wrap even on a
  hypothetical 32-bit `size_t` platform — `ZipReader::read` widens
  `entry.local_offset` to `uint64_t` exactly once (`lh_off`) and adds every
  fixed field offset against that value; offset arithmetic combining two
  untrusted `u32` fields (CD offset+size, Local Header data
  start+compressed size) is likewise done in `uint64_t`; a hard cap of
  65535 parsed Central Directory entries on read, and a matching
  `TooManyEntries` rejection on write (the EOCD's entry-count fields are
  16 bits, so a real 65536+-entry archive would otherwise silently declare
  a wrapped, wrong count); a per-entry 256 MB decompression-bomb cap from
  `ca::deflate::inflate` PLUS an aggregate 256 MB (configurable) budget
  across every entry `unzip()` decompresses; `ZipReader::read` throws
  `DeclaredSizeMismatch` if the ACTUAL decompressed size ever disagrees with
  the Central Directory's declared `Uncompressed_Size` (an attacker-controlled
  field) instead of silently trimming to it, which is what makes the
  aggregate budget's declared-size pre-check trustworthy rather than
  bypassable by an entry that understates its real decompressed cost;
  encrypted entries (General-Purpose flag bit 0) rejected with a clear error
  instead of attempting to decompress ciphertext; unsupported compression
  methods rejected explicitly; `ZipWriter` rejects (rather than silently
  truncating) a name over 65535 bytes or data over 4 GiB, AND rejects
  (`ArchiveTooLarge`) writing enough entries to push the cumulative archive's
  Local Header / Central Directory offsets past the same 4 GiB (non-ZIP64)
  limit, via a shared `detail::require_fits_u32` helper rather than an
  unchecked `static_cast<uint32_t>` at each of the three call sites that
  narrow a running `buf_.size()`.
- Zip-slip / path traversal: this package is in-memory only (no
  disk-writing API), so a malicious entry name cannot escape any directory
  through this library — documented in README/module doc so a caller
  building disk extraction on top of `ZipEntry::name` knows the
  sanitization responsibility is theirs.
- Tests (90 checks): all 12 mandatory test cases from
  code/specs/CMP09-zip.md, including a **real** subprocess-based CLI-interop
  test against the system `zip`/`unzip` tools in both directions (this
  repo's other language ports document TC-10 as "manual, subprocess-based"
  without automating it; this port actually shells out via `std::system`,
  skipping gracefully when the tools are not on `PATH`), a real
  dynamic-Huffman ZIP entry fixture (the same bytes used by
  `rust/zip`'s test suite, produced independently by CPython's `zipfile`)
  proving the reader decodes dynamic Huffman it never wrote itself, and
  dedicated regression tests for each security-review finding below
  (`test_declared_size_mismatch_rejected`, `test_writer_name_too_long_rejected`,
  `test_writer_too_many_entries_rejected`, `test_extreme_local_offset_rejected`).
- Verified clean under GCC and Clang with `-std=c++17 -pedantic-errors
  -Wall -Wextra -Werror` (MSVC is exercised in CI via the shared
  `iso-harness`).

### Spec sync

- Fixed a pre-existing bug in `code/specs/CMP09-zip.md`'s Local File Header
  and Central Directory Header byte-offset tables: `Last_Mod_File_Time` and
  `Last_Mod_File_Date` were each mis-sized as 4 bytes instead of 2,
  cascading a 4-byte offset error through every field after them (e.g. the
  spec said the Local File Header's fixed portion was 34 bytes and CRC-32
  lived at offset 18; the correct, standard PKZIP/APPNOTE.TXT layout — which
  every implementation in this repo, including the Rust reference, has
  always actually written and read — is a 30-byte fixed Local Header with
  CRC-32 at offset 14, and a 46-byte fixed Central Directory Header). Also
  added the missing C++ row to the Package Naming table.
