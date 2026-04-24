# Changelog — java/zip

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the ZIP archive format (CMP09) in Java.
- `Zip.ZipWriter`: sequential in-memory archive builder.
  - `addFile(name, data, compress)` — auto-selects DEFLATE or Stored.
  - `addDirectory(name)` — directory entry with Unix mode 0o040755.
  - `finish()` — emits Local Headers, Central Directory, and EOCD.
- `Zip.ZipReader`: EOCD-first random-access archive reader.
  - `entries()` — parsed entry list (names only, lazy data).
  - `read(name)` — decompress + CRC-32 verify on demand.
- RFC 1951 DEFLATE compressor: single fixed-Huffman block via LZSS.
- RFC 1951 DEFLATE decompressor: stored and fixed-Huffman blocks.
- `Zip.zip(List<ZipEntry>)` and `Zip.unzip(byte[])` convenience API.
- CRC-32 with table-driven reflected polynomial 0xEDB88320.
- UTF-8 filename support (GP flag bit 11).
- Fall-back to Stored when DEFLATE expands the data (incompressible inputs).
- 256 MB decompression bomb guard in the DEFLATE decompressor.
- 12 JUnit 5 tests (TC-01 through TC-12) mirroring the C# reference suite.
- Depends on `com.codingadventures:lzss` via Gradle composite build.
