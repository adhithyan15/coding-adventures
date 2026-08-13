# Changelog

## [0.2.0] - 2026-08-12

### Added

- `rawDeflate(data)` and `rawInflate(data)`: the RFC 1951 codec on its own, with
  no ZIP framing. DEFLATE is the compressor inside `zlib`, `gzip`, and PNG's
  `IDAT`; exporting it keeps those formats from each carrying a second copy of
  the same bit-packing code.
- **Dynamic Huffman decoding (BTYPE=10).** Previously rejected outright, which
  meant this reader failed on most archives produced by zlib or Info-ZIP, since
  those emit dynamic blocks for anything but the smallest inputs. Adds a
  canonical Huffman table builder, the code-length alphabet with its 16/17/18
  run-length escapes, and the permuted code-length order.
- **Length symbol 285.** RFC 1951 encodes length 258 either as symbol 284 with
  five extra bits or as symbol 285 with none; the table stopped at 284, so any
  stream using the cheaper form — which a run of identical bytes reliably
  produces — was rejected as an invalid length symbol.

### Changed

- The fixed and dynamic block bodies now share one `decodeHuffmanBlock` routine
  parameterised by its symbol and distance readers, since the two differ only in
  how a symbol comes off the bit stream.
- The encoder is unchanged and its output stays byte-identical: it still emits
  one fixed-Huffman block and still spells length 258 as symbol 284. Only the
  reader grew.

### Tests

- Decoder conformance is now checked against Node's `zlib` as an **oracle** —
  round-tripping our encoder through our decoder only proves the two agree with
  each other. Covers every compression level, incompressible input, a 4 KB run
  that forces symbol 285, and an assertion that the oracle really did emit a
  dynamic block.
- Malformed-stream guards: a code-length repeat that overruns the alphabet, and
  a table where no symbol resolves within RFC 1951's 15-bit cap.
- 43 tests; 98% line coverage.

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
