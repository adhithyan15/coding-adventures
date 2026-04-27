# Changelog

## [0.1.0] - 2026-04-23

### Added

- Initial implementation of the ZIP archive format (CMP09 — PKZIP 1989).
- `ZipWriter`: builds ZIP archives incrementally in memory.
  - `addFile(name, data, compress?)`: adds a file entry compressed with DEFLATE (method 8) or stored verbatim (method 0) based on which is smaller.
  - `addDirectory(name)`: adds a directory entry.
  - `finish()`: appends the Central Directory and EOCD record and returns the complete archive.
- `ZipReader`: parses ZIP archives using the EOCD-first strategy.
  - `entries()`: lists all `ZipEntry` metadata objects.
  - `read(entry)`: decompresses and CRC-validates a single entry.
  - `readByName(name)`: convenience wrapper.
- `zipBytes(entries, compress?)`: one-shot compression for `[name, data]` pairs.
- `unzip(data)`: one-shot decompression, returns `Map<string, Uint8Array>`.
- `crc32(data, initial?)`: table-driven CRC-32 (polynomial 0xEDB88320). Supports incremental updates.
- `dosDatetime(year, month, day, ...)`: encodes MS-DOS datetime. `DOS_EPOCH` constant for 1980-01-01.
- RFC 1951 DEFLATE inlined (fixed Huffman BTYPE=01), backed by `@coding-adventures/lzss` for LZ77 tokenization (32 KB window).
- `BitWriter` / `BitReader` using `bigint` accumulator for overflow-safe bit manipulation.
- 32 test cases covering TC-1 through TC-12 from the CMP09 spec, plus CRC-32 vectors, DOS datetime encoding, error paths (corrupt CRC, no EOCD, unsupported method, missing entry), and direct `ZipWriter` API tests.
