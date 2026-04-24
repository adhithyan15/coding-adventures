# Changelog — java/lzss

All notable changes to this package are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-24

### Added

- `LzssToken` sealed interface with `Literal(byte)` and `Match(int offset, int length)` record variants.
- `Lzss.encode` — greedy sliding-window encoder; emits tokens with configurable `windowSize`, `maxMatch`, and `minMatch`.
- `Lzss.decode` — token-stream decoder; handles overlapping matches (run-length expansion).
- `Lzss.compress` — one-shot convenience: encode + serialise to CMP02 wire format.
- `Lzss.decompress` — one-shot convenience: deserialise CMP02 + decode.
- Internal `serialiseTokens` / `deserialiseTokens` — CMP02 wire-format read/write using `java.nio.ByteBuffer` (big-endian).
- DoS guard in `deserialiseTokens`: `block_count` is capped against actual payload size.
- 15 JUnit 5 tests covering round-trips, encoder structure, decoder correctness, compression ratio, and security vectors.
