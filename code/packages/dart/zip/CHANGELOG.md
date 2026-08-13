# Changelog

## 0.2.0 - 2026-08-13

- Added the CMP09 portable `rawDeflate`, `rawInflate`, and
  `rawInflateCounted` API while retaining `deflateCompress` and `inflate` as
  compatibility entry points.
- Added stable payload-blind `RawInflateError.code` failures, exact compressed
  byte consumption, strict dynamic-header, Huffman, repeat, reserved-symbol,
  and output-limit validation, including RFC-conforming 32-slot distance
  headers.
- Added a consumer for all 34 language-neutral `zip-raw-rfc1951-v1` cases and
  an independent Dart SDK raw-zlib interoperability oracle.
- Hardened method-8 ZIP reads to reject unused bytes inside a declared
  compressed payload and to cap inflate at the smaller of the entry's declared
  size and the caller's limit.
- Declared an explicit empty host-capability profile and added format and
  analyzer gates to the package BUILD front door.

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
  - Entries with the encrypted flag (GP flag bit 0) are rejected, checked
    from the Central Directory's copy of the flags field (authoritative)
    as well as the Local Header's — a crafted archive whose two copies of
    the flags field disagree cannot slip an encrypted entry past the check
    by clearing only the Local Header's bit.
  - Reading is capped at `defaultMaxOutputBytes` (256 MB) as a
    decompression-bomb guard, both on the declared and the actual
    decompressed size. The decoder accumulates output in a byte-native
    growable buffer (`_ByteBuffer`, backed by `Uint8List`) rather than a
    boxed `List<int>` — on the Dart VM a `List<int>` slot is a full 8-byte
    word, so a `List<int>`-based accumulator bounded at "256 MB" of
    *elements* would actually permit roughly 2 GB of real memory, silently
    defeating the documented bound by close to an order of magnitude.
  - Archive entry counts are capped at 65535 (the natural ZIP64-less
    ceiling) both when writing and when parsing a Central Directory.
  - The number of Central Directory headers actually parsed is
    cross-checked against the EOCD's own declared entry count. Each
    entry's advance to the next CD header trusts that entry's own
    (attacker-controlled) name/extra/comment-length fields; without this
    check, a crafted archive that inflates one of those fields desyncs the
    parser from the real next header, which then fails its signature check
    and silently stops — returning a truncated-but-plausible entry list
    with no error, rather than the corruption it actually is.
  - All Central Directory / Local Header field reads are bounds-checked
    before indexing into the archive buffer.
- Added 30 tests in `test/zip_test.dart` covering CMP09 spec TC-1 through
  TC-12 (round-trip Stored/DEFLATE, multi-file, directory entries, CRC-32
  corruption detection, EOCD/random access, incompressible-data fallback,
  empty file, 100 KB compression ratio, real CLI interop in both
  directions, Unicode filenames, nested paths), plus CRC-32 unit tests,
  DEFLATE round-trip tests, the real-world dynamic-Huffman fixture, and
  edge cases (missing entry, unsupported method, encrypted entry via both
  Local Header and Central Directory flags, Central Directory entry-count
  mismatch, missing EOCD, MS-DOS epoch).
