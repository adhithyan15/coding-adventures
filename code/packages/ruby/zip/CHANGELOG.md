# Changelog

## [0.1.0] - 2026-04-23

### Added

- Initial implementation of the ZIP archive format (CMP09 — PKZIP 1989) in Ruby.
- `CodingAdventures::Zip::ZipWriter`: builds ZIP archives incrementally in memory.
  - `add_file(name, data, compress: true)`: adds a file entry, auto-compresses if beneficial.
  - `add_directory(name)`: adds a directory entry.
  - `finish`: appends Central Directory and EOCD, returns binary String.
- `CodingAdventures::Zip::ZipReader`: parses ZIP archives using EOCD-first strategy.
  - `entries`: lists all `ZipEntry` structs.
  - `read(entry)`: decompresses and CRC-validates one entry.
  - `read_by_name(name)`: convenience wrapper.
- `CodingAdventures::Zip.zip(entries, compress: true)`: one-shot compression.
- `CodingAdventures::Zip.unzip(data)`: one-shot decompression → Hash of name → data.
- `CodingAdventures::Zip.crc32(data, initial: 0)`: table-driven CRC-32 (polynomial 0xEDB88320).
- `CodingAdventures::Zip.dos_datetime(year, month, day, ...)`: MS-DOS datetime encoder.
- `CodingAdventures::Zip::DOS_EPOCH`: constant `0x00210000` for 1980-01-01 00:00:00.
- RFC 1951 DEFLATE inlined (fixed Huffman BTYPE=01), backed by `coding_adventures_lzss` for LZ77 tokenization (32 KB window). Cannot reuse the repo's `coding_adventures_deflate` gem — it uses a custom non-RFC-1951 wire format.
- `BitWriter`/`BitReader` classes for LSB-first bit I/O.
- 34 tests covering TC-1 through TC-12, CRC-32 vectors, DOS datetime, error paths.
