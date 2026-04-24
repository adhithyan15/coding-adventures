# Changelog — CodingAdventures.Zip.FSharp

All notable changes to this package will be documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
