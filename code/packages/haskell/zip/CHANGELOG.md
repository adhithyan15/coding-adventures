# Changelog — zip (Haskell, CMP09)

## [0.1.0] — 2026-04-24

### Added

- `Zip` module with full ZIP archive read/write support (CMP09).
- `writeZip` — build an archive from `(name, data, compress)` triples.
- `readZip` — parse all entries using the EOCD-first strategy.
- `readEntry` — random-access read of a single file by name.
- `zip'` / `unzip'` — convenience wrappers (primed to avoid Prelude clash).
- RFC 1951 DEFLATE (fixed Huffman, BTYPE=01) compressor and decompressor,
  using `lzss` for LZ77 match-finding (window=32768, max=255, min=3).
- Table-driven CRC-32 (polynomial 0xEDB88320) with incremental update support.
- Auto-fallback: DEFLATE is used only when it strictly reduces size; otherwise
  method=0 (Stored) is chosen — handles already-compressed data transparently.
- Directory entry support (names ending with `/`).
- UTF-8 filename support (GP flag bit 11).
- MS-DOS epoch timestamp (1980-01-01 00:00:00) used for all entries.
- 256 MB decompression cap to guard against decompression-bomb attacks.
- 12-case Hspec test suite covering all major behaviours.
