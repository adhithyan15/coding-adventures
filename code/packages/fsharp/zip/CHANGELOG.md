# Changelog — CodingAdventures.Zip.FSharp

All notable changes to this package will be documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.1] — 2026-08-03

### Rescued

This package was written on a branch (`worktree-feat+zstd-and-catchups`) that
was never opened as a PR and went stale for 3+ months. It has been pulled
into a fresh branch off current `main` and re-verified end to end:

- `dotnet test` (net9.0, current toolchain) passes unchanged against the
  `CodingAdventures.Lzss.FSharp` dependency as it exists on `main` today — no
  API drift was found between the stale branch and the current `Lzss`
  package (`Lzss.Encode(data, windowSize, maxMatch, minMatch)` signature is
  unchanged).
- Added an `F#` row to `code/specs/CMP09-zip.md`'s Package Naming table,
  which had not been updated when this package was originally written.

### Fixed

Hardened `ZipReader` against attacker-controlled Central Directory fields.
The Central Directory's `Compressed_Size`, `Uncompressed_Size`,
`Relative_Offset_Of_Local_Header`, `CD_Offset`, and `CD_Size` fields are
32-bit values read verbatim from archive bytes. Narrowing an oversized
`uint32` (e.g. `0x80000000u`) straight to `int` wraps it negative — which
can defeat a bounds check written in `int` arithmetic, since a negative
value always compares less than a small positive `data.Length`. All of
this offset/size arithmetic in `ZipReader` now happens in `int64` and is
range-checked before narrowing to `int` for indexing, so malformed input
consistently raises `InvalidDataException` (documented behavior) instead of
an unhandled `ArgumentOutOfRangeException` from `ReadOnlySpan`, or (in one
case) driving the post-decompress trim step with a wrapped-negative slice
bound. Also rejects entry names that encode to more than 65535 UTF-8 bytes
in `ZipWriter.AddFile`/`AddDirectory` — File_Name_Length is a 16-bit field,
and writing one without a check would silently truncate it, corrupting the
archive rather than failing closed.

Added four "Hardening" xUnit tests covering: a corrupted `Compressed_Size`,
a corrupted `Relative_Offset_Of_Local_Header`, a corrupted
`Uncompressed_Size` that must NOT spuriously trigger the trim step, and an
oversized entry name — bringing the suite to 16 tests (85.1% line coverage,
up from 84.1%).

## [0.1.0] — 2026-04-24

### Added

- `ZipEntry` record type (`Name: string`, `Data: byte[]`) representing a single
  file or directory entry in a ZIP archive.
- `ZipWriter` class with `AddFile`, `AddDirectory`, and `Finish` members for
  building ZIP archives incrementally in memory.
  - Auto-selects DEFLATE (method 8) when compressed output is strictly smaller
    than the original; falls back to Stored (method 0) otherwise.
  - UTF-8 filenames encoded with General Purpose Bit 11 set (RFC compliance).
  - Fixed DOS timestamp 1980-01-01 00:00:00 for reproducible archives.
  - Unix external attributes written for file (`0o100644`) and directory
    (`0o040755`) entries.
- `ZipReader` class with `Entries` property and `Read(name)` method for
  random-access extraction without reading all entries.
  - EOCD-first parsing strategy; Central Directory is the authoritative source
    for sizes and method.
  - CRC-32 verification after decompression.
  - Rejects encrypted entries (GP flag bit 0) with a clear error message.
- `ZipArchive` module with `zip` and `unzip` convenience functions for one-shot
  archive creation and extraction.
- Pure F# RFC 1951 DEFLATE compressor (fixed Huffman, BTYPE=01) backed by the
  `CodingAdventures.Lzss.FSharp` tokeniser (window=32768, maxMatch=255, minMatch=3).
- Pure F# RFC 1951 DEFLATE decompressor supporting stored blocks (BTYPE=00) and
  fixed Huffman blocks (BTYPE=01).
  - 256 MB decompression bomb guard.
  - LEN/NLEN one's-complement validation on stored blocks.
- Table-driven CRC-32 (polynomial 0xEDB88320, RFC 1952 §8).
- 12 xUnit test cases: round-trip Stored, round-trip DEFLATE, multiple files,
  directory entries, CRC mismatch detection, random-access read, incompressible
  fallback, empty file, 100 KB large file, Unicode filenames, nested paths,
  empty archive.
