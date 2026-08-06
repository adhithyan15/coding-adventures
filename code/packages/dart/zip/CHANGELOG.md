# Changelog

## 0.1.0

- Added the initial Dart implementation of the CMP09 ZIP archive format.
- Added `ZipWriter` (`addFile`, `addDirectory`, `finish`) and `ZipReader`
  (`entries`, `read`, `readByName`), plus `zipBytes`/`unzip` convenience
  wrappers.
- Implemented RFC 1951 DEFLATE directly in this package rather than
  depending on `coding_adventures_deflate`: that package's wire format is a
  private, self-designed serialization for internal round-tripping, not
  the standard raw DEFLATE bit-stream a ZIP entry must carry. This matches
  every other language's `zip` package in the repository (Python, Go,
  Rust, Ruby, TypeScript, Elixir, Lua, Swift, Perl), all of which depend
  only on the sibling `lzss` package. See `lessons.md` for the full
  investigation and `README.md` for the detailed rationale.
- The writer emits fixed-Huffman blocks (BTYPE=01) only, using the
  `coding_adventures_lzss` package for LZ77 match-finding (window=32768,
  max_match=255, min_match=3).
- The reader (`inflate`) decodes all three RFC 1951 block types — stored,
  fixed Huffman, and dynamic Huffman — so it can open archives written by
  real-world producers (`zip`(1), Python's `zipfile`, Java's `jar`,
  Microsoft Office), which almost always use dynamic Huffman blocks.
  Verified against a real Python-`zipfile`-produced fixture (also carried
  by `rust/zip`'s test suite) and against the system `zip`/`unzip` CLI via
  `dart:io Process` in both directions (TC-10).
- Implemented CRC-32 (polynomial `0xEDB88320`) for entry integrity
  verification on read.
- Implemented MS-DOS packed date/time encoding (`dosDatetime`, `dosEpoch`).
- Auto-compression policy: DEFLATE is attempted for every file; the
  compressed form is used only if strictly smaller than the original,
  otherwise the entry falls back to Stored (method 0) — covers
  already-compressed and incompressible inputs automatically (TC-7).
- Security hardening per the CMP09 spec's Security Considerations:
  - EOCD is located by a backward scan bounded to the last `22 + 65535`
    bytes, never an unbounded search.
  - The Central Directory (not the Local Header) is treated as the
    authoritative source for size/method/offset.
  - Entries with unsupported compression methods raise a clear error
    instead of producing garbage.
  - Entries with the encrypted flag (GP flag bit 0) are rejected.
  - Reading is capped at `defaultMaxOutputBytes` (256 MB) as a
    decompression-bomb guard, both on the declared and the actual
    decompressed size.
  - Archive entry counts are capped at 65535 (the natural ZIP64-less
    ceiling) both when writing and when parsing a Central Directory.
  - All Central Directory / Local Header field reads are bounds-checked
    before indexing into the archive buffer.
- Added 28 tests in `test/zip_test.dart` covering CMP09 spec TC-1 through
  TC-12 (round-trip Stored/DEFLATE, multi-file, directory entries, CRC-32
  corruption detection, EOCD/random access, incompressible-data fallback,
  empty file, 100 KB compression ratio, real CLI interop in both
  directions, Unicode filenames, nested paths), plus CRC-32 unit tests,
  DEFLATE round-trip tests, the real-world dynamic-Huffman fixture, and
  edge cases (missing entry, unsupported method, encrypted entry, missing
  EOCD, MS-DOS epoch).
