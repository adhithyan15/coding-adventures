# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.1] — 2026-07-02

### Changed

- `ZipReader::read` now delegates method-8 (DEFLATE) decompression to
  `deflate::inflate`, which decodes all three RFC 1951 block types — stored
  (BTYPE=00), fixed Huffman (BTYPE=01), and **dynamic Huffman (BTYPE=10)**. The
  previous inline `deflate_decompress` only handled stored and fixed-Huffman
  blocks and explicitly rejected dynamic-Huffman entries. Because virtually
  every real-world producer emits dynamic-Huffman blocks — Microsoft Office
  writing `.xlsx`/`.docx`/`.pptx`, `zip`(1), Python `zipfile`, Java `jar` — the
  reader could not open archives people actually have. This is the foundational
  fix for reading OOXML packages.

### Removed

- The inline `deflate_decompress` (fixed-Huffman only) and its exclusive helpers
  (`BitReader`, `fixed_ll_decode`), now redundant with the `deflate` crate. The
  writer's round-trip tests were rerouted to `deflate::inflate`, which also
  proves our fixed-Huffman output is standard RFC 1951 any decoder can read.

### Added

- `deflate` path dependency (for the RFC 1951 `inflate` decoder).
- `test_read_dynamic_huffman_entry` — reads a real ZIP produced by Python's
  `zipfile` whose single entry uses a dynamic-Huffman block, exercising the full
  path through `unzip`/`ZipReader::read` including the CRC-32 check.

## [0.1.0] — 2026-04-23

### Added

- Initial implementation of the ZIP archive format (CMP09).
- `ZipWriter` — builds a multi-file `.zip` archive in memory:
  - `add_file(name, data, compress)` — add a file with optional DEFLATE compression
  - `add_directory(name)` — add a directory entry
  - `finish()` → `Vec<u8>` — emit the complete archive bytes (Local File Headers +
    Central Directory + End of Central Directory record)
  - Auto-fallback: if DEFLATE output is not smaller than original, entry is stored
    uncompressed (method 0) regardless of the `compress` flag
- `ZipReader<'a>` — read an existing `.zip` archive:
  - `new(data)` — parse the End of Central Directory (EOCD-first scan) and Central
    Directory; returns `Err` on malformed archives
  - `entries()` — slice of `ZipEntry` metadata structs
  - `read(entry)` — decompress and verify CRC-32 for a single entry
  - `read_by_name(name)` — convenience wrapper
- `zip(entries)` / `unzip(data)` — high-level one-shot helpers
- `crc32(data, initial)` — CRC-32/ISO-HDLC (polynomial 0xEDB88320), precomputed
  256-entry table, supports incremental/chained calls
- `dos_datetime(y, m, d, h, min, s)` — pack a timestamp into MS-DOS date/time format
- `DOS_EPOCH` constant — MS-DOS epoch (1980-01-01 00:00:00) as packed `u32`
- RFC 1951 DEFLATE (inline, no zlib wrapper):
  - `deflate_compress(data)` — fixed-Huffman encoder backed by `lzss` for
    LZ77 match-finding
  - `deflate_decompress(data)` — decoder supporting stored blocks (BTYPE=00)
    and fixed-Huffman blocks (BTYPE=01)
  - `BitWriter` / `BitReader` — LSB-first packed bit-stream helpers
- 23 unit tests + 5 doctests; all pass
- Security: output capped at 256 MB in `unzip()`; CRC-32 validated on every `read()`
