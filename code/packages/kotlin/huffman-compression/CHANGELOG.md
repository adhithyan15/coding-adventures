# Changelog — huffman-compression (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.1] — 2026-11-10

### Security
- SEC-003: `decompress` now treats the code-lengths table as hostile input and
  throws `IllegalArgumentException` for `symbol_count > 256`, duplicate
  symbols, code lengths outside 1..56, tables not sorted by length,
  over-subscribed (Kraft > 1) tables, and a declared `original_length` larger
  than 8 x payload bytes (checked before allocating). The header is read as
  unsigned, bits are read in O(1) from the payload, and decoding fails once the
  accumulated bits reach the longest code length. Valid output still round-trips.

## [0.1.0] — 2026-04-25

### Added
- `HuffmanCompression` object with `compress(ByteArray?)` and
  `decompress(ByteArray?)` implementing the CMP04 wire format.
- Idiomatic Kotlin port of the Java CMP04 implementation:
  - `buildString { }` for bit concatenation.
  - `sortedWith(compareBy { })` for code-lengths sorting.
  - `padStart(length, '0')` for canonical code left-padding.
  - `HashMap<String, Int>` for O(1) code-to-symbol lookup during decode.
  - `accumulated.clear()` instead of `setLength(0)`.
  - Both `compress` and `decompress` accept `null` (treated as empty).
- `packBitsLsbFirst(String): ByteArray` private helper with LSB-first bit
  packing (same convention as LZW/GIF).
- `build.gradle.kts` with composite build dependency on `kotlin/huffman-tree`.
- `settings.gradle.kts` with `includeBuild("../huffman-tree")`.
- 45 unit tests (mirrors the Java test suite) covering:
  - Round-trip fidelity (13 tests)
  - Exact wire-format bytes including the "AAABBC" worked example (7 tests)
  - Edge cases: empty, null, single byte, high bytes, null bytes, short
    header, truncated stream (11 tests)
  - Compression effectiveness: skewed, repeated, uniform distributions (3 tests)
  - Determinism (3 tests)
  - Error handling: bit-stream exhaustion throws `IllegalArgumentException` (1 test)
- `BUILD` / `BUILD_windows`, `README.md`, `.gitignore`.
