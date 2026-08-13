# Changelog

## [0.3.0] - 2026-08-13

### Added

- Consumption of the 34-case language-neutral ZIP raw RFC 1951 v1 corpus for
  stored, fixed, dynamic, multi-block, foreign-decoder, counted-boundary,
  malformed-stream, output-cap, and incremental CRC-32 behavior.
- `RawInflateError` with a closed, payload-blind error-code taxonomy, plus the
  exported `RAW_INFLATE_MAX_OUTPUT` hard ceiling.
- Explicit empty capability metadata for the pure in-memory package.
- Correct package and PNG-downstream BUILD front doors: dependency installs now
  run in subshells so each gate returns to and tests its intended package.

### Changed

- `rawInflate` and `ZipReader` now require a non-negative safe-integer ceiling
  no larger than 256 MiB and validate it before output allocation.
- Dynamic blocks accept all 32 RFC 1951 distance code-length slots when the two
  reserved slots are unused; actually decoding distance symbol 30 or 31 still
  fails closed through both fixed and dynamic tables.
- `rawInflateCounted` and every malformed-stream path now return stable error
  codes without echoing attacker-controlled bytes, counts, offsets, or lengths.
- `ZipReader` rejects suffix bytes hidden inside a DEFLATE entry's declared
  compressed payload by enforcing exact BFINAL byte consumption.

### Security

- The hard ceiling cannot be disabled with infinity, unsafe integers,
  fractions, or a larger number. Cap checks happen before every output growth
  or back-reference copy, and failures return no partial output.
- Exact consumed-byte reporting and stable payload-free errors are enforced by
  the shared neutral corpus and an independent Python zlib/binascii validator.

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

- `rawInflateCounted(data, maxOutput?)` returns the output **and** how many
  input bytes the stream actually used. DEFLATE announces its own end with
  BFINAL, so a stream can finish well before the region its container allotted
  it, and a plain inflate never reports the gap. A format that knows where its
  compressed data should stop — PNG's `IDAT` before its Adler-32, gzip before
  its CRC — needs that number to refuse the difference, which is otherwise free
  carriage inside a file every tool calls valid.

### Changed

- The fixed and dynamic block bodies now share one `decodeHuffmanBlock` routine
  parameterised by its symbol and distance readers, since the two differ only in
  how a symbol comes off the bit stream.
- The encoder is unchanged and its output stays byte-identical: it still emits
  one fixed-Huffman block and still spells length 258 as symbol 284. Only the
  reader grew.

### Security

- **The output cap now counts bytes.** Inflate accumulated into a plain
  `number[]`, where V8 spends four to eight bytes per element, so the 256 MB
  ceiling really allowed one to two gigabytes of backing store plus a transient
  copy on each growth — enough to kill the process before the limit it was
  supposedly enforcing was reached. Output now accumulates in a growable
  `Uint8Array`. Dynamic blocks made this urgent rather than theoretical: they
  reach DEFLATE's 1032:1 ceiling, so a few hundred kilobytes of hostile input
  can demand hundreds of megabytes.
- **`rawInflate(data, maxOutput?)` takes a ceiling**, because a library reading
  bytes it did not write cannot know its embedder's budget. `ZipReader.read`
  now passes the SMALLER of the entry's declared uncompressed size and the
  reader's own ceiling. The declared size alone would be worse than the fixed
  limit it replaced: it is four bytes the archive chose, can say 4 GiB, and the
  CRC-32 that catches the lie only runs once the memory is already committed.
- **`new ZipReader(bytes, { maxOutput })`** makes that ceiling configurable, for
  the same reason `rawInflate`'s is. It is validated in the constructor rather
  than left to the inflater, because `Infinity` — the natural way to write "no
  limit" — is the one value that survives the `Math.min` and would quietly hand
  the ceiling back to the archive. NaN and negatives are rejected there too, so
  both entry points treat the same value the same way.
- **Huffman tables are checked against Kraft's inequality.** Over-subscribed
  tables (more codes claimed than exist at a length) are rejected outright, and
  incomplete tables are rejected everywhere RFC 1951 forbids them. Without this
  the decoder accepted streams zlib refuses, which is the shape of a
  content-inspection bypass: one tool rejects the file, another extracts real
  content from it.
- The one exception RFC 1951 §3.2.7 allows is keyed on the code's **length**,
  not on the symbol count: a lone distance code "is encoded using one bit, not
  zero bits". A single TWO-bit distance code still leaves a hole and is rejected,
  matching zlib's `max != 1` test. The literal/length alphabet is held to the
  stricter rule deliberately — accepting less than the reference implementation
  is the safe direction to differ in.
- `HLIT`/`HDIST` are range-checked at the header against RFC 1951's 286 and 30,
  rather than failing deep inside the block when an unassignable symbol turns up.

### Tests

- Decoder conformance is now checked against Node's `zlib` as an **oracle** —
  round-tripping our encoder through our decoder only proves the two agree with
  each other. Covers every compression level, incompressible input, a 4 KB run
  that forces symbol 285, and an assertion that the oracle really did emit a
  dynamic block.
- Malformed-stream guards: a code-length repeat that overruns the alphabet,
  incomplete and over-subscribed code-length tables, an incomplete
  literal/length table, and out-of-range `HLIT`/`HDIST`.
- Cap behaviour: a caller-supplied ceiling, a stored block hitting it, a
  nonsensical ceiling rejected rather than ignored, and a 500:1 zlib-built bomb
  stopped at the stated byte count.
- 62 tests; 98% line coverage. The clamp regression is verified adversarially:
  with the `Math.min` removed it fails, which is the only way to know a guard
  test is testing the guard.

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
