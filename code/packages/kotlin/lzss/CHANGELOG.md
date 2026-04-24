# Changelog — kotlin/lzss

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial Kotlin implementation of the LZSS (CMP02) compression algorithm.
- `LzssToken` sealed class with `Literal` and `Match` data classes.
- `Lzss` object with:
  - `encode(data, windowSize, maxMatch, minMatch)` — sliding-window tokeniser.
  - `decode(tokens)` — token-stream decoder with overlap/run-length support.
  - `compress(data)` — one-shot compression to CMP02 wire format.
  - `decompress(data)` — one-shot decompression from CMP02 wire format.
  - `serialiseTokens(tokens, originalLength)` — internal serialiser.
  - `deserialiseTokens(data)` — internal deserialiser with DoS cap on block_count.
- 12 unit tests covering:
  1. Round-trip: empty input
  2. Round-trip: single byte
  3. Round-trip: repetitive text (banana × 50)
  4. Round-trip: all 256 byte values
  5. Round-trip: 1 KB patterned data
  6. Encode: unique data produces only Literals
  7. Encode: repeated data produces Match tokens (ABABAB canonical case)
  8. Compression effectiveness: repetitive data compresses to fewer bytes
  9. Decode: correct output from a known token list (run-length AAAAAAA)
  10. Known wire-format vector: byte-exact check for compressed "AAAAAAA"
  11. Round-trip: Unicode UTF-8 text (Japanese "こんにちは世界")
  12. Round-trip: 10 KB large input (also asserts size reduction)
- Literate programming style: inline explanations, wire-format diagrams,
  and complexity notes throughout the source.
- `required_capabilities.json`: no capabilities needed (pure in-memory).
- `BUILD` and `BUILD_windows` files: `gradle test`.
